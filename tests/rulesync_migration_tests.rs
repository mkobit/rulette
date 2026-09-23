#![cfg(any(target_os = "linux", target_os = "android"))]

use assert_cmd::Command;
use rulette::{ActivationMode, PortableActivation, TargetActivation, TargetActivationOverrides};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Translates RuleSync target identifiers into Rulette target strings.
fn map_rulesync_target(target: &str) -> Option<(&'static str, &'static str)> {
    match target {
        "cursor" => Some(("cursor", "project")),
        "claudecode" => Some(("claude", "project")),
        "codexcli" => Some(("codex", "project")),
        "opencode" => Some(("opencode", "project")),
        "antigravity" => Some(("antigravity", "project")),
        _ => None,
    }
}

#[test]
fn test_rulesync_config_to_rulette_config_translation() {
    let fixture_path = Path::new("tests/fixtures/rulesync/.rulesync/rulesync.jsonc");
    let content = fs::read_to_string(fixture_path).expect("failed to read rulesync.jsonc fixture");

    let parsed: serde_json::Value =
        json5::from_str(&content).expect("failed to parse rulesync.jsonc fixture");
    let targets = parsed
        .get("targets")
        .and_then(|v| v.as_array())
        .expect("rulesync.jsonc missing targets array");

    let mut rulette_targets = Vec::new();
    for target in targets {
        let name = target.as_str().expect("target entry is not a string");
        if let Some((mapped_target, scope)) = map_rulesync_target(name) {
            rulette_targets.push(serde_json::json!({
                "target": mapped_target,
                "scope": scope,
            }));
        }
    }

    assert_eq!(rulette_targets.len(), 5);

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config_path = temp_dir.path().join("rulette.transform.jsonc");
    let rulette_config = serde_json::json!({
        "inputs": ["."],
        "targets": rulette_targets,
    });
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&rulette_config).unwrap(),
    )
    .expect("failed to write rulette.transform.jsonc");

    // Verify written config parses with json5 and contains expected targets
    let re_read = fs::read_to_string(&config_path).unwrap();
    let re_parsed: serde_json::Value = json5::from_str(&re_read).unwrap();
    assert_eq!(re_parsed["inputs"], serde_json::json!(["."]));
    let targets_arr = re_parsed["targets"].as_array().unwrap();
    assert_eq!(targets_arr.len(), 5);
}

