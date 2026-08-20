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
    output.replace("\r\n", "\n").replace("\\r\\n", "\\n")
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
fn test_emit_antigravity() {
    assert_snapshot!(emit_to_format("antigravity"));
}

#[test]
fn test_emit_cursor_mcp() {
    let ir = r#"{
      "entities": [
        {
          "kind": "mcp-server",
          "metadata": { "name": "filesystem" },
          "config": {
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"],
            "env": { "FOO": "bar" }
          }
        }
      ]
    }"#;

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("ir-json")
        .arg("-o")
        .arg("cursor-mcp:-")
        .write_stdin(ir)
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(output).unwrap();
    let server = &json["mcpServers"]["filesystem"];
    assert_eq!(server["command"].as_str().unwrap(), "npx");
    assert_eq!(
        server["args"].as_array().unwrap(),
        &vec![
            serde_json::json!("-y"),
            serde_json::json!("@modelcontextprotocol/server-filesystem"),
            serde_json::json!("/path")
        ]
    );
    assert_eq!(server["env"]["FOO"].as_str().unwrap(), "bar");
}

#[test]
fn test_cursor_mcp_round_trip() {
    let ir = r#"{
      "entities": [
        {
          "kind": "mcp-server",
          "metadata": { "name": "filesystem" },
          "config": { "command": "npx", "args": [], "env": {} }
        }
      ]
    }"#;

    let mut emit_cmd = Command::cargo_bin("rulette").unwrap();
    let emitted = emit_cmd
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("ir-json")
        .arg("-o")
        .arg("cursor-mcp:-")
        .write_stdin(ir)
        .assert()
        .success();
    let cursor_mcp_json = str::from_utf8(&emitted.get_output().stdout).unwrap();

    let mut parse_cmd = Command::cargo_bin("rulette").unwrap();
    let parsed = parse_cmd
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("cursor-mcp")
        .write_stdin(cursor_mcp_json)
        .assert()
        .success();
    let parsed_output = str::from_utf8(&parsed.get_output().stdout).unwrap();
    let parsed_json: serde_json::Value = serde_json::from_str(parsed_output).unwrap();

    let entities = parsed_json["entities"].as_array().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0]["kind"].as_str().unwrap(), "mcp-server");
    assert_eq!(
        entities[0]["metadata"]["name"].as_str().unwrap(),
        "filesystem"
    );
    assert_eq!(entities[0]["config"]["command"].as_str().unwrap(), "npx");
}

#[test]
fn test_codex_directory_scope_groups_into_nested_agents_md() {
    let ir = r#"{
      "entities": [
        {
          "kind": "rule",
          "metadata": { "name": "backend-rule", "rulette:directory-scope": "src/backend" },
          "body": "Use Go idioms."
        },
        {
          "kind": "rule",
          "metadata": { "name": "frontend-rule", "rulette:directory-scope": "src/frontend" },
          "body": "Use React idioms."
        },
        {
          "kind": "rule",
          "metadata": { "name": "global-rule" },
          "body": "Always write tests."
        }
      ]
    }"#;

    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg("-")
        .arg("--from")
        .arg("ir-json")
        .arg("-o")
        .arg(format!("codex:{}", temp_dir.path().display()))
        .write_stdin(ir)
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(temp_dir.path().join("AGENTS.md")).unwrap(),
        "Always write tests."
    );
    assert_eq!(
        std::fs::read_to_string(temp_dir.path().join("src/backend/AGENTS.md")).unwrap(),
        "Use Go idioms."
    );
    assert_eq!(
        std::fs::read_to_string(temp_dir.path().join("src/frontend/AGENTS.md")).unwrap(),
        "Use React idioms."
    );
}

#[test]
fn test_codex_directory_scope_inferred_from_real_nested_agents_md_tree() {
    // Parsing real nested AGENTS.md files (not --set, not IR JSON) must infer
    // rulette:directory-scope from each file's location, so the round trip
    // through `transform` reproduces the same nested layout on emit.
    let input_dir = tempfile::tempdir().unwrap();
    std::fs::write(input_dir.path().join("AGENTS.md"), "Always write tests.").unwrap();
    std::fs::create_dir_all(input_dir.path().join("src/backend")).unwrap();
    std::fs::write(
        input_dir.path().join("src/backend/AGENTS.md"),
        "Use Go idioms.",
    )
    .unwrap();

    // Run with cwd set to the input dir and a relative "." input so the
    // walked filenames are relative, matching the PRD's own `./rules/`-style
    // examples -- an absolute input path is covered separately (see
    // infer_codex_directory_scope's own unit tests for that fallback).
    let output_dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.current_dir(input_dir.path())
        .arg("transform")
        .arg(".")
        .arg("-o")
        .arg(format!("codex:{}", output_dir.path().display()))
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(output_dir.path().join("AGENTS.md")).unwrap(),
        "Always write tests."
    );
    assert_eq!(
        std::fs::read_to_string(output_dir.path().join("src/backend/AGENTS.md")).unwrap(),
        "Use Go idioms."
    );
}

#[test]
fn test_codex_directory_scope_rejects_path_traversal() {
    let ir = r#"{
      "entities": [
        {
          "kind": "rule",
          "metadata": { "name": "malicious-rule", "rulette:directory-scope": "../../etc" },
          "body": "payload"
        }
      ]
    }"#;

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg("-")
        .arg("--from")
        .arg("ir-json")
        .arg("--to")
        .arg("codex")
        .write_stdin(ir)
        .assert()
        .failure()
        .stderr(predicates::str::contains("Invalid rulette:directory-scope"));
}

#[test]
fn test_emit_ir_json() {
    assert_snapshot!(emit_to_format("ir-json"));
}

#[test]
fn test_emit_ir_toml() {
    assert_snapshot!(emit_to_format("ir-toml"));
}
