use assert_cmd::Command;
use insta::assert_snapshot;
use std::str;

fn emit_to_format(format: &str) -> String {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("tests/fixtures/emit_fixture.json")
        .arg("-o")
        .arg(format!("{}:-", format))
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    output
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\\n")
}

#[test]
fn test_emit_claude() {
    assert_snapshot!(emit_to_format("claude"));
}

#[test]
fn test_emit_cursor_mdc() {
    assert_snapshot!(emit_to_format("cursor-mdc"));
}

#[test]
fn test_emit_agent_skills() {
    assert_snapshot!(emit_to_format("agent-skills"));
}

#[test]
fn test_emit_copilot() {
    assert_snapshot!(emit_to_format("copilot"));
}

#[test]
fn test_emit_windsurf() {
    assert_snapshot!(emit_to_format("windsurf"));
}

#[test]
fn test_emit_codex() {
    assert_snapshot!(emit_to_format("codex"));
}

#[test]
fn test_emit_gemini() {
    assert_snapshot!(emit_to_format("gemini"));
}

#[test]
fn test_emit_ir_json() {
    assert_snapshot!(emit_to_format("ir-json"));
}

#[test]
fn test_emit_ir_toml() {
    assert_snapshot!(emit_to_format("ir-toml"));
}