#[test]
fn test_rulesync_rule_to_rulette_activation_translation() {
    let rule_fixture_path = Path::new("tests/fixtures/rulesync/.rulesync/rules/typescript.md");
    let raw = fs::read_to_string(rule_fixture_path).expect("failed to read typescript.md fixture");

    // Extract frontmatter
    assert!(raw.starts_with("---\n"));
    let end_fm = raw[4..]
        .find("\n---\n")
        .expect("closing frontmatter delimiter not found")
        + 4;
    let frontmatter_str = &raw[4..end_fm];
    let fm_yaml: serde_yaml::Value =
        serde_yaml::from_str(frontmatter_str).expect("frontmatter is valid yaml");

    // Translate RuleSync globs and per-tool blocks into typed TargetActivation
    let globs: Vec<String> = fm_yaml["globs"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();

    let default_activation = PortableActivation {
        mode: vec![ActivationMode::Glob],
        globs: Some(globs),
        pattern: None,
        description: None,
    };

    let mut overrides = BTreeMap::new();
    if let Some(cursor_block) = fm_yaml.get("cursor") {
        if cursor_block["alwaysApply"].as_bool() == Some(true) {
            overrides.insert(
                "cursor".to_owned(),
                PortableActivation {
                    mode: vec![ActivationMode::Always],
                    globs: None,
                    pattern: None,
                    description: None,
                },
            );
        }
    }

    if let Some(antigravity_block) = fm_yaml.get("antigravity") {
        if antigravity_block["trigger"].as_str() == Some("model_decision") {
            overrides.insert(
                "antigravity".to_owned(),
                PortableActivation {
                    mode: vec![ActivationMode::Model],
                    globs: None,
                    pattern: None,
                    description: antigravity_block["description"]
                        .as_str()
                        .map(ToOwned::to_owned),
                },
            );
        }
    }

    let target_activation = TargetActivation::Wrapped(TargetActivationOverrides {
        default: default_activation,
        overrides,
    });

    // Cursor resolves to Always mode
    let cursor_resolved = target_activation.resolve("cursor");
    assert_eq!(cursor_resolved.mode, vec![ActivationMode::Always]);

    // Antigravity resolves to Model mode with description
    let antigravity_resolved = target_activation.resolve("antigravity");
    assert_eq!(antigravity_resolved.mode, vec![ActivationMode::Model]);
    assert_eq!(
        antigravity_resolved.description.as_deref(),
        Some("Apply when editing or reviewing TypeScript files")
    );

    // Claude and others fall back to default Glob mode
    let claude_resolved = target_activation.resolve("claude");
    assert_eq!(claude_resolved.mode, vec![ActivationMode::Glob]);
    assert_eq!(
        claude_resolved.globs.as_deref(),
        Some(&["**/*.ts".to_owned(), "**/*.tsx".to_owned()][..])
    );

    let codex_resolved = target_activation.resolve("codex");
    assert_eq!(codex_resolved.mode, vec![ActivationMode::Glob]);

    let opencode_resolved = target_activation.resolve("opencode");
    assert_eq!(opencode_resolved.mode, vec![ActivationMode::Glob]);
}

#[test]
fn test_rulesync_migrated_rule_multi_target_emission_parity() {
    let temp_workspace = tempfile::tempdir().expect("failed to create temp workspace");
    let project_dir = temp_workspace.path().join("project");
    let stage_dir = temp_workspace.path().join("stage");
    fs::create_dir_all(&project_dir).unwrap();

    // Create a migrated rule in Claude format (.claude/rules/typescript.md)
    let claude_rules_dir = project_dir.join(".claude").join("rules");
    fs::create_dir_all(&claude_rules_dir).unwrap();
    let rule_content = r#"---
paths:
  - "**/*.ts"
  - "**/*.tsx"
---
# TypeScript conventions

Guidelines for writing maintainable, type-safe TypeScript code.

- Prefer `unknown` over `any` for unvalidated dynamic inputs.
- Enable `strict` mode in compiler configurations.
- Avoid non-null assertions (`!`) where optional chaining or type guards apply.
- Declare explicit return types on all exported functions.
"#;
    fs::write(claude_rules_dir.join("typescript.md"), rule_content).unwrap();

    // Stage transformation to Cursor, OpenCode, and Antigravity
    let mut stage_cmd = Command::cargo_bin("rulette").unwrap();
    let stage_output = stage_cmd
        .arg("transform")
        .arg(&project_dir)
        .arg("--from")
        .arg("claude")
        .arg("--target")
        .arg("cursor@project")
        .arg("--target")
        .arg("opencode@project")
        .arg("--target")
        .arg("antigravity@project")
        .arg("--stage")
        .arg(&stage_dir)
        .arg("--project-root")
        .arg(&project_dir)
        .output()
        .expect("failed to execute stage command");

    assert!(
        stage_output.status.success(),
        "staging failed: {}",
        String::from_utf8_lossy(&stage_output.stderr)
    );

    // Extract plan digest from stderr
    let stderr_str = String::from_utf8_lossy(&stage_output.stderr);
    let digest_line = stderr_str
        .lines()
        .find(|l| l.contains("plan digest:"))
        .expect("plan digest not printed to stderr");
    let digest = digest_line
        .split_whitespace()
        .last()
        .expect("could not extract digest token");

    // Apply the plan
    let mut apply_cmd = Command::cargo_bin("rulette").unwrap();
    let apply_output = apply_cmd
        .arg("transform")
        .arg("--apply")
        .arg(stage_dir.join("rulette.plan.json"))
        .arg("--expect-plan-sha256")
        .arg(digest)
        .arg("--allow-project-root")
        .arg(&project_dir)
        .output()
        .expect("failed to execute apply command");

    assert!(
        apply_output.status.success(),
        "apply failed: {}",
        String::from_utf8_lossy(&apply_output.stderr)
    );

    // Verify Cursor output
    let cursor_rule = project_dir
        .join(".cursor")
        .join("rules")
        .join(".claude")
        .join("rules")
        .join("typescript.md.mdc");
    assert!(
        cursor_rule.exists(),
        "Cursor rule not found: {:?}",
        cursor_rule
    );
    let cursor_content = fs::read_to_string(&cursor_rule).unwrap();
    assert!(cursor_content.contains("# TypeScript conventions"));
    assert!(cursor_content.contains("Prefer `unknown` over `any`"));

    // Verify OpenCode output
    let opencode_rule = project_dir
        .join(".opencode")
        .join("rules")
        .join(".claude")
        .join("rules")
        .join("typescript.md.md");
    assert!(
        opencode_rule.exists(),
        "OpenCode rule not found: {:?}",
        opencode_rule
    );
    let opencode_content = fs::read_to_string(&opencode_rule).unwrap();
    assert!(opencode_content.contains("# TypeScript conventions"));

    // Verify Antigravity output (.agents/rules/ mapped prefix)
    let antigravity_rule = project_dir
        .join(".agents")
        .join("rules")
        .join(".claude")
        .join("rules")
        .join("typescript.md.md");
    assert!(
        antigravity_rule.exists(),
        "Antigravity rule not found: {:?}",
        antigravity_rule
    );
    let antigravity_content = fs::read_to_string(&antigravity_rule).unwrap();
    assert!(antigravity_content.contains("# TypeScript conventions"));
}
