use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Args;
use serde_json::{json, Value};

use crate::ir::permissions::{
    resolve_permission_policy, PermissionAction, PermissionPolicy, ResolvedPermissionPolicy,
    TargetPermissionPolicy,
};

/// Arguments for `rulette hook-eval`.
#[derive(Args, Debug, Clone)]
pub struct HookEvalArgs {
    /// Path to the permission policy JSON file
    #[arg(long, value_name = "PATH")]
    pub policy: PathBuf,

    /// Target harness format (antigravity, cursor, claude, codex).
    /// Auto-detected from incoming payload if omitted.
    #[arg(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// Emit polyglot hook manifests to directory instead of evaluating stdin
    #[arg(long, value_name = "DIR")]
    pub emit_manifests: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookTarget {
    Antigravity,
    Cursor,
    Claude,
    Codex,
}

impl HookTarget {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "antigravity" | "agy" => Some(Self::Antigravity),
            "cursor" => Some(Self::Cursor),
            "claude" | "claude-code" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }
}

/// Custom error signal for Cursor hook denial exiting with code 2.
#[derive(Debug)]
pub struct HookDeniedExit(pub u8);

impl std::fmt::Display for HookDeniedExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hook denied with exit code {}", self.0)
    }
}

impl std::error::Error for HookDeniedExit {}

impl HookEvalArgs {
    pub fn execute(&self) -> Result<()> {
        if let Some(ref dir) = self.emit_manifests {
            let policy_str = self.policy.to_string_lossy();
            let is_windows = cfg!(windows);
            crate::emitters::hooks::write_polyglot_hook_manifests(dir, &policy_str, is_windows)?;
            return Ok(());
        }

        let policy_content = fs::read_to_string(&self.policy)
            .with_context(|| format!("Failed to read policy file at {:?}", self.policy))?;

        let resolved_policy = load_and_resolve_policy(&policy_content, self.target.as_deref())
            .with_context(|| "Failed to parse and resolve policy")?;

        let mut stdin_input = String::new();
        io::stdin()
            .read_to_string(&mut stdin_input)
            .with_context(|| "Failed to read payload from standard input")?;

        let payload: Value = serde_json::from_str(&stdin_input)
            .with_context(|| "Failed to parse incoming standard input payload as JSON")?;

        let target = match self.target.as_deref().and_then(HookTarget::parse) {
            Some(t) => t,
            None => detect_target_from_payload(&payload),
        };

        let (tool, invocation_arg, raw_command) = extract_invocation(&payload, target);

        let decision = if is_shell_tool(&tool) {
            let cmd = raw_command
                .as_deref()
                .unwrap_or(invocation_arg.as_deref().unwrap_or(""));
            evaluate_compound_shell_command(&resolved_policy, cmd)
        } else {
            resolved_policy.evaluate(&tool, invocation_arg.as_deref())
        };

        emit_response(target, decision, raw_command.as_deref())
    }
}

/// Load and resolve policy from string content.
pub fn load_and_resolve_policy(
    content: &str,
    target: Option<&str>,
) -> Result<ResolvedPermissionPolicy> {
    let policy: PermissionPolicy = if let Ok(target_policy) =
        serde_json::from_str::<TargetPermissionPolicy>(content)
    {
        target_policy.resolve(target.unwrap_or("default")).clone()
    } else if let Ok(plain_policy) = serde_json::from_str::<PermissionPolicy>(content) {
        plain_policy
    } else {
        bail!("Invalid policy JSON: must conform to TargetPermissionPolicy or PermissionPolicy");
    };

    resolve_permission_policy(&policy, "hook-eval", "policy")
}

/// Detect target harness from payload structure.
pub fn detect_target_from_payload(payload: &Value) -> HookTarget {
    if payload.get("toolCall").is_some() {
        HookTarget::Antigravity
    } else if payload.get("command").is_some() && payload.get("tool_input").is_none() {
        HookTarget::Cursor
    } else {
        HookTarget::Claude
    }
}

fn is_shell_tool(tool: &str) -> bool {
    let lower = tool.to_lowercase();
    matches!(
        lower.as_str(),
        "bash" | "sh" | "shell" | "run_command" | "command"
    )
}

