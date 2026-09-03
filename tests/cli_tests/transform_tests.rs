use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

const CODEX_FIXTURE: &str = "tests/fixtures/v0_1/codex";

fn graph_from(command: &mut Command) -> serde_json::Value {
    let output = command.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("transform must write a compilation graph JSON value")
}

#[test]
fn transform_outputs_a_graph_and_selects_exact_package_ids() {
    let mut full_command = Command::cargo_bin("rulette").unwrap();
    full_command.arg("transform").arg(CODEX_FIXTURE);
    let full_graph = graph_from(&mut full_command);
    let ids: Vec<_> = full_graph["packages"]
        .as_object()
        .expect("graph packages are keyed by package ID")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "the fixture has one rule and one skill package"
    );

    let mut selected_command = Command::cargo_bin("rulette").unwrap();
    selected_command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--select")
        .arg(&ids[1]);
    let selected_graph = graph_from(&mut selected_command);
    let selected_ids: Vec<_> = selected_graph["packages"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(selected_ids, vec![ids[1].clone()]);
}

#[test]
fn selection_union_is_deterministic_and_unknown_package_ids_fail() {
    let mut full_command = Command::cargo_bin("rulette").unwrap();
    full_command.arg("transform").arg(CODEX_FIXTURE);
    let full_graph = graph_from(&mut full_command);
    let ids: Vec<_> = full_graph["packages"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();

    let mut selected_command = Command::cargo_bin("rulette").unwrap();
    selected_command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--select")
        .arg(&ids[1])
        .arg("--select")
        .arg(&ids[0])
        .arg("--select")
        .arg(&ids[1]);
    let selected_graph = graph_from(&mut selected_command);
    let selected_ids: Vec<_> = selected_graph["packages"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(selected_ids, ids);

    let mut unknown_command = Command::cargo_bin("rulette").unwrap();
    unknown_command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--select")
        .arg("pkg_0000000000000000000000000000000000000000000000000000000000000000")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown package ID"));
}

#[test]
fn removed_mutation_flags_fail_during_argument_parsing() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("does-not-need-to-exist")
        .arg("--filter")
        .arg("kind == \"rule\"")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument '--filter'"));
}

#[test]
fn legacy_entity_interchange_is_not_a_graph_frontend() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("ir-json")
        .write_stdin(r#"{"entities": []}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'ir-json'"));
}

#[test]
fn graph_interchange_requires_an_explicit_decoder() {
    let graph = r#"{
  "graph_version": "0.1",
  "packages": {}
}"#;

    let mut auto = Command::cargo_bin("rulette").unwrap();
    auto.arg("transform")
        .arg("-")
        .write_stdin(graph)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "standard input requires an explicit --from",
        ));

    let mut explicit = Command::cargo_bin("rulette").unwrap();
    explicit
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("graph-json")
        .write_stdin(graph)
        .assert()
        .success();
}

#[test]
fn auto_rejects_native_and_neutral_path_graph_interchange() {
    let temporary = tempfile::tempdir().unwrap();
    let native = temporary.path().join("AGENTS.md");
    let graph = temporary.path().join("snapshot.data");
    std::fs::write(&native, "Follow the repository guidance.\n").unwrap();
    std::fs::write(
        &graph,
        r#"{
  "graph_version": "0.1",
  "packages": {}
}"#,
    )
    .unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(native)
        .arg(graph)
        .assert()
        .failure()
        .stderr(predicate::str::contains("native and graph interchange"));
}

#[test]
fn auto_rejects_graph_interchange_bytes_at_a_native_path() {
    let temporary = tempfile::tempdir().unwrap();
    let native = temporary.path().join("AGENTS.md");
    std::fs::write(
        &native,
        r#"{
  "graph_version": "0.1",
  "packages": {}
}"#,
    )
    .unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(native)
        .assert()
        .failure()
        .stderr(predicate::str::contains("native and graph interchange"));
}

