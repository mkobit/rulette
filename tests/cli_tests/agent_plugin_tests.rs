use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

const CODEX_FIXTURE: &str = "tests/fixtures/v0_1/codex";
const PLUGIN_MANIFEST_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
const MCP_CONFIG_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";

fn create_sample_plugin_tree(root: &Path) {
    let manifest = serde_json::json!({
        "$schema": PLUGIN_MANIFEST_SCHEMA,
        "name": "sample-e2e-plugin",
        "version": "1.0.0",
        "description": "Integration test plugin for end-to-end and round-trip verification.",
        "author": {
            "name": "Integration Tester",
            "email": "tester@example.com",
            "url": "https://example.com/tester"
        },
        "homepage": "https://example.com/plugin",
        "repository": "https://github.com/example/sample-e2e-plugin",
        "license": "Apache-2.0",
        "keywords": ["agent", "plugin", "e2e"],
        "extensions": {
            "com.example.client": {
                "active": true,
                "timeout": 30
            }
        }
    });
    fs::write(
        root.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mcp = serde_json::json!({
        "$schema": MCP_CONFIG_SCHEMA,
        "mcpServers": {
            "fetch-server": {
                "type": "stdio",
                "command": "./bin/fetch",
                "args": ["--cache", "/tmp"],
                "env": { "DEBUG": "1" },
                "cwd": "${PLUGIN_ROOT}/work"
            },
            "remote-query": {
                "type": "streamable-http",
                "url": "https://api.example.com/query",
                "headers": { "Authorization": "Bearer secret-token" }
            }
        }
    });
    fs::write(
        root.join("mcp.json"),
        serde_json::to_string_pretty(&mcp).unwrap(),
    )
    .unwrap();

    let skill_a_dir = root.join("skills/git-flow");
    fs::create_dir_all(skill_a_dir.join("scripts")).unwrap();
    fs::write(
        skill_a_dir.join("SKILL.md"),
        "---\nname: git-flow\ndescription: Branching and pull request workflow instructions.\n---\n# Git Flow\nAlways open a pull request.\n",
    )
    .unwrap();
    fs::write(
        skill_a_dir.join("scripts/validate.sh"),
        "#!/bin/sh\necho validating branch\n",
    )
    .unwrap();

    let skill_b_dir = root.join("skills/code-review");
    fs::create_dir_all(&skill_b_dir).unwrap();
    fs::write(
        skill_b_dir.join("SKILL.md"),
        "---\nname: code-review\ndescription: Automated code review instructions.\n---\n# Code Review\nCheck style and safety.\n",
    )
    .unwrap();

    let ext_dir = root.join("com.example.client");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(
        ext_dir.join("settings.json"),
        r#"{"autoTrigger": true, "profile": "dev"}"#,
    )
    .unwrap();
}

fn create_tar_from_dir(dir: &Path) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    for entry in walkdir::WalkDir::new(dir).min_depth(1).sort_by_file_name() {
        let entry = entry.unwrap();
        let relative = entry.path().strip_prefix(dir).unwrap();
        let path_str = relative.to_str().unwrap().replace('\\', "/");
        if entry.file_type().is_file() {
            let contents = fs::read(entry.path()).unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(contents.len() as u64);
            header.set_cksum();
            builder
                .append_data(&mut header, &path_str, contents.as_slice())
                .unwrap();
        }
    }
    builder.finish().unwrap();
    drop(builder);
    archive
}