/// Extract canonical tool, invocation argument, and raw command string from payload.
pub fn extract_invocation(
    payload: &Value,
    target: HookTarget,
) -> (String, Option<String>, Option<String>) {
    match target {
        HookTarget::Antigravity => {
            if let Some(tool_call) = payload.get("toolCall") {
                let name = tool_call
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("run_command");
                let args = tool_call.get("args");

                if name == "run_command" {
                    let cmd = args
                        .and_then(|a| {
                            a.get("CommandLine")
                                .or_else(|| a.get("command"))
                                .and_then(Value::as_str)
                        })
                        .map(str::to_string);
                    return ("bash".to_string(), cmd.clone(), cmd);
                }

                let arg = args.and_then(|a| {
                    if let Some(obj) = a.as_object() {
                        obj.values().find_map(Value::as_str).map(str::to_string)
                    } else {
                        None
                    }
                });
                (name.to_string(), arg, None)
            } else {
                ("unknown".to_string(), None, None)
            }
        }
        HookTarget::Cursor => {
            let cmd = payload
                .get("command")
                .and_then(Value::as_str)
                .map(str::to_string);
            ("bash".to_string(), cmd.clone(), cmd)
        }
        HookTarget::Claude | HookTarget::Codex => {
            let tool_name = payload
                .get("tool_name")
                .and_then(Value::as_str)
                .unwrap_or("Bash");
            let tool_input = payload.get("tool_input");

            if tool_name.eq_ignore_ascii_case("bash") || tool_name.eq_ignore_ascii_case("shell") {
                let cmd = tool_input
                    .and_then(|i| i.get("command").and_then(Value::as_str))
                    .map(str::to_string);
                ("bash".to_string(), cmd.clone(), cmd)
            } else {
                let arg = tool_input.and_then(|i| {
                    if let Some(obj) = i.as_object() {
                        obj.get("path")
                            .or_else(|| obj.get("file_path"))
                            .or_else(|| obj.get("url"))
                            .and_then(Value::as_str)
                            .or_else(|| obj.values().find_map(Value::as_str))
                            .map(str::to_string)
                    } else {
                        None
                    }
                });
                (tool_name.to_string(), arg, None)
            }
        }
    }
}

/// Tokenize and split compound shell command line on operators outside quotes: `&&`, `||`, `;`, `|`.
pub fn split_shell_commands(command: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        if escaped {
            current.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        if c == '\\' && !in_single_quote {
            escaped = true;
            current.push(c);
            i += 1;
            continue;
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            current.push(c);
            i += 1;
            continue;
        }

        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            current.push(c);
            i += 1;
            continue;
        }

        if !in_single_quote && !in_double_quote {
            if c == '&' && i + 1 < chars.len() && chars[i + 1] == '&' {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    commands.push(trimmed.to_string());
                }
                current.clear();
                i += 2;
                continue;
            }
            if c == '|' && i + 1 < chars.len() && chars[i + 1] == '|' {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    commands.push(trimmed.to_string());
                }
                current.clear();
                i += 2;
                continue;
            }
            if c == ';' {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    commands.push(trimmed.to_string());
                }
                current.clear();
                i += 1;
                continue;
            }
            if c == '|' {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    commands.push(trimmed.to_string());
                }
                current.clear();
                i += 1;
                continue;
            }
        }

        current.push(c);
        i += 1;
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        commands.push(trimmed.to_string());
    }

    commands
}

/// Evaluate compound shell command line fail-closed against permission policy.
pub fn evaluate_compound_shell_command(
    policy: &ResolvedPermissionPolicy,
    command: &str,
) -> PermissionAction {
    let subcommands = split_shell_commands(command);
    if subcommands.is_empty() {
        return policy.evaluate("bash", Some(command));
    }

    let mut effective = PermissionAction::Allow;
    for sub in subcommands {
        let dec = policy.evaluate("bash", Some(&sub));
        effective = effective.join_restrictive(dec);
        if effective == PermissionAction::Deny {
            break;
        }
    }
    effective
}

