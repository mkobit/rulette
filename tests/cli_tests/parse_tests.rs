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
    let normalized_output = output
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\\n")
        .replace("\\\\", "/");
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
    let normalized_output = output
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\\n")
        .replace("\\\\", "/");
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
    let normalized_output = output
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\\n")
        .replace("\\\\", "/");
    assert_snapshot!(normalized_output);
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
    let normalized_output = output
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\\n")
        .replace("\\\\", "/");
    assert_snapshot!(normalized_output);
}
