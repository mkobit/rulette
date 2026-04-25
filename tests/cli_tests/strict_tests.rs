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
fn test_skill_to_claude_strict_error() {
    let original_file = "tests/fixtures/agent-skills/example.skill.md";

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(original_file)
        .arg("--to")
        .arg("claude")
        .arg("--strict")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: Lossy conversion"));
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
