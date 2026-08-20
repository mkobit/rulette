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
        .arg("kind == \"mcp-server\"")
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

    // Sanitize absolute paths and normalize separators for cross-platform snapshot stability
    let raw_temp_str = temp_dir.path().to_string_lossy().replace("\\", "/");
    let canonical_temp_str = match fs::canonicalize(temp_dir.path()) {
        Ok(p) => p.to_string_lossy().replace("\\", "/"),
        Err(_) => raw_temp_str.clone(),
    };

    let mut temp_paths = vec![raw_temp_str, canonical_temp_str];
    for p in temp_paths.iter_mut() {
        if p.starts_with("//?/") {
            *p = p[4..].to_string();
        }
    }

    let mut normalized_output = normalized_output.replace("\\\\", "/");

    for p in temp_paths {
        let p_lower = p.to_lowercase();
        while let Some(pos) = normalized_output.to_lowercase().find(&p_lower) {
            normalized_output.replace_range(pos..pos + p.len(), "[TEMP_DIR]");
        }
    }

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

    // Verify Claude output (Skills go to individual SKILL.md files, matching
    // the format's own namesake for round-trip identity)
    let emitted_claude_file = output_dir.join("target-skill").join("SKILL.md");
    assert!(emitted_claude_file.exists());
    let claude_content = fs::read_to_string(emitted_claude_file).unwrap();
    assert!(claude_content.contains("# Target\nContent"));

    // Verify TOML output preserves the injected key
    let output_str = str::from_utf8(&output.stdout).unwrap();
    assert!(output_str.contains("injected_key = \"injected_val\""));
}

#[test]
fn test_claude_hook_roundtrip() {
    let temp_dir = tempdir().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let input_file = claude_dir.join("settings.json");
    fs::write(
        &input_file,
        r#"{
  "hooks": {
    "PreToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 script.py"
          }
        ]
      }
    ]
  }
}"#,
    )
    .unwrap();

    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    // Run transform: .claude/settings.json -> IR -> settings.json
    let mut cmd = StdCommand::new(&cargo_bin);
    let output_dir = temp_dir.path().join("output");
    fs::create_dir_all(&output_dir).unwrap();

    let output = cmd
        .arg("transform")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--to")
        .arg("claude")
        .arg("--out")
        .arg(output_dir.to_str().unwrap())
        .output()
        .expect("Failed to execute rulette");

    assert!(
        output.status.success(),
        "Command failed: {}",
        str::from_utf8(&output.stderr).unwrap()
    );

    let emitted_settings = output_dir.join("settings.json");
    assert!(emitted_settings.exists());
    let settings_content = fs::read_to_string(emitted_settings).unwrap();

    // Verify it preserved the semantic meaning in the output JSON
    let json: serde_json::Value = serde_json::from_str(&settings_content).unwrap();
    let hooks = json.get("hooks").unwrap().as_object().unwrap();
    let pre_tool_use = hooks.get("PreToolUse").unwrap().as_array().unwrap();
    let first_hook = pre_tool_use[0].get("hooks").unwrap().as_array().unwrap();
    assert_eq!(
        first_hook[0].get("command").unwrap().as_str().unwrap(),
        "python3 script.py"
    );
}

#[test]
fn test_rule_to_skill_promotion_pipeline() {
    let temp_dir = tempdir().unwrap();
    let input_file = temp_dir.path().join("generic-rule.md");
    fs::write(&input_file, "# Generic Rule\nThis is the rule body.").unwrap();

    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    let output_dir = temp_dir.path().join("promoted-skill");
    fs::create_dir_all(&output_dir).unwrap();

    // Promote generic rule to agent-skill
    let mut cmd = StdCommand::new(&cargo_bin);
    let output = cmd
        .arg("transform")
        .arg(input_file.to_str().unwrap())
        .arg("--to")
        .arg("agent-skills")
        .arg("--name")
        .arg("refactor-pro")
        .arg("--description")
        .arg("Advanced refactoring skill")
        .arg("--out")
        .arg(output_dir.to_str().unwrap())
        .output()
        .expect("Failed to execute rulette");

    assert!(
        output.status.success(),
        "Command failed: {}",
        str::from_utf8(&output.stderr).unwrap()
    );

    // Verify Agent Skill structure (namesake SKILL.md, directory form)
    let skill_md = output_dir.join("refactor-pro").join("SKILL.md");
    assert!(skill_md.exists());
    let content = fs::read_to_string(skill_md).unwrap();
    assert!(content.contains("name: refactor-pro"));
    assert!(content.contains("description: Advanced refactoring skill"));
    assert!(content.contains("# Generic Rule\nThis is the rule body."));
}

