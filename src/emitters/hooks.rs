//! Polyglot PreToolUse hook manifest generation and emission.
//!
//! Generates configuration manifests for Google Antigravity, Cursor,
//! Claude Code, and OpenAI Codex to route tool invocation approvals
//! to `rulette hook-eval`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

/// Target harness for hook manifests.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum HookManifestTarget {
    Antigravity,
    Cursor,
    Claude,
    Codex,
}

impl HookManifestTarget {
    /// Returns the target identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Antigravity => "antigravity",
            Self::Cursor => "cursor",
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// Returns the default relative manifest file path.
    pub fn manifest_path(&self) -> &'static str {
        match self {
            Self::Antigravity => ".agents/hooks.json",
            Self::Cursor => ".cursor/hooks.json",
            Self::Claude => ".claude/settings.json",
            Self::Codex => ".codex/hooks.json",
        }
    }

    /// Parse target from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "antigravity" | "agy" => Some(Self::Antigravity),
            "cursor" => Some(Self::Cursor),
            "claude" | "claude-code" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    /// Returns all supported targets.
    pub fn all() -> &'static [Self] {
        &[Self::Antigravity, Self::Cursor, Self::Claude, Self::Codex]
    }
}

/// Construct invocation command string for `rulette hook-eval`.
///
/// Normalizes command on Windows hosts to `rulette.exe hook-eval`.
pub fn build_hook_command(policy_path: &str, is_windows: bool) -> String {
    let binary = if is_windows { "rulette.exe" } else { "rulette" };
    format!("{binary} hook-eval --policy {policy_path}")
}

/// Generate Google Antigravity hook manifest payload.
pub fn generate_antigravity_hook_manifest(command: &str) -> Value {
    json!({
        "rulette": {
            "PreToolUse": [
                {
                    "matcher": "run_command",
                    "hooks": [
                        {
                            "type": "command",
                            "command": command
                        }
                    ]
                }
            ]
        }
    })
}

/// Generate Cursor hook manifest payload.
pub fn generate_cursor_hook_manifest(command: &str) -> Value {
    json!({
        "version": 1,
        "hooks": {
            "beforeShellExecution": [
                {
                    "command": command
                }
            ]
        }
    })
}

/// Generate Claude Code settings hook manifest payload.
pub fn generate_claude_hook_manifest(command: &str) -> Value {
    json!({
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": command
                        }
                    ]
                }
            ]
        }
    })
}

/// Generate OpenAI Codex hook manifest payload.
pub fn generate_codex_hook_manifest(command: &str) -> Value {
    json!({
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "^Bash$",
                    "hooks": [
                        {
                            "type": "command",
                            "command": command,
                            "statusMessage": "Evaluating tool permission with rulette"
                        }
                    ]
                }
            ]
        }
    })
}

/// Generate hook manifest payload for specified target.
pub fn generate_hook_manifest(target: HookManifestTarget, command: &str) -> Value {
    match target {
        HookManifestTarget::Antigravity => generate_antigravity_hook_manifest(command),
        HookManifestTarget::Cursor => generate_cursor_hook_manifest(command),
        HookManifestTarget::Claude => generate_claude_hook_manifest(command),
        HookManifestTarget::Codex => generate_codex_hook_manifest(command),
    }
}