fn stage_and_apply(input: &Path, project_root: &Path, allow_lossy: bool) -> String {
    let temporary = tempfile::tempdir().unwrap();
    let stage_dir = temporary.path().join("stage");

    let mut stage_cmd = Command::cargo_bin("rulette").unwrap();
    stage_cmd
        .arg("transform")
        .arg(input)
        .arg("--target")
        .arg("agent-plugin@project")
        .arg("--project-root")
        .arg(project_root)
        .arg("--stage")
        .arg(&stage_dir);
    if allow_lossy {
        stage_cmd.arg("--allow-lossy");
    }
    let stage_output = stage_cmd.assert().success().get_output().stderr.clone();
    let stage_output_str = String::from_utf8(stage_output).unwrap();
    let digest = stage_output_str
        .lines()
        .find_map(|line| line.strip_prefix("plan digest: "))
        .expect("plan digest output")
        .trim()
        .to_owned();

    let mut apply_cmd = Command::cargo_bin("rulette").unwrap();
    apply_cmd
        .arg("transform")
        .arg("--apply")
        .arg(stage_dir.join("rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(digest)
        .arg("--allow-project-root")
        .arg(project_root)
        .assert()
        .success();

    stage_output_str
}

#[test]
fn strict_mode_rejects_rule_inputs_for_agent_plugin_target() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    fs::create_dir(&project_root).unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("agent-plugin@project")
        .arg("--project-root")
        .arg(&project_root)
        .arg("--stage")
        .arg(temporary.path().join("stage"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("unaccepted capability loss"));

    assert!(
        !temporary.path().join("stage").exists(),
        "staging must not write files on strict rejection"
    );
}

#[test]
fn allow_lossy_lowers_rules_to_skills_and_synthesizes_manifest() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    fs::create_dir(&project_root).unwrap();

    let stage_output = stage_and_apply(Path::new(CODEX_FIXTURE), &project_root, true);

    assert!(stage_output.contains("rule-lowered-as-skill"));
    assert!(stage_output.contains("synthesized-manifest"));

    let manifest_path = project_root.join("plugin.json");
    assert!(manifest_path.exists(), "plugin.json must be synthesized");
    let manifest_content: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest_content["$schema"], PLUGIN_MANIFEST_SCHEMA);
    assert!(!manifest_content["name"].as_str().unwrap().is_empty());

    let skills_dir = project_root.join("skills");
    assert!(skills_dir.exists(), "skills directory must be created");
    let skill_entries: Vec<PathBuf> = fs::read_dir(&skills_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert!(
        !skill_entries.is_empty(),
        "at least one skill must be emitted"
    );

    let mut found_lowered_rule = false;
    for skill_path in skill_entries {
        let skill_md = skill_path.join("SKILL.md");
        if skill_md.exists() {
            let content = fs::read_to_string(&skill_md).unwrap();
            assert!(
                content.starts_with("---\n"),
                "SKILL.md must have YAML frontmatter"
            );
            assert!(
                content.contains("name:"),
                "SKILL.md frontmatter must contain name"
            );
            assert!(
                content.contains("description:"),
                "SKILL.md frontmatter must contain description"
            );
            found_lowered_rule = true;
        }
    }
    assert!(found_lowered_rule, "lowered skill instruction was verified");
}

#[test]
fn round_trip_agent_plugin_package_losslessly_in_strict_mode() {
    let source_dir = tempfile::tempdir().unwrap();
    create_sample_plugin_tree(source_dir.path());

    // 1. Transform source to first output target in strict mode (no --allow-lossy)
    let out_dir_1 = tempfile::tempdir().unwrap();
    fs::create_dir_all(out_dir_1.path()).unwrap();
    stage_and_apply(source_dir.path(), out_dir_1.path(), false);

    // Verify first output
    let manifest_1: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir_1.path().join("plugin.json")).expect("plugin.json exists"),
    )
    .unwrap();
    assert_eq!(manifest_1["name"], "sample-e2e-plugin");
    assert_eq!(manifest_1["version"], "1.0.0");
    assert_eq!(manifest_1["author"]["name"], "Integration Tester");
    assert_eq!(
        manifest_1["extensions"]["com.example.client"]["active"],
        true
    );

    let mcp_1: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir_1.path().join("mcp.json")).expect("mcp.json exists"),
    )
    .unwrap();
    assert_eq!(mcp_1["$schema"], MCP_CONFIG_SCHEMA);
    assert_eq!(
        mcp_1["mcpServers"]["fetch-server"]["command"],
        "./bin/fetch"
    );
    assert_eq!(
        mcp_1["mcpServers"]["remote-query"]["url"],
        "https://api.example.com/query"
    );

    assert!(out_dir_1.path().join("skills/git-flow/SKILL.md").exists());
    assert!(out_dir_1
        .path()
        .join("skills/git-flow/scripts/validate.sh")
        .exists());
    assert!(out_dir_1
        .path()
        .join("skills/code-review/SKILL.md")
        .exists());
    assert!(out_dir_1
        .path()
        .join("com.example.client/settings.json")
        .exists());

    // 2. Round-trip: transform first output to second output target
    let out_dir_2 = tempfile::tempdir().unwrap();
    fs::create_dir_all(out_dir_2.path()).unwrap();
    stage_and_apply(out_dir_1.path(), out_dir_2.path(), false);

    // Verify identical content across roundtrip
    for file in [
        "plugin.json",
        "mcp.json",
        "skills/git-flow/SKILL.md",
        "skills/git-flow/scripts/validate.sh",
        "skills/code-review/SKILL.md",
        "com.example.client/settings.json",
    ] {
        let bytes_1 = fs::read(out_dir_1.path().join(file)).unwrap();
        let bytes_2 = fs::read(out_dir_2.path().join(file)).unwrap();
        assert_eq!(
            bytes_1, bytes_2,
            "roundtrip artifact `{file}` must match identically"
        );
    }
}

