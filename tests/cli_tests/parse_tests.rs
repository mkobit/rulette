use assert_cmd::Command;
use insta::assert_snapshot;
use std::str;

#[test]
fn test_claude_skill_parsing() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("parse")
        .arg("tests/fixtures/claude/example.md")
        .arg("--from")
        .arg("claude")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let normalized_output = output.replace("\r\n", "\n");
    assert_snapshot!(normalized_output);
}

#[test]
fn test_agent_skills_parsing() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("parse")
        .arg("tests/fixtures/agent-skills/example.skill.md")
        .arg("--from")
        .arg("agent-skills")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let normalized_output = output.replace("\r\n", "\n");
    assert_snapshot!(normalized_output);
}

#[test]
fn test_codex_parsing() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("parse")
        .arg("tests/fixtures/codex/CLAUDE.md")
        .arg("--from")
        .arg("codex")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let normalized_output = output.replace("\r\n", "\n");
    assert_snapshot!(normalized_output);
}