#[test]
fn explicit_graph_selection_rejects_a_known_native_frontend_input() {
    let temporary = tempfile::tempdir().unwrap();
    let native = temporary.path().join("AGENTS.md");
    let graph = temporary.path().join("snapshot.json");
    std::fs::write(&native, "Follow the repository guidance.\n").unwrap();
    std::fs::write(
        &graph,
        r#"{
  "graph_version": "0.1",
  "packages": {}
}"#,
    )
    .unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(native)
        .arg(graph)
        .arg("--from")
        .arg("graph-json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("native and graph interchange"));
}

#[test]
fn auto_detects_a_standalone_cursor_mdc_rule() {
    let temporary = tempfile::tempdir().unwrap();
    let rule = temporary.path().join("rust.mdc");
    std::fs::write(&rule, "Use rustfmt.\n").unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command.arg("transform").arg(rule);
    let graph = graph_from(&mut command);

    let package = graph["packages"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(package["provenance"]["frontend"], "cursor");
}

#[test]
fn auto_detects_canonical_antigravity_layouts_and_rejects_them_for_codex() {
    for (path, contents) in [
        (
            ".antigravity/rules/rule.md",
            "Follow the repository guidance.\n",
        ),
        (".antigravity/settings.json", "{}"),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let file = source.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, contents).unwrap();

        let mut auto = Command::cargo_bin("rulette").unwrap();
        auto.arg("transform").arg(&source);
        let graph = graph_from(&mut auto);
        assert_eq!(
            graph["packages"]
                .as_object()
                .unwrap()
                .values()
                .next()
                .unwrap()["provenance"]["frontend"],
            "antigravity",
            "{path}"
        );

        let mut explicit_codex = Command::cargo_bin("rulette").unwrap();
        explicit_codex
            .arg("transform")
            .arg(&source)
            .arg("--from")
            .arg("codex")
            .assert()
            .failure()
            .stderr(predicate::str::contains("antigravity"));
    }
}

#[test]
fn explicit_native_rejects_a_standalone_cursor_mdc_rule() {
    let temporary = tempfile::tempdir().unwrap();
    let native = temporary.path().join("AGENTS.md");
    let rule = temporary.path().join("rust.mdc");
    std::fs::write(&native, "Follow the repository guidance.\n").unwrap();
    std::fs::write(&rule, "Use rustfmt.\n").unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(native)
        .arg(rule)
        .arg("--from")
        .arg("codex")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cursor-mdc"));
}

#[test]
fn native_frontends_emit_unrecognized_warnings_without_dropping_recognized_packages() {
    let fixtures = [
        ("codex", "AGENTS.md", "Follow the repository guidance.\n"),
        ("claude", "CLAUDE.md", "Follow the repository guidance.\n"),
        (
            "cursor-mdc",
            ".cursor/rules/rule.mdc",
            "Follow the repository guidance.\n",
        ),
        (
            "opencode",
            ".opencode/rules/rule.md",
            "Follow the repository guidance.\n",
        ),
        (
            "antigravity",
            ".agent/rules/rule.md",
            "Follow the repository guidance.\n",
        ),
    ];

    for (frontend, package_path, package_contents) in fixtures {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let package = source.join(package_path);
        std::fs::create_dir_all(package.parent().unwrap()).unwrap();
        std::fs::write(&package, package_contents).unwrap();
        std::fs::write(source.join("notes.txt"), "Safe but unrelated text.\n").unwrap();

        let mut command = Command::cargo_bin("rulette").unwrap();
        command
            .arg("transform")
            .arg(&source)
            .arg("--from")
            .arg(frontend);
        let graph = graph_from(&mut command);

        assert!(!graph["packages"].as_object().unwrap().is_empty());
        assert!(
            graph["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"] == "unrecognized-native-file"),
            "{frontend} must retain the unrecognized-file warning"
        );
    }
}

