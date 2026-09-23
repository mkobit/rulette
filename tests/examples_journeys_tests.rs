#![cfg(any(target_os = "linux", target_os = "android"))]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn extract_plan_digest(stderr: &str) -> String {
    for line in stderr.lines() {
        if let Some(digest) = line.strip_prefix("plan digest: ") {
            return digest.trim().to_owned();
        }
    }
    panic!("plan digest not found in stderr: {stderr}");
}

#[test]
fn test_journey_1_fanout_end_to_end() {
    let temp_workspace = tempfile::tempdir().expect("failed to create temp workspace");
    let project_dir = temp_workspace.path().join("project");
    let stage_dir = temp_workspace.path().join("stage");

    // Copy journey 1 fixture files to temp project
    fs::create_dir_all(project_dir.join("rules")).unwrap();
    fs::copy(
        "examples/journey-1-fanout/rules/conventions.md",
        project_dir.join("rules/conventions.md"),
    )
    .unwrap();
    fs::copy(
        "examples/journey-1-fanout/rulette.transform.jsonc",
        project_dir.join("rulette.transform.jsonc"),
    )
    .unwrap();

    // 1. Stage the publication plan
    let mut stage_cmd = Command::cargo_bin("rulette").unwrap();
    let stage_output = stage_cmd
        .current_dir(&project_dir)
        .arg("transform")
        .arg("--config")
        .arg("rulette.transform.jsonc")
        .arg("--from")
        .arg("antigravity")
        .arg("--stage")
        .arg(&stage_dir)
        .arg("--project-root")
        .arg(&project_dir)
        .assert()
        .success();

    let stderr = String::from_utf8(stage_output.get_output().stderr.clone()).unwrap();
    let digest = extract_plan_digest(&stderr);

    // 2. Preflight check before apply reports absent files
    let mut check_before = Command::cargo_bin("rulette").unwrap();
    check_before
        .current_dir(&project_dir)
        .arg("transform")
        .arg("--apply")
        .arg(stage_dir.join("rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(&digest)
        .arg("--allow-project-root")
        .arg(&project_dir)
        .arg("--check")
        .assert()
        .failure();

    // 3. Apply the publication plan
    let mut apply_cmd = Command::cargo_bin("rulette").unwrap();
    apply_cmd
        .current_dir(&project_dir)
        .arg("transform")
        .arg("--apply")
        .arg(stage_dir.join("rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(&digest)
        .arg("--allow-project-root")
        .arg(&project_dir)
        .assert()
        .success();

    // Verify all 5 destination files exist
    assert!(project_dir.join("AGENTS.md").exists());
    assert!(project_dir.join("CLAUDE.md").exists());
    assert!(project_dir.join(".cursor/rules/conventions.mdc").exists());
    assert!(project_dir.join(".opencode/rules/conventions.md").exists());
    assert!(project_dir.join(".agents/rules/conventions.md").exists());

    // 4. Verify check passes cleanly post-apply
    let mut check_after = Command::cargo_bin("rulette").unwrap();
    check_after
        .current_dir(&project_dir)
        .arg("transform")
        .arg("--config")
        .arg("rulette.transform.jsonc")
        .arg("--from")
        .arg("antigravity")
        .arg("--project-root")
        .arg(&project_dir)
        .arg("--check")
        .assert()
        .success()
        .stderr(predicate::str::contains("unchanged"));
}

#[test]
fn test_journey_2_ci_gates() {
    let example_dir = Path::new("examples/journey-2-ci-gates");

    // Coverage gate strict
    let mut inspect_cmd = Command::cargo_bin("rulette").unwrap();
    inspect_cmd
        .current_dir(example_dir)
        .arg("inspect")
        .arg("./rules/")
        .arg("--from")
        .arg("antigravity")
        .arg("--coverage")
        .arg("--strict")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "rule         codex        supported",
        ))
        .stdout(predicate::str::contains(
            "rule         cursor       supported",
        ));
}

#[test]
fn test_journey_3_consolidation_lossless_check() {
    let example_dir = Path::new("examples/journey-3-consolidation");

    // Pre-existing files faithfully match canonical rule lowering
    let mut check_cmd = Command::cargo_bin("rulette").unwrap();
    check_cmd
        .current_dir(example_dir)
        .arg("transform")
        .arg("--config")
        .arg("rulette.transform.jsonc")
        .arg("--from")
        .arg("antigravity")
        .arg("--project-root")
        .arg(".")
        .arg("--check")
        .assert()
        .success()
        .stderr(predicate::str::contains("unchanged"));
}

#[test]
fn test_journey_4_target_activation_overrides() {
    let temp_workspace = tempfile::tempdir().expect("failed to create temp workspace");
    let project_dir = temp_workspace.path().join("project");
    let stage_dir = temp_workspace.path().join("stage");

    fs::create_dir_all(project_dir.join("rules")).unwrap();
    fs::copy(
        "examples/journey-4-target-overrides/rules/typescript.md",
        project_dir.join("rules/typescript.md"),
    )
    .unwrap();
    fs::copy(
        "examples/journey-4-target-overrides/rulette.transform.jsonc",
        project_dir.join("rulette.transform.jsonc"),
    )
    .unwrap();

    let mut stage_cmd = Command::cargo_bin("rulette").unwrap();
    let stage_output = stage_cmd
        .current_dir(&project_dir)
        .arg("transform")
        .arg("--config")
        .arg("rulette.transform.jsonc")
        .arg("--from")
        .arg("antigravity")
        .arg("--stage")
        .arg(&stage_dir)
        .arg("--project-root")
        .arg(&project_dir)
        .assert()
        .success();

    let stderr = String::from_utf8(stage_output.get_output().stderr.clone()).unwrap();
    let digest = extract_plan_digest(&stderr);

    let mut apply_cmd = Command::cargo_bin("rulette").unwrap();
    apply_cmd
        .current_dir(&project_dir)
        .arg("transform")
        .arg("--apply")
        .arg(stage_dir.join("rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(&digest)
        .arg("--allow-project-root")
        .arg(&project_dir)
        .assert()
        .success();

    // Verify Cursor has alwaysApply: true
    let cursor_content =
        fs::read_to_string(project_dir.join(".cursor/rules/typescript.mdc")).unwrap();
    assert!(cursor_content.contains("alwaysApply: true"));

    // Verify Antigravity has trigger: model_decision
    let antigravity_content =
        fs::read_to_string(project_dir.join(".agents/rules/typescript.md")).unwrap();
    assert!(antigravity_content.contains("trigger: model_decision"));

    // Verify Claude has rule content
    let claude_content = fs::read_to_string(project_dir.join("CLAUDE.md")).unwrap();
    assert!(claude_content.contains("TypeScript conventions"));
}

#[test]
fn test_journey_5_inspect_and_coverage() {
    let example_dir = Path::new("examples/journey-5-inspect-coverage");

    // 1. Inspect target-specific lowering
    let mut inspect_to = Command::cargo_bin("rulette").unwrap();
    inspect_to
        .current_dir(example_dir)
        .arg("inspect")
        .arg("./rules/")
        .arg("--from")
        .arg("antigravity")
        .arg("--to")
        .arg("cursor")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "=== Capability findings for cursor ===",
        ))
        .stdout(predicate::str::contains("representable"));

    // 2. Cross-target coverage matrix
    let mut inspect_cov = Command::cargo_bin("rulette").unwrap();
    inspect_cov
        .current_dir(example_dir)
        .arg("inspect")
        .arg("./rules/")
        .arg("--from")
        .arg("antigravity")
        .arg("--coverage")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Coverage matrix ==="));

    // 3. Machine readable JSON coverage
    let mut inspect_json = Command::cargo_bin("rulette").unwrap();
    let json_output = inspect_json
        .current_dir(example_dir)
        .arg("inspect")
        .arg("./rules/")
        .arg("--from")
        .arg("antigravity")
        .arg("--coverage")
        .arg("--json")
        .assert()
        .success();

    let json_text = String::from_utf8(json_output.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    assert!(parsed.is_array());
    assert!(!parsed.as_array().unwrap().is_empty());
}
