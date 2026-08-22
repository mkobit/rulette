use assert_cmd::Command;
use predicates::prelude::*;

const CODEX_FIXTURE: &str = "tests/fixtures/v0_1/codex";

#[test]
fn native_lowering_rejects_loss_by_default() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("cursor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unaccepted capability loss"));
}

#[test]
fn allow_lossy_keeps_structured_findings_without_writing_native_output() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    let assertion = command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--target")
        .arg("cursor")
        .arg("--allow-lossy")
        .assert()
        .success()
        .stderr(predicate::str::contains("skill-lowered-as-rule"));

    let graph: serde_json::Value = serde_json::from_slice(&assertion.get_output().stdout)
        .expect("native lowering must still leave graph interchange on stdout");
    assert!(graph.get("graph_version").is_some());
}

#[test]
fn allow_lossy_is_inapplicable_to_lossless_graph_output() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--allow-lossy")
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires at least one --target"));
}

#[test]
fn transform_no_longer_accepts_strict() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--strict")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument '--strict'"));
}