#[test]
fn native_frontends_retain_recognized_unsupported_content_as_packages() {
    let fixtures = [
        ("codex", ".codex/config.toml", "model = \"gpt-5\"\n"),
        ("claude", ".mcp.json", r#"{"mcpServers":{}}"#),
        ("cursor-mdc", ".cursor/mcp.json", "{}"),
        ("opencode", "opencode.json", "{}"),
        ("antigravity", ".antigravity/settings.json", "{}"),
    ];

    for (frontend, source_path, contents) in fixtures {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let file = source.join(source_path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, contents).unwrap();

        let mut command = Command::cargo_bin("rulette").unwrap();
        command
            .arg("transform")
            .arg(&source)
            .arg("--from")
            .arg(frontend);
        let graph = graph_from(&mut command);

        assert!(!graph["packages"].as_object().unwrap().is_empty());
    }
}

#[test]
fn native_frontends_preserve_recognized_malformed_content_as_errors() {
    let fixtures = [
        ("codex", "AGENTS.md", b"\xff".as_slice()),
        ("claude", "CLAUDE.md", b"\xff".as_slice()),
        (
            "cursor-mdc",
            ".cursor/rules/rule.mdc",
            b"---\n: invalid\n---\nrule\n".as_slice(),
        ),
        ("opencode", "opencode.json", b"{".as_slice()),
        (
            "antigravity",
            ".agent/rules/rule.md",
            b"---\n: invalid\n---\nrule\n".as_slice(),
        ),
    ];

    for (frontend, source_path, contents) in fixtures {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let file = source.join(source_path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, contents).unwrap();

        let mut command = Command::cargo_bin("rulette").unwrap();
        command
            .arg("transform")
            .arg(&source)
            .arg("--from")
            .arg(frontend)
            .assert()
            .failure();
    }
}

#[test]
fn native_frontends_reject_warning_only_source_sets() {
    for frontend in ["codex", "claude", "cursor-mdc", "opencode", "antigravity"] {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("notes.txt"), "Safe but unrelated text.\n").unwrap();

        let mut command = Command::cargo_bin("rulette").unwrap();
        command
            .arg("transform")
            .arg(&source)
            .arg("--from")
            .arg(frontend)
            .assert()
            .failure()
            .stderr(predicate::str::contains("unsupported source syntax"));
    }
}

#[test]
fn equal_root_native_skills_from_distinct_inputs_reach_stable_collisions() {
    for (frontend, skill_root) in [
        ("cursor-mdc", ".cursor/skills/review"),
        ("opencode", ".opencode/skills/review"),
        ("antigravity", ".antigravity/skills/review"),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let first = temporary.path().join("first");
        let second = temporary.path().join("second");

        for (source, description, body, companion) in [
            (&first, "First review", "# First\n", "first companion"),
            (&second, "Second review", "# Second\n", "second companion"),
        ] {
            let skill = source.join(skill_root);
            std::fs::create_dir_all(skill.join("scripts")).unwrap();
            std::fs::write(
                skill.join("SKILL.md"),
                format!("---\nname: review\ndescription: {description}\n---\n{body}"),
            )
            .unwrap();
            std::fs::write(skill.join("scripts/check"), companion).unwrap();
        }

        let stderr = [[&first, &second], [&second, &first]]
            .into_iter()
            .map(|inputs| {
                let mut command = Command::cargo_bin("rulette").unwrap();
                command
                    .arg("transform")
                    .arg(inputs[0])
                    .arg(inputs[1])
                    .arg("--from")
                    .arg(frontend)
                    .assert()
                    .failure()
                    .stderr(predicate::str::contains("aggregate package collisions:"))
                    .stderr(predicate::str::contains("semantic identity `skill:review`"))
                    .stderr(predicate::str::contains("outer input `"))
                    .get_output()
                    .stderr
                    .clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(stderr[0], stderr[1], "{frontend}");
    }
}

#[test]
fn byte_identical_native_duplicates_report_both_indexes_before_lowering() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    let stage_dir = temporary.path().join("stage");
    std::fs::create_dir(&project_root).unwrap();
    std::fs::write(project_root.join("sentinel.txt"), "keep me").unwrap();
    let first = temporary.path().join("first/.codex");
    let second = temporary.path().join("second/.codex");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("config.toml"), "model = \"gpt-5\"\n").unwrap();
    std::fs::write(second.join("config.toml"), "model = \"gpt-5\"\n").unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(temporary.path().join("first"))
        .arg(temporary.path().join("second"))
        .arg("--from")
        .arg("codex")
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg(project_root)
        .arg("--stage")
        .arg(&stage_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("aggregate package collisions:"))
        .stderr(predicate::str::contains(
            "semantic identity `unsupported:codex-config/.codex/config.toml`",
        ))
        .stderr(predicate::str::contains("package ID `pkg_"))
        .stderr(predicate::str::contains("outer input `"))
        .stderr(predicate::str::contains("unaccepted capability loss").not());
    assert!(!stage_dir.exists());
    assert_eq!(
        std::fs::read_to_string(temporary.path().join("project/sentinel.txt")).unwrap(),
        "keep me"
    );
}

