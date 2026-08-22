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