#[test]
fn test_multi_target_override_pipeline() {
    let temp_dir = tempdir().unwrap();
    let input_file = temp_dir.path().join("rule-with-overrides.md");
    fs::write(
        &input_file,
        r#"---
description: A rule with target-specific activation
rulette:activation:
  default:
    mode: [always]
  overrides:
    cursor:
      mode: [glob]
      globs: ["src/**/*.rs"]
---
# Overridden Rule
This is the rule body."#,
    )
    .unwrap();

    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    // 1. Transform to cursor-mdc stdout
    let mut cmd = StdCommand::new(&cargo_bin);
    let output = cmd
        .arg("transform")
        .arg(input_file.to_str().unwrap())
        .arg("-o")
        .arg("cursor-mdc:-")
        .output()
        .expect("Failed to execute rulette transform to cursor-mdc");
    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();
    assert!(stdout.contains("alwaysApply: false"));
    assert!(stdout.contains("globs: src/**/*.rs"));
    assert!(stdout.contains("# Overridden Rule"));

    // 2. Transform to claude stdout
    let mut cmd = StdCommand::new(&cargo_bin);
    let output = cmd
        .arg("transform")
        .arg(input_file.to_str().unwrap())
        .arg("-o")
        .arg("claude:-")
        .output()
        .expect("Failed to execute rulette transform to claude");
    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();
    assert!(stdout.contains("# Overridden Rule"));

    // 3. Transform to ir-json stdout and verify round-trip structure
    let mut cmd = StdCommand::new(&cargo_bin);
    let output = cmd
        .arg("transform")
        .arg(input_file.to_str().unwrap())
        .arg("-o")
        .arg("ir-json:-")
        .output()
        .expect("Failed to execute rulette transform to ir-json");
    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout).unwrap();
    let rule_entity = &json["entities"][0];
    let activation = &rule_entity["metadata"]["rulette:activation"];
    assert_eq!(
        activation["default"]["mode"],
        serde_json::json!(["always"])
    );
    assert_eq!(
        activation["overrides"]["cursor"]["mode"],
        serde_json::json!(["glob"])
    );
}

#[test]
fn test_antigravity_pipeline_transform_and_roundtrip() {
    let temp_dir = tempdir().unwrap();
    let input_file = temp_dir.path().join("rule_with_overrides.mdc");
    fs::write(
        &input_file,
        r#"---
description: Universal guideline
rulette:activation:
  default:
    mode:
      - glob
    globs:
      - "**/*.py"
  overrides:
    antigravity:
      mode:
        - model
      description: "Apply when python code is modified"
---
# Python Rule
Follow PEP 8."#,
    )
    .unwrap();

    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    // 1. Transform to antigravity stdout
    let mut cmd = StdCommand::new(&cargo_bin);
    let output = cmd
        .arg("transform")
        .arg(input_file.to_str().unwrap())
        .arg("-o")
        .arg("antigravity:-")
        .output()
        .expect("Failed to execute rulette transform to antigravity");
    assert!(output.status.success());
    let stdout = str::from_utf8(&output.stdout).unwrap();
    assert!(stdout.contains("trigger: model_decision"));
    assert!(stdout.contains("description: Universal guideline"));
    assert!(stdout.contains("# Python Rule"));

    // 2. Transform to an output directory and verify file structure
    let out_dir = temp_dir.path().join("antigravity_out");
    fs::create_dir_all(&out_dir).unwrap();

    let mut cmd = StdCommand::new(&cargo_bin);
    let output = cmd
        .arg("transform")
        .arg(input_file.to_str().unwrap())
        .arg("-o")
        .arg(format!("antigravity:{}", out_dir.to_str().unwrap()))
        .output()
        .expect("Failed to transform to antigravity directory");
    assert!(output.status.success());

    let rule_file = out_dir.join("rule_with_overrides.md");
    assert!(rule_file.exists());
    let emitted_rule = fs::read_to_string(&rule_file).unwrap();
    assert!(emitted_rule.contains("trigger: model_decision"));

    // 3. Re-parse emitted rule back through rulette
    let mut reparse_cmd = StdCommand::new(&cargo_bin);
    let reparse_out = reparse_cmd
        .arg("transform")
        .arg(rule_file.to_str().unwrap())
        .output()
        .expect("Failed to re-parse emitted antigravity rule");
    assert!(reparse_out.status.success());
    let reparse_json: serde_json::Value =
        serde_json::from_str(str::from_utf8(&reparse_out.stdout).unwrap()).unwrap();
    let re_entity = &reparse_json["entities"][0];
    assert_eq!(
        re_entity["metadata"]["rulette:activation"]["mode"],
        serde_json::json!(["model"])
    );
}

