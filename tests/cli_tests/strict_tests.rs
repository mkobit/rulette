use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_rule_to_agent_skills_warning() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "---").unwrap();
    writeln!(temp_file, "name: just-a-rule").unwrap();
    writeln!(temp_file, "---").unwrap();
    writeln!(temp_file, "Rule body").unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(temp_file.path())
        .arg("--to")
        .arg("agent-skills")
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning: Lossy conversion"));
}

#[test]
fn test_rule_to_agent_skills_strict_error() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "---").unwrap();
    writeln!(temp_file, "name: just-a-rule").unwrap();
    writeln!(temp_file, "---").unwrap();
    writeln!(temp_file, "Rule body").unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(temp_file.path())
        .arg("--to")
        .arg("agent-skills")
        .arg("--strict")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: Lossy conversion"));
}

#[test]
fn test_agent_to_claude_strict_error() {
    let original_file = "tests/fixtures/gemini/subagent.md";

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(original_file)
        .arg("--from")
        .arg("gemini")
        .arg("--to")
        .arg("claude")
        .arg("--strict")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: Lossy conversion"));
}

fn write_hook_fixture(temp_dir: &std::path::Path) {
    let claude_dir = temp_dir.join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(
        claude_dir.join("settings.json"),
        r#"{
  "hooks": {
    "PreToolUse": [
      { "hooks": [ { "type": "command", "command": "python3 script.py" } ] }
    ]
  }
}"#,
    )
    .unwrap();
}

#[test]
fn test_hook_to_codex_strict_error() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_hook_fixture(temp_dir.path());

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(temp_dir.path())
        .arg("--to")
        .arg("codex")
        .arg("--strict")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: Lossy conversion: Hook"));
}

#[test]
fn test_hook_to_codex_warning() {
    let temp_dir = tempfile::tempdir().unwrap();
    write_hook_fixture(temp_dir.path());

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(temp_dir.path())
        .arg("--to")
        .arg("codex")
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning: Lossy conversion: Hook"));
}

#[test]
fn test_rule_to_cursor_mcp_strict_error() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "---").unwrap();
    writeln!(temp_file, "name: just-a-rule").unwrap();
    writeln!(temp_file, "---").unwrap();
    writeln!(temp_file, "Rule body").unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(temp_file.path())
        .arg("--to")
        .arg("cursor-mcp")
        .arg("--strict")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: Lossy conversion: Rule"));
}

#[test]
fn test_gemini_agent_real_file_round_trip_strict_success() {
    // Regression test: parsing from a real file path (not stdin/synthetic IR)
    // injects an internal rulette:source_file bookkeeping key into extra.
    // That key must not be treated as lossy user metadata nor leaked into
    // the emitted output.
    let original_file = "tests/fixtures/gemini/subagent.md";

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(original_file)
        .arg("--from")
        .arg("gemini")
        .arg("--to")
        .arg("gemini")
        .arg("--strict")
        .assert()
        .success()
        .stderr(predicate::str::contains("Error: Lossy conversion").not());

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    assert!(
        !output.contains("rulette:source_file"),
        "internal bookkeeping key leaked into emitted output: {output}"
    );
}

#[test]
fn test_cursor_mdc_real_file_does_not_leak_source_file() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("tests/fixtures/cursor/example.mdc")
        .arg("--to")
        .arg("cursor-mdc")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    assert!(
        !output.contains("rulette:source_file"),
        "internal bookkeeping key leaked into emitted output: {output}"
    );
}

#[test]
fn test_lossless_conversion_strict_success() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{{").unwrap();
    writeln!(temp_file, "  \"ir_version\": \"0.1\",").unwrap();
    writeln!(temp_file, "  \"entities\": [").unwrap();
    writeln!(temp_file, "    {{").unwrap();
    writeln!(temp_file, "      \"kind\": \"rule\",").unwrap();
    writeln!(temp_file, "      \"metadata\": {{").unwrap();
    writeln!(temp_file, "        \"name\": \"test-rule\"").unwrap();
    writeln!(temp_file, "      }},").unwrap();
    writeln!(temp_file, "      \"body\": \"just a body\"").unwrap();
    writeln!(temp_file, "    }}").unwrap();
    writeln!(temp_file, "  ]").unwrap();
    writeln!(temp_file, "}}").unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(temp_file.path())
        .arg("--from")
        .arg("ir-json")
        .arg("--to")
        .arg("ir-json")
        .arg("--strict")
        .assert()
        .success()
        .stderr(predicate::str::contains("Error: Lossy conversion").not());
}