#[test]
fn aggregate_collisions_precede_unavailable_target_mapping_errors() {
    let temporary = tempfile::tempdir().unwrap();
    let user_root = temporary.path().join("user");
    std::fs::create_dir(&user_root).unwrap();
    let first = temporary.path().join("first/.codex");
    let second = temporary.path().join("second/.codex");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("config.toml"), "model = \"gpt-5\"\n").unwrap();
    std::fs::write(second.join("config.toml"), "model = \"gpt-5\"\n").unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(temporary.path().join("first"))
        .arg(temporary.path().join("second"))
        .arg("--from")
        .arg("codex")
        .arg("--target")
        .arg("cursor@user")
        .arg("--user-root")
        .arg(format!("cursor={}", user_root.display()))
        .arg("--check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("aggregate package collisions:"))
        .stderr(predicate::str::contains("no v0.1 user mapping for cursor").not());
}

#[test]
fn source_discovery_precedes_backend_target_resolution() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    std::fs::create_dir(&project_root).unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(temporary.path().join("missing-source"))
        .arg("--from")
        .arg("codex")
        .arg("--target")
        .arg("unavailable@project")
        .arg("--project-root")
        .arg(project_root)
        .arg("--check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported v0.1 target").not());
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn repeated_target_spellings_stage_one_resolved_target() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("AGENTS.md");
    let project_root = temporary.path().join("project");
    let stage_dir = temporary.path().join("stage");
    std::fs::write(&source, "Follow the repository guidance.\n").unwrap();
    std::fs::create_dir(&project_root).unwrap();

    Command::cargo_bin("rulette")
        .unwrap()
        .arg("transform")
        .arg(source)
        .arg("--from")
        .arg("codex")
        .arg("--target")
        .arg("codex@project")
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg(project_root)
        .arg("--stage")
        .arg(&stage_dir)
        .assert()
        .success();

    let plan: serde_json::Value =
        serde_json::from_slice(&std::fs::read(stage_dir.join("rulette.plan.json")).unwrap())
            .unwrap();
    assert_eq!(plan["entries"].as_array().unwrap().len(), 1);
    assert_eq!(plan["entries"][0]["target"], "codex");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn multiple_resolved_targets_stage_independent_artifact_sets() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("AGENTS.md");
    let project_root = temporary.path().join("project");
    let stage_dir = temporary.path().join("stage");
    std::fs::write(&source, "Follow the repository guidance.\n").unwrap();
    std::fs::create_dir(&project_root).unwrap();

    Command::cargo_bin("rulette")
        .unwrap()
        .arg("transform")
        .arg(source)
        .arg("--from")
        .arg("codex")
        .arg("--target")
        .arg("codex@project")
        .arg("--target")
        .arg("opencode@project")
        .arg("--project-root")
        .arg(project_root)
        .arg("--stage")
        .arg(&stage_dir)
        .assert()
        .success();

    let plan: serde_json::Value =
        serde_json::from_slice(&std::fs::read(stage_dir.join("rulette.plan.json")).unwrap())
            .unwrap();
    let targets = plan["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["target"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        targets,
        std::collections::BTreeSet::from(["codex", "opencode"])
    );
    let entries = plan["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    let mut by_target = std::collections::BTreeMap::new();
    for entry in entries {
        by_target.insert(entry["target"].as_str().unwrap(), entry);
    }
    assert_eq!(by_target["codex"]["artifact"]["native_path"], "AGENTS.md");
    assert_eq!(
        by_target["opencode"]["artifact"]["native_path"],
        "rules/AGENTS.md.md"
    );
    assert_eq!(
        by_target["codex"]["source_package"],
        by_target["opencode"]["source_package"]
    );
    for entry in entries {
        let bytes =
            std::fs::read(stage_dir.join(entry["stage_artifact_path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes, b"Follow the repository guidance.\n");
    }
}

#[test]
fn tar_input_keeps_cursor_skill_companions_in_one_package() {
    let temporary = tempfile::tempdir().unwrap();
    let archive_path = temporary.path().join("cursor-snapshot.tar");
    std::fs::write(&archive_path, cursor_skill_tar_fixture()).unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(&archive_path)
        .arg("--from")
        .arg("cursor-mdc");
    let graph = graph_from(&mut command);
    let packages = graph["packages"].as_object().unwrap();
    assert_eq!(packages.len(), 1);
    let package = packages.values().next().unwrap();
    assert!(package["resources"].get("scripts/check").is_some());
}

#[test]
fn explicit_stdin_directory_and_stdin_tar_remain_separate_snapshot_inputs() {
    let temporary = tempfile::tempdir().unwrap();
    let skill = temporary.path().join("stdin/.cursor/skills/review");
    std::fs::create_dir_all(skill.join("scripts")).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: review\ndescription: Filesystem review\n---\n# Filesystem\n",
    )
    .unwrap();
    std::fs::write(
        skill.join("scripts/filesystem-check"),
        "filesystem companion",
    )
    .unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .current_dir(temporary.path())
        .arg("transform")
        .arg("stdin")
        .arg("-")
        .arg("--from")
        .arg("cursor-mdc")
        .write_stdin(cursor_skill_tar_fixture())
        .assert()
        .failure()
        .stderr(predicate::str::contains("aggregate package collisions:"))
        .stderr(predicate::str::contains("semantic identity `skill:review`"))
        .stderr(predicate::str::contains("outer input `"));
}

#[test]
fn plain_native_stdin_is_rejected() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("codex")
        .write_stdin("Follow the repository guidance.\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("plain native standard input"));
}

#[test]
fn tar_stdin_is_accepted_for_an_explicit_native_decoder() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("codex")
        .write_stdin(tar_fixture())
        .assert()
        .success();
}

#[test]
fn gzip_tar_stdin_is_accepted_for_an_explicit_native_decoder() {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&tar_fixture()).unwrap();
    let gzip_tar = encoder.finish().unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("codex")
        .write_stdin(gzip_tar)
        .assert()
        .success();
}

