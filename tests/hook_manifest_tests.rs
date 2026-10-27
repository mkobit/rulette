use assert_cmd::Command;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

use rulette::emitters::hooks::{
    build_hook_command, emit_polyglot_hook_manifests, generate_antigravity_hook_manifest,
    generate_claude_hook_manifest, generate_codex_hook_manifest, generate_cursor_hook_manifest,
    merge_hook_manifest, write_polyglot_hook_manifests, HookManifestTarget,
};

#[test]
fn test_build_hook_command_normalization() {
    let unix_cmd = build_hook_command(".rulette/policy.json", false);
    assert_eq!(unix_cmd, "rulette hook-eval --policy .rulette/policy.json");

    let win_cmd = build_hook_command(".rulette/policy.json", true);
    assert_eq!(
        win_cmd,
        "rulette.exe hook-eval --policy .rulette/policy.json"
    );
}

#[test]
fn test_generate_antigravity_hook_manifest() {
    let cmd = "rulette hook-eval --policy .rulette/policy.json";
    let manifest = generate_antigravity_hook_manifest(cmd);

    assert_eq!(
        manifest["rulette"]["PreToolUse"][0]["matcher"],
        "run_command"
    );
    assert_eq!(
        manifest["rulette"]["PreToolUse"][0]["hooks"][0]["type"],
        "command"
    );
    assert_eq!(
        manifest["rulette"]["PreToolUse"][0]["hooks"][0]["command"],
        cmd
    );
}

#[test]
fn test_generate_cursor_hook_manifest() {
    let cmd = "rulette hook-eval --policy .rulette/policy.json";
    let manifest = generate_cursor_hook_manifest(cmd);

    assert_eq!(manifest["version"], 1);
    assert_eq!(manifest["hooks"]["beforeShellExecution"][0]["command"], cmd);
}

#[test]
fn test_generate_claude_hook_manifest() {
    let cmd = "rulette hook-eval --policy .rulette/policy.json";
    let manifest = generate_claude_hook_manifest(cmd);

    assert_eq!(manifest["hooks"]["PreToolUse"][0]["matcher"], "Bash");
    assert_eq!(
        manifest["hooks"]["PreToolUse"][0]["hooks"][0]["type"],
        "command"
    );
    assert_eq!(
        manifest["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        cmd
    );
}

#[test]
fn test_generate_codex_hook_manifest() {
    let cmd = "rulette hook-eval --policy .rulette/policy.json";
    let manifest = generate_codex_hook_manifest(cmd);

    assert_eq!(manifest["hooks"]["PreToolUse"][0]["matcher"], "^Bash$");
    assert_eq!(
        manifest["hooks"]["PreToolUse"][0]["hooks"][0]["type"],
        "command"
    );
    assert_eq!(
        manifest["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        cmd
    );
}

#[test]
fn test_emit_polyglot_hook_manifests_paths_and_targets() {
    let manifests = emit_polyglot_hook_manifests(".rulette/policy.json", false);
    assert_eq!(manifests.len(), 4);

    let agy = manifests
        .iter()
        .find(|m| m.target == HookManifestTarget::Antigravity)
        .unwrap();
    assert_eq!(agy.path, ".agents/hooks.json");

    let cursor = manifests
        .iter()
        .find(|m| m.target == HookManifestTarget::Cursor)
        .unwrap();
    assert_eq!(cursor.path, ".cursor/hooks.json");

    let claude = manifests
        .iter()
        .find(|m| m.target == HookManifestTarget::Claude)
        .unwrap();
    assert_eq!(claude.path, ".claude/settings.json");

    let codex = manifests
        .iter()
        .find(|m| m.target == HookManifestTarget::Codex)
        .unwrap();
    assert_eq!(codex.path, ".codex/hooks.json");
}

#[test]
fn test_merge_hook_manifest_preserves_existing_keys() {
    let cmd = "rulette hook-eval --policy .rulette/policy.json";

    // Test Claude settings merge
    let existing_claude = json!({
        "permissions": { "allow": ["Bash(ls)"] },
        "hooks": {
            "SessionStart": [{ "matcher": "", "hooks": [{ "command": "bd prime" }] }]
        }
    });
    let merged_claude = merge_hook_manifest(HookManifestTarget::Claude, &existing_claude, cmd);
    assert!(merged_claude["permissions"]["allow"].is_array());
    assert_eq!(
        merged_claude["hooks"]["SessionStart"][0]["hooks"][0]["command"],
        "bd prime"
    );
    assert_eq!(
        merged_claude["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        cmd
    );

    // Test Codex hooks merge
    let existing_codex = json!({
        "hooks": {
            "UserPromptSubmit": [{ "hooks": [{ "command": "bd codex-hook" }] }]
        }
    });
    let merged_codex = merge_hook_manifest(HookManifestTarget::Codex, &existing_codex, cmd);
    assert_eq!(
        merged_codex["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
        "bd codex-hook"
    );
    assert_eq!(
        merged_codex["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        cmd
    );
}

#[test]
fn test_write_polyglot_hook_manifests_filesystem() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let written = write_polyglot_hook_manifests(root, ".rulette/policy.json", false).unwrap();
    assert_eq!(written.len(), 4);

    assert!(root.join(".agents/hooks.json").exists());
    assert!(root.join(".cursor/hooks.json").exists());
    assert!(root.join(".claude/settings.json").exists());
    assert!(root.join(".codex/hooks.json").exists());

    let claude_str = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
    assert!(claude_str.contains("rulette hook-eval --policy .rulette/policy.json"));
}

#[test]
fn test_cli_hook_manifest_command_dry_run_and_write() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("hook-manifest")
        .arg("--policy")
        .arg(".rulette/policy.json")
        .arg("--dry-run")
        .assert();

    let output = assert.success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains(".agents/hooks.json"));
    assert!(stdout.contains(".cursor/hooks.json"));
    assert!(stdout.contains(".claude/settings.json"));
    assert!(stdout.contains(".codex/hooks.json"));

    // Now write to directory
    let mut write_cmd = Command::cargo_bin("rulette").unwrap();
    write_cmd
        .arg("hook-manifest")
        .arg("--policy")
        .arg(".rulette/policy.json")
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--windows")
        .assert()
        .success();

    let cursor_content = fs::read_to_string(temp_dir.path().join(".cursor/hooks.json")).unwrap();
    assert!(cursor_content.contains("rulette.exe hook-eval --policy .rulette/policy.json"));

    // Test single target emission
    let single_dir = TempDir::new().unwrap();
    let mut target_cmd = Command::cargo_bin("rulette").unwrap();
    target_cmd
        .arg("hook-manifest")
        .arg("--policy")
        .arg(".rulette/policy.json")
        .arg("--target")
        .arg("antigravity")
        .arg("--output-dir")
        .arg(single_dir.path())
        .assert()
        .success();

    assert!(single_dir.path().join(".agents/hooks.json").exists());
    assert!(!single_dir.path().join(".cursor/hooks.json").exists());

    // Test hook-eval --emit-manifests flag
    let eval_emit_dir = TempDir::new().unwrap();
    let mut eval_cmd = Command::cargo_bin("rulette").unwrap();
    eval_cmd
        .arg("hook-eval")
        .arg("--policy")
        .arg(".rulette/policy.json")
        .arg("--emit-manifests")
        .arg(eval_emit_dir.path())
        .assert()
        .success();

    assert!(eval_emit_dir.path().join(".claude/settings.json").exists());
    assert!(eval_emit_dir.path().join(".codex/hooks.json").exists());
}
