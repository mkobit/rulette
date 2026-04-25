use insta::assert_snapshot;
use std::fs;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};
use std::str;
use tempfile::tempdir;

#[test]
fn test_parse_transform_emit_pipeline() {
    let temp_dir = tempdir().unwrap();
    let input_dir = temp_dir.path().join("complex_input");
    fs::create_dir_all(&input_dir).unwrap();

    let claude_dir = input_dir.join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let mut settings_file = fs::File::create(claude_dir.join("settings.json")).unwrap();
    writeln!(
        settings_file,
        r#"{{
  "mcpServers": {{
    "filesystem": {{
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"]
    }}
  }},
  "permissions": {{
    "allowManagedPermissionRulesOnly": true
  }}
}}"#
    )
    .unwrap();

    let skills_dir = input_dir.join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    let mut skill1_file = fs::File::create(skills_dir.join("skill1.md")).unwrap();
    writeln!(
        skill1_file,
        r#"---
name: awesome-skill
description: An awesome skill
version: 1.0.0
license: MIT
---
# Awesome Skill
This is an awesome skill."#
    )
    .unwrap();

    let mut rule1_file = fs::File::create(input_dir.join("rule1.md")).unwrap();
    writeln!(
        rule1_file,
        r#"---
name: basic-rule
description: A basic rule
---
# Basic Rule
This is a basic rule."#
    )
    .unwrap();

    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    let mut parse_cmd = StdCommand::new(&cargo_bin);
    parse_cmd
        .arg("transform")
        .arg(input_dir.to_str().unwrap())
        .arg("--from")
        .arg("auto")
        .stdout(Stdio::piped());

    let mut parse_child = parse_cmd.spawn().expect("Failed to spawn parse command");

    let mut transform_cmd = StdCommand::new(&cargo_bin);
    transform_cmd
        .arg("transform")
        .arg("-")
        .arg("--exclude")
        .arg("mcp-server")
        .arg("--set")
        .arg("pipeline_processed=true")
        .stdin(parse_child.stdout.take().unwrap())
        .stdout(Stdio::piped());

    let mut transform_child = transform_cmd
        .spawn()
        .expect("Failed to spawn transform command");

    let mut emit_cmd = StdCommand::new(&cargo_bin);
    let output = emit_cmd
        .arg("transform")
        .arg("-")
        .arg("-o")
        .arg("ir-json:-")
        .stdin(transform_child.stdout.take().unwrap())
        .output()
        .expect("Failed to execute emit command");

    assert!(
        output.status.success(),
        "Pipeline failed: {}",
        str::from_utf8(&output.stderr).unwrap()
    );
    let _ = parse_child.wait();
    let _ = transform_child.wait();

    let output_str = str::from_utf8(&output.stdout).unwrap();

    let json: serde_json::Value = serde_json::from_str(output_str).unwrap();
    let mut entities = json.get("entities").unwrap().as_array().unwrap().clone();

    entities.sort_by(|a, b| {
        let name_a = a
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let name_b = b
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        name_a.cmp(name_b)
    });

    let sorted_json = serde_json::json!({ "entities": entities });
    let normalized_output = serde_json::to_string_pretty(&sorted_json).unwrap();

    assert_snapshot!(normalized_output);
}

#[test]
fn test_complex_pipeline_to_claude() {
    let temp_dir = tempdir().unwrap();
    let input_dir = temp_dir.path().join("input");
    fs::create_dir_all(&input_dir).unwrap();

    let mut skill_file = fs::File::create(input_dir.join("test.skill.md")).unwrap();
    writeln!(
        skill_file,
        r#"---
name: target-skill
description: A skill to emit
---
# Target
Content"#
    )
    .unwrap();

    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    let mut parse_cmd = StdCommand::new(&cargo_bin);
    let mut parse_child = parse_cmd
        .arg("transform")
        .arg(input_dir.to_str().unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn parse");

    let mut transform_cmd = StdCommand::new(&cargo_bin);
    let mut transform_child = transform_cmd
        .arg("transform")
        .arg("-")
        .arg("--set")
        .arg("injected_key=injected_val")
        .stdin(parse_child.stdout.take().unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn transform");

    let output_dir = temp_dir.path().join("output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut emit_cmd = StdCommand::new(&cargo_bin);
    let output = emit_cmd
        .arg("transform")
        .arg("-")
        .arg("-o")
        .arg(format!("claude:{}", output_dir.display()))
        .arg("-o")
        .arg("ir-toml:-")
        .stdin(transform_child.stdout.take().unwrap())
        .output()
        .expect("Failed to execute emit");

    assert!(
        output.status.success(),
        "Pipeline failed: {}",
        str::from_utf8(&output.stderr).unwrap()
    );
    let _ = parse_child.wait();
    let _ = transform_child.wait();

    // Verify Claude output (Claude puts it in CLAUDE.md for rules)
    let emitted_claude_file = output_dir.join("CLAUDE.md");
    assert!(emitted_claude_file.exists());
    let claude_content = fs::read_to_string(emitted_claude_file).unwrap();
    assert!(claude_content.contains("# Target\nContent"));

    // Verify TOML output preserves the injected key
    let output_str = str::from_utf8(&output.stdout).unwrap();
    assert!(output_str.contains("injected_key = \"injected_val\""));
}
