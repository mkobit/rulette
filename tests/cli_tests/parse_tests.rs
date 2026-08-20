use assert_cmd::Command;
use insta::assert_snapshot;
use std::str;

#[test]
fn test_claude_skill_parsing() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("tests/fixtures/claude/example.md")
        .arg("--from")
        .arg("claude")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let normalized_output = output.replace("\r\n", "\n").replace("\\r\\n", "\\n");
    assert_snapshot!(normalized_output);
}

#[test]
fn test_agent_skills_invalid_name_length() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("tests/fixtures/agent-skills/invalid-name-length.skill.md")
        .arg("--from")
        .arg("agent-skills")
        .assert()
        .failure();

    let output = str::from_utf8(&assert.get_output().stderr).unwrap();
    assert!(output.contains("name length must be between 1 and 64 characters"));
}

#[test]
fn test_agent_skills_parsing() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("tests/fixtures/agent-skills/example.skill.md")
        .arg("--from")
        .arg("agent-skills")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let normalized_output = output.replace("\r\n", "\n").replace("\\r\\n", "\\n");
    assert_snapshot!(normalized_output);
}

#[test]
fn test_cursor_mdc_parsing() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("tests/fixtures/cursor/example.mdc")
        .arg("--from")
        .arg("cursor-mdc")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let normalized_output = output.replace("\r\n", "\n").replace("\\r\\n", "\\n");
    assert_snapshot!(normalized_output);
}

#[test]
fn test_cursor_mdc_auto_detection() {
    // Spec scenario: a .cursor/rules/*.mdc file passed without --from SHALL be
    // auto-detected as Cursor MDC and produce the same IR as an explicit --from.
    let mut explicit_cmd = Command::cargo_bin("rulette").unwrap();
    let explicit = explicit_cmd
        .arg("transform")
        .arg("tests/fixtures/cursor/example.mdc")
        .arg("--from")
        .arg("cursor-mdc")
        .assert()
        .success();
    let explicit_output = str::from_utf8(&explicit.get_output().stdout).unwrap();

    let mut auto_cmd = Command::cargo_bin("rulette").unwrap();
    let auto = auto_cmd
        .arg("transform")
        .arg("tests/fixtures/cursor/example.mdc")
        .assert()
        .success();
    let auto_output = str::from_utf8(&auto.get_output().stdout).unwrap();

    assert_eq!(auto_output, explicit_output);

    let json: serde_json::Value = serde_json::from_str(auto_output).unwrap();
    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0]["kind"].as_str().unwrap(), "rule");
    assert_eq!(
        entities[0]["metadata"]["description"].as_str().unwrap(),
        "Test rule for Cursor"
    );
}

#[test]
fn test_gemini_subagent_parsing() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("tests/fixtures/gemini/subagent.md")
        .arg("--from")
        .arg("gemini")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let normalized_output = output.replace("\r\n", "\n").replace("\\r\\n", "\\n");
    assert_snapshot!(normalized_output);
}

#[test]
fn test_antigravity_rule_parsing_and_auto_detection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let agy_dir = temp_dir.path().join(".antigravity");
    std::fs::create_dir_all(&agy_dir).unwrap();

    let rule_path = agy_dir.join("rust-rules.md");
    std::fs::write(
        &rule_path,
        r#"---
description: Rust guidelines
trigger: glob
globs:
  - "**/*.rs"
  - "**/Cargo.toml"
---
Always write idiomatic Rust."#,
    )
    .unwrap();

    // 1. Explicit --from antigravity
    let mut explicit_cmd = Command::cargo_bin("rulette").unwrap();
    let explicit = explicit_cmd
        .arg("transform")
        .arg(rule_path.to_str().unwrap())
        .arg("--from")
        .arg("antigravity")
        .assert()
        .success();
    let explicit_output = str::from_utf8(&explicit.get_output().stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(explicit_output).unwrap();
    let entity = &json["entities"][0];
    assert_eq!(entity["kind"].as_str().unwrap(), "rule");
    assert_eq!(
        entity["metadata"]["description"].as_str().unwrap(),
        "Rust guidelines"
    );
    assert_eq!(
        entity["metadata"]["rulette:activation"]["mode"],
        serde_json::json!(["glob"])
    );
    assert_eq!(
        entity["metadata"]["rulette:activation"]["globs"],
        serde_json::json!(["**/*.rs", "**/Cargo.toml"])
    );

    // 2. Auto-detection without --from
    let mut auto_cmd = Command::cargo_bin("rulette").unwrap();
    let auto = auto_cmd
        .arg("transform")
        .arg(rule_path.to_str().unwrap())
        .assert()
        .success();
    let auto_output = str::from_utf8(&auto.get_output().stdout).unwrap();
    assert_eq!(auto_output, explicit_output);
}