#[test]
fn auto_rejects_tar_stdin() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .write_stdin(tar_fixture())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "standard input requires an explicit --from",
        ));
}

#[test]
fn auto_rejects_gzip_tar_stdin() {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&tar_fixture()).unwrap();
    let gzip_tar = encoder.finish().unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .write_stdin(gzip_tar)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "standard input requires an explicit --from",
        ));
}

#[test]
fn repeated_stdin_is_rejected_before_reading_input() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .arg("-")
        .arg("--from")
        .arg("codex")
        .write_stdin("Follow the repository guidance.\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "standard input may be supplied only once",
        ));
}

fn tar_fixture() -> Vec<u8> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    let contents = b"Follow the repository guidance.\n";
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(contents.len() as u64);
    header.set_cksum();
    builder
        .append_data(&mut header, "AGENTS.md", contents.as_slice())
        .unwrap();
    builder.finish().unwrap();
    drop(builder);
    archive
}

fn cursor_skill_tar_fixture() -> Vec<u8> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    for (path, contents, mode) in [
        (
            ".cursor/skills/review/SKILL.md",
            b"---\nname: review\ndescription: Review changes\n---\n# Review\n".as_slice(),
            0o644,
        ),
        (
            ".cursor/skills/review/scripts/check",
            b"#!/bin/sh\nexit 0\n".as_slice(),
            0o755,
        ),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_mode(mode);
        header.set_size(contents.len() as u64);
        header.set_cksum();
        builder.append_data(&mut header, path, contents).unwrap();
    }
    builder.finish().unwrap();
    drop(builder);
    archive
}