/// Emit native approval JSON response to stdout.
pub fn emit_response(
    target: HookTarget,
    decision: PermissionAction,
    raw_command: Option<&str>,
) -> Result<()> {
    match target {
        HookTarget::Antigravity => {
            let response = match decision {
                PermissionAction::Allow => {
                    let cmd = raw_command.unwrap_or("");
                    let override_entry = format!("command({cmd})");
                    json!({
                        "decision": "auto_approve",
                        "reason": "rulette policy",
                        "permissionOverrides": [override_entry]
                    })
                }
                PermissionAction::Ask => json!({
                    "decision": "ask",
                    "reason": "rulette policy: requires approval"
                }),
                PermissionAction::Deny => json!({
                    "decision": "deny",
                    "reason": "rulette policy: denied by rule"
                }),
            };
            println!("{}", serde_json::to_string(&response)?);
            Ok(())
        }
        HookTarget::Cursor => match decision {
            PermissionAction::Allow => {
                let response = json!({ "permission": "allow" });
                println!("{}", serde_json::to_string(&response)?);
                Ok(())
            }
            PermissionAction::Ask => {
                let response = json!({ "permission": "ask" });
                println!("{}", serde_json::to_string(&response)?);
                Ok(())
            }
            PermissionAction::Deny => {
                let response = json!({ "permission": "deny" });
                println!("{}", serde_json::to_string(&response)?);
                bail!(HookDeniedExit(2));
            }
        },
        HookTarget::Claude | HookTarget::Codex => {
            let action_str = match decision {
                PermissionAction::Allow => "allow",
                PermissionAction::Ask => "ask",
                PermissionAction::Deny => "deny",
            };
            let response = json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": action_str
                }
            });
            println!("{}", serde_json::to_string(&response)?);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::permissions::ToolAccessRule;

    #[test]
    fn test_split_shell_commands() {
        let cmd = "git status && npm test";
        assert_eq!(split_shell_commands(cmd), vec!["git status", "npm test"]);

        let cmd = "echo 'hello && world' ; cargo check";
        assert_eq!(
            split_shell_commands(cmd),
            vec!["echo 'hello && world'", "cargo check"]
        );

        let cmd = "a || b | c && d ; e";
        assert_eq!(split_shell_commands(cmd), vec!["a", "b", "c", "d", "e"]);

        let cmd = r#"echo "quoted;semicolon" && ls"#;
        assert_eq!(
            split_shell_commands(cmd),
            vec![r#"echo "quoted;semicolon""#, "ls"]
        );

        let cmd = r"echo escaped\&operator && pwd";
        assert_eq!(
            split_shell_commands(cmd),
            vec![r"echo escaped\&operator", "pwd"]
        );
    }

    #[test]
    fn test_evaluate_compound_shell_fail_closed() {
        let policy = PermissionPolicy {
            default_mode: Some(PermissionAction::Ask),
            rules: vec![
                ToolAccessRule::new("bash", Some("git *".into()), PermissionAction::Allow),
                ToolAccessRule::new("bash", Some("npm test".into()), PermissionAction::Allow),
                ToolAccessRule::new("bash", Some("rm *".into()), PermissionAction::Deny),
                ToolAccessRule::new("bash", Some("git push *".into()), PermissionAction::Ask),
            ],
        };
        let resolved = resolve_permission_policy(&policy, "test", "pkg").unwrap();

        // All allowed -> Allow
        assert_eq!(
            evaluate_compound_shell_command(&resolved, "git status && npm test"),
            PermissionAction::Allow
        );

        // One Deny -> Deny
        assert_eq!(
            evaluate_compound_shell_command(&resolved, "git status && rm -rf /"),
            PermissionAction::Deny
        );

        // One Ask -> Ask
        assert_eq!(
            evaluate_compound_shell_command(&resolved, "git status && git push origin main"),
            PermissionAction::Ask
        );

        // Ask and Deny -> Deny
        assert_eq!(
            evaluate_compound_shell_command(&resolved, "git push origin main && rm -rf /"),
            PermissionAction::Deny
        );
    }

    #[test]
    fn test_detect_target_and_extract_antigravity() {
        let payload = json!({
            "toolCall": {
                "name": "run_command",
                "args": {
                    "CommandLine": "git status"
                }
            }
        });
        let target = detect_target_from_payload(&payload);
        assert_eq!(target, HookTarget::Antigravity);

        let (tool, arg, raw_cmd) = extract_invocation(&payload, target);
        assert_eq!(tool, "bash");
        assert_eq!(arg.as_deref(), Some("git status"));
        assert_eq!(raw_cmd.as_deref(), Some("git status"));
    }

    #[test]
    fn test_detect_target_and_extract_cursor() {
        let payload = json!({
            "command": "npm test"
        });
        let target = detect_target_from_payload(&payload);
        assert_eq!(target, HookTarget::Cursor);

        let (tool, arg, raw_cmd) = extract_invocation(&payload, target);
        assert_eq!(tool, "bash");
        assert_eq!(arg.as_deref(), Some("npm test"));
        assert_eq!(raw_cmd.as_deref(), Some("npm test"));
    }

    #[test]
    fn test_detect_target_and_extract_claude() {
        let payload = json!({
            "tool_name": "Bash",
            "tool_input": {
                "command": "cargo check"
            }
        });
        let target = detect_target_from_payload(&payload);
        assert_eq!(target, HookTarget::Claude);

        let (tool, arg, raw_cmd) = extract_invocation(&payload, target);
        assert_eq!(tool, "bash");
        assert_eq!(arg.as_deref(), Some("cargo check"));
        assert_eq!(raw_cmd.as_deref(), Some("cargo check"));
    }
}
