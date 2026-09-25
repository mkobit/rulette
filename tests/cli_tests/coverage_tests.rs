use assert_cmd::Command;
use predicates::prelude::*;

const CODEX_FIXTURE: &str = "tests/fixtures/v0_1/codex";

#[test]
fn coverage_reports_core_target_package_kind_cells() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    let output = command
        .arg("-q")
        .arg("inspect")
        .arg(CODEX_FIXTURE)
        .arg("--coverage")
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(entries.iter().any(|entry| {
        entry["target"] == "cursor"
            && entry["package_kind"] == "skill"
            && entry["status"] == "dropped"
    }));
    assert!(entries.iter().all(|entry| {
        ["codex", "opencode", "claude", "cursor", "antigravity"]
            .contains(&entry["target"].as_str().unwrap())
    }));
}

#[test]
fn inspect_target_lists_structured_findings_with_provenance() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("inspect")
        .arg(CODEX_FIXTURE)
        .arg("--to")
        .arg("cursor")
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-lowered-as-rule"))
        .stdout(predicate::str::contains("provenance"));
}

#[test]
fn coverage_strict_is_scoped_to_inspect_and_fails_on_loss() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("-q")
        .arg("inspect")
        .arg(CODEX_FIXTURE)
        .arg("--coverage")
        .arg("--strict")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Coverage check failed"));
}

#[test]
fn global_strict_is_a_usage_error() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("--strict")
        .arg("inspect")
        .arg(CODEX_FIXTURE)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument '--strict'"));
}

#[test]
fn inspect_target_agent_plugin_reports_rule_loss_and_manifest_synthesis() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("inspect")
        .arg(CODEX_FIXTURE)
        .arg("--to")
        .arg("agent-plugin")
        .assert()
        .success()
        .stdout(predicate::str::contains("rule-lowered-as-skill"))
        .stdout(predicate::str::contains("synthesized-manifest"));
}

#[test]
fn inspect_agent_plugin_surfaces_diagnostics_and_extension_findings() {
    let temporary = tempfile::tempdir().unwrap();
    let plugin_json = r#"{
        "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
        "name": "my-plugin",
        "unknown_extra_field": "test_value"
    }"#;
    std::fs::write(temporary.path().join("plugin.json"), plugin_json).unwrap();

    let ext_dir = temporary.path().join("com.example.client");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(ext_dir.join("ext.json"), r#"{"key": "value"}"#).unwrap();

    let skill_dir = temporary.path().join("skills/sample");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: sample\ndescription: sample skill\n---\n# Sample\n",
    )
    .unwrap();

    // 1. Inspect graph surfaces non-fatal unknown field warning diagnostic
    let mut inspect_cmd = Command::cargo_bin("rulette").unwrap();
    let output = inspect_cmd
        .arg("inspect")
        .arg(temporary.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout_str = String::from_utf8(output).unwrap();
    assert!(stdout_str.contains("unknown-manifest-field"));
    assert!(stdout_str.contains("unknown_extra_field"));

    // 2. Inspect with --to codex reports dropped reverse-domain extension
    let mut inspect_codex = Command::cargo_bin("rulette").unwrap();
    inspect_codex
        .arg("inspect")
        .arg(temporary.path())
        .arg("--to")
        .arg("codex")
        .assert()
        .success()
        .stdout(predicate::str::contains("unsupported-semantic"))
        .stdout(predicate::str::contains("extension"));

    // 3. Inspect with --to agent-plugin retains reverse-domain extension and skill
    let mut inspect_self = Command::cargo_bin("rulette").unwrap();
    inspect_self
        .arg("inspect")
        .arg(temporary.path())
        .arg("--to")
        .arg("agent-plugin")
        .assert()
        .success()
        .stdout(predicate::str::contains("representable"));
}