#[test]
fn native_targets_require_a_stage_and_explicit_scope_roots() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    std::fs::create_dir(&project_root).unwrap();

    let mut missing_stage = Command::cargo_bin("rulette").unwrap();
    missing_stage
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg(&project_root)
        .assert()
        .failure()
        .stderr(predicate::str::contains("--target requires --stage"));

    let mut missing_root = Command::cargo_bin("rulette").unwrap();
    missing_root
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@project")
        .arg("--stage")
        .arg(temporary.path().join("stage"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("--project-root is required"));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn source_stage_writes_a_plan_and_keeps_graph_on_stdout() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    let stage = temporary.path().join("stage");
    std::fs::create_dir(&project_root).unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    let assertion = command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg(&project_root)
        .arg("--stage")
        .arg(&stage)
        .assert()
        .success()
        .stderr(predicate::str::contains("plan digest: sha256_"));

    let graph: serde_json::Value = serde_json::from_slice(&assertion.get_output().stdout).unwrap();
    assert!(graph.get("graph_version").is_some());
    assert!(stage.join("rulette.plan.json").is_file());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn source_check_reports_sorted_statuses_without_creating_a_stage() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    let stage = temporary.path().join("stage");
    std::fs::create_dir(&project_root).unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("-q")
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg(&project_root)
        .arg("--check")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(" absent"))
        .stderr(predicate::str::contains("Error:").not());

    assert!(!stage.exists());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn apply_requires_a_plan_digest_and_explicit_authority() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    let stage = temporary.path().join("stage");
    std::fs::create_dir(&project_root).unwrap();

    let mut stage_command = Command::cargo_bin("rulette").unwrap();
    stage_command
        .arg("-q")
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg(&project_root)
        .arg("--stage")
        .arg(&stage)
        .assert()
        .success();

    let mut apply = Command::cargo_bin("rulette").unwrap();
    apply
        .arg("transform")
        .arg("--apply")
        .arg(stage.join("rulette.plan.json"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("--expect-plan-sha256 is required"));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn plan_apply_uses_the_expected_digest_and_reports_created_entries() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    let stage = temporary.path().join("stage");
    std::fs::create_dir(&project_root).unwrap();

    let mut stage_command = Command::cargo_bin("rulette").unwrap();
    let staged = stage_command
        .arg("-q")
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg(&project_root)
        .arg("--stage")
        .arg(&stage)
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let digest = String::from_utf8(staged)
        .unwrap()
        .strip_prefix("plan digest: ")
        .unwrap()
        .trim()
        .to_owned();

    let mut apply = Command::cargo_bin("rulette").unwrap();
    apply
        .arg("transform")
        .arg("--apply")
        .arg(stage.join("rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(digest)
        .arg("--allow-project-root")
        .arg(&project_root)
        .assert()
        .success()
        .stderr(predicate::str::contains("created "));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn empty_authority_paths_are_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let stage = temporary.path().join("stage");
    let user_root = temporary.path().join("user");
    std::fs::create_dir(&user_root).unwrap();

    let mut empty_project_root = Command::cargo_bin("rulette").unwrap();
    empty_project_root
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@project")
        .arg("--project-root")
        .arg("")
        .arg("--stage")
        .arg(temporary.path().join("project-stage"))
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "a value is required for '--project-root",
        ));

    let mut stage_command = Command::cargo_bin("rulette").unwrap();
    let staged = stage_command
        .arg("-q")
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("codex@user")
        .arg("--user-root")
        .arg(format!("codex={}", user_root.display()))
        .arg("--stage")
        .arg(&stage)
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let digest = String::from_utf8(staged)
        .unwrap()
        .strip_prefix("plan digest: ")
        .unwrap()
        .trim()
        .to_owned();

    let mut empty_user_root = Command::cargo_bin("rulette").unwrap();
    empty_user_root
        .arg("transform")
        .arg("--apply")
        .arg(stage.join("rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(digest)
        .arg("--allow-user-root")
        .arg("codex=")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "authority root path may not be empty",
        ));
}
