use assert_cmd::Command;
use predicates::prelude::*;

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