#[test]
fn round_trip_agent_plugin_from_tar_archive() {
    let source_dir = tempfile::tempdir().unwrap();
    create_sample_plugin_tree(source_dir.path());
    let tar_bytes = create_tar_from_dir(source_dir.path());

    let tar_file = tempfile::Builder::new().suffix(".tar").tempfile().unwrap();
    fs::write(tar_file.path(), tar_bytes).unwrap();

    let out_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(out_dir.path()).unwrap();
    stage_and_apply(tar_file.path(), out_dir.path(), false);

    assert!(out_dir.path().join("plugin.json").exists());
    assert!(out_dir.path().join("mcp.json").exists());
    assert!(out_dir.path().join("skills/git-flow/SKILL.md").exists());
    assert!(out_dir.path().join("skills/code-review/SKILL.md").exists());
    assert!(out_dir
        .path()
        .join("com.example.client/settings.json")
        .exists());
}

#[test]
fn round_trip_agent_plugin_from_tar_stdin() {
    let source_dir = tempfile::tempdir().unwrap();
    create_sample_plugin_tree(source_dir.path());
    let tar_bytes = create_tar_from_dir(source_dir.path());

    let out_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(out_dir.path()).unwrap();
    let stage_dir = tempfile::tempdir().unwrap();
    let mut stage_cmd = Command::cargo_bin("rulette").unwrap();
    let staged = stage_cmd
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("agent-plugin")
        .arg("--target")
        .arg("agent-plugin@project")
        .arg("--project-root")
        .arg(out_dir.path())
        .arg("--stage")
        .arg(stage_dir.path().join("stage"))
        .write_stdin(tar_bytes)
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let digest = String::from_utf8(staged)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("plan digest: "))
        .expect("plan digest output")
        .trim()
        .to_owned();

    let mut apply_cmd = Command::cargo_bin("rulette").unwrap();
    apply_cmd
        .arg("transform")
        .arg("--apply")
        .arg(stage_dir.path().join("stage/rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(digest)
        .arg("--allow-project-root")
        .arg(out_dir.path())
        .assert()
        .success();

    assert!(out_dir.path().join("plugin.json").exists());
    assert!(out_dir.path().join("mcp.json").exists());
    assert!(out_dir.path().join("skills/git-flow/SKILL.md").exists());
}

#[test]
fn auto_detects_agent_plugin_input_frontend() {
    let source_dir = tempfile::tempdir().unwrap();
    create_sample_plugin_tree(source_dir.path());

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let output = cmd
        .arg("transform")
        .arg(source_dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let graph: serde_json::Value =
        serde_json::from_slice(&output).expect("valid compilation graph JSON");
    let packages = graph["packages"]
        .as_object()
        .expect("packages object in graph");

    // Expected: 1 manifest + 2 MCP servers + 2 skills + 1 extension = 6 packages
    assert_eq!(packages.len(), 6);
    for package in packages.values() {
        assert_eq!(
            package["provenance"]["frontend"], "agent-plugin",
            "frontend provenance must auto-detect as agent-plugin"
        );
    }
}

#[test]
fn transform_surfaces_invalid_mcp_servers_and_unknown_fields_warnings() {
    let source_dir = tempfile::tempdir().unwrap();
    let manifest = serde_json::json!({
        "$schema": PLUGIN_MANIFEST_SCHEMA,
        "name": "warning-plugin",
        "custom_unknown_field": "test"
    });
    fs::write(
        source_dir.path().join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mcp = serde_json::json!({
        "$schema": MCP_CONFIG_SCHEMA,
        "mcpServers": {
            "good-srv": {
                "type": "stdio",
                "command": "./bin/good"
            },
            "bad-srv": {
                "type": "stdio",
                "command": "../escape"
            }
        }
    });
    fs::write(
        source_dir.path().join("mcp.json"),
        serde_json::to_string_pretty(&mcp).unwrap(),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let transform_output = cmd
        .arg("transform")
        .arg(source_dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let graph: serde_json::Value =
        serde_json::from_slice(&transform_output).expect("valid compilation graph JSON");
    let diagnostics = graph["diagnostics"]
        .as_array()
        .expect("diagnostics array in graph");
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == "unknown-manifest-field"),
        "graph diagnostics must include unknown-manifest-field"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == "invalid-mcp-server"),
        "graph diagnostics must include invalid-mcp-server"
    );

    let out_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(out_dir.path()).unwrap();
    stage_and_apply(source_dir.path(), out_dir.path(), false);

    // Verify good server was retained in emitted mcp.json
    let mcp_emitted: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.path().join("mcp.json")).expect("mcp.json exists"),
    )
    .unwrap();
    assert!(mcp_emitted["mcpServers"].get("good-srv").is_some());
    assert!(mcp_emitted["mcpServers"].get("bad-srv").is_none());
}