/// Merge hook manifest into existing JSON payload.
pub fn merge_hook_manifest(target: HookManifestTarget, existing: &Value, command: &str) -> Value {
    let mut base = match existing {
        Value::Object(map) => Value::Object(map.clone()),
        _ => return generate_hook_manifest(target, command),
    };

    match target {
        HookManifestTarget::Antigravity => {
            let hook_entry = json!({
                "PreToolUse": [
                    {
                        "matcher": "run_command",
                        "hooks": [
                            {
                                "type": "command",
                                "command": command
                            }
                        ]
                    }
                ]
            });
            base["rulette"] = hook_entry;
        }
        HookManifestTarget::Cursor => {
            base["version"] = json!(1);
            if !base.get("hooks").is_some_and(Value::is_object) {
                base["hooks"] = json!({});
            }
            let hooks_obj = base.get_mut("hooks").unwrap().as_object_mut().unwrap();
            let before_shell = hooks_obj
                .entry("beforeShellExecution")
                .or_insert_with(|| json!([]));

            if let Some(arr) = before_shell.as_array_mut() {
                let exists = arr
                    .iter()
                    .any(|item| item.get("command").and_then(Value::as_str) == Some(command));
                if !exists {
                    arr.push(json!({ "command": command }));
                }
            }
        }
        HookManifestTarget::Claude => {
            if !base.get("hooks").is_some_and(Value::is_object) {
                base["hooks"] = json!({});
            }
            let hooks_obj = base.get_mut("hooks").unwrap().as_object_mut().unwrap();
            let pre_tool = hooks_obj.entry("PreToolUse").or_insert_with(|| json!([]));

            if let Some(arr) = pre_tool.as_array_mut() {
                let exists = arr.iter().any(|item| {
                    item.get("hooks").is_some_and(|h| {
                        h.as_array().is_some_and(|sub| {
                            sub.iter()
                                .any(|s| s.get("command").and_then(Value::as_str) == Some(command))
                        })
                    })
                });
                if !exists {
                    arr.push(json!({
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": command
                            }
                        ]
                    }));
                }
            }
        }
        HookManifestTarget::Codex => {
            if !base.get("hooks").is_some_and(Value::is_object) {
                base["hooks"] = json!({});
            }
            let hooks_obj = base.get_mut("hooks").unwrap().as_object_mut().unwrap();
            let pre_tool = hooks_obj.entry("PreToolUse").or_insert_with(|| json!([]));

            if let Some(arr) = pre_tool.as_array_mut() {
                let exists = arr.iter().any(|item| {
                    item.get("hooks").is_some_and(|h| {
                        h.as_array().is_some_and(|sub| {
                            sub.iter()
                                .any(|s| s.get("command").and_then(Value::as_str) == Some(command))
                        })
                    })
                });
                if !exists {
                    arr.push(json!({
                        "matcher": "^Bash$",
                        "hooks": [
                            {
                                "type": "command",
                                "command": command,
                                "statusMessage": "Evaluating tool permission with rulette"
                            }
                        ]
                    }));
                }
            }
        }
    }

    base
}

/// Generated hook manifest item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolyglotHookManifest {
    pub target: HookManifestTarget,
    pub path: &'static str,
    pub content: Value,
}

/// Emit polyglot hook manifests for all supported harnesses.
pub fn emit_polyglot_hook_manifests(
    policy_path: &str,
    is_windows: bool,
) -> Vec<PolyglotHookManifest> {
    let command = build_hook_command(policy_path, is_windows);
    HookManifestTarget::all()
        .iter()
        .map(|&target| PolyglotHookManifest {
            target,
            path: target.manifest_path(),
            content: generate_hook_manifest(target, &command),
        })
        .collect()
}

/// Write single hook manifest to root directory, merging with existing file if present.
pub fn write_hook_manifest(
    root: &Path,
    target: HookManifestTarget,
    policy_path: &str,
    is_windows: bool,
) -> Result<PathBuf> {
    let command = build_hook_command(policy_path, is_windows);
    let relative_path = target.manifest_path();
    let full_path = root.join(relative_path);

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for {:?}", full_path))?;
    }

    let merged_value = if full_path.exists() {
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read existing manifest at {:?}", full_path))?;
        let existing: Value = serde_json::from_str(&content).unwrap_or(Value::Null);
        merge_hook_manifest(target, &existing, &command)
    } else {
        generate_hook_manifest(target, &command)
    };

    let formatted = serde_json::to_string_pretty(&merged_value)
        .context("Failed to format hook manifest JSON")?;
    fs::write(&full_path, format!("{formatted}\n"))
        .with_context(|| format!("Failed to write manifest to {:?}", full_path))?;

    Ok(full_path)
}

/// Write all polyglot hook manifests to root directory.
pub fn write_polyglot_hook_manifests(
    root: &Path,
    policy_path: &str,
    is_windows: bool,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for &target in HookManifestTarget::all() {
        let path = write_hook_manifest(root, target, policy_path, is_windows)?;
        paths.push(path);
    }
    Ok(paths)
}
