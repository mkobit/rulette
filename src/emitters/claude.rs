use super::Emitter;
use crate::{Entity, HookEventKind, RuletteDocument};
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ClaudeEmitter;

impl Emitter for ClaudeEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        tracing::debug!(
            "Emitting document with {} entities (strict={})",
            doc.entities.len(),
            strict
        );
        let mut map = HashMap::new();

        // 1. Process Rule/Skill/Agent into CLAUDE.md or commands/*.md
        let mut rules_output = String::new();
        let mut mcp_servers = HashMap::new();
        let mut hooks = HashMap::new();
        let mut extra = HashMap::new();

        for entity in &doc.entities {
            match entity {
                Entity::Rule(rule) => {
                    rules_output.push_str(&rule.body);
                    rules_output.push_str("\n\n");
                }
                Entity::Skill(skill) => {
                    // Emit each skill as its own file
                    map.insert(
                        PathBuf::from(format!("{}.md", skill.metadata.name)),
                        skill.body.clone(),
                    );
                }
                Entity::Agent(_) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: Agent to Claude format drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Agent to Claude format drops metadata"
                        );
                    }
                }
                Entity::McpServer(mcp) => {
                    mcp_servers.insert(
                        mcp.metadata.name.clone(),
                        ClaudeMcpConfig {
                            command: &mcp.config.command,
                            args: &mcp.config.args,
                            env: &mcp.config.env,
                        },
                    );
                }
                Entity::Hook(hook) => {
                    // If we have a semantic event, we can reconstruct the Claude structure
                    if let Some(event) = &hook.metadata.hook_event {
                        let name = match event.event {
                            HookEventKind::PreToolUse => "PreToolUse",
                            HookEventKind::PostToolUse => "PostToolUse",
                            HookEventKind::Notification => "Notification",
                            HookEventKind::Stop => "Stop",
                            HookEventKind::SubagentStop => "SubagentStop",
                        };

                        if let Some(cmd) = &event.command {
                            let hook_val = json!([
                                {
                                    "hooks": [
                                        {
                                            "type": "command",
                                            "command": cmd
                                        }
                                    ]
                                }
                            ]);
                            hooks.insert(name.to_string(), hook_val);
                        } else {
                            // Fallback to extra if no command but has event
                            for (k, v) in &hook.metadata.extra {
                                hooks.insert(k.clone(), v.clone());
                            }
                        }
                    } else {
                        // Passthrough for hooks without semantic mapping
                        for (k, v) in &hook.metadata.extra {
                            hooks.insert(k.clone(), v.clone());
                        }
                    }
                }
                Entity::Permissions(perms) => {
                    for (k, v) in &perms.metadata.extra {
                        extra.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        if !rules_output.is_empty() {
            map.insert(
                PathBuf::from("CLAUDE.md"),
                rules_output.trim_end().to_string(),
            );
        }

        if !mcp_servers.is_empty() || !hooks.is_empty() || !extra.is_empty() {
            #[derive(serde::Serialize)]
            #[serde(rename_all = "camelCase")]
            struct ClaudeSettingsFile<'a> {
                #[serde(skip_serializing_if = "HashMap::is_empty")]
                mcp_servers: HashMap<String, ClaudeMcpConfig<'a>>,
                #[serde(skip_serializing_if = "HashMap::is_empty")]
                hooks: HashMap<String, serde_json::Value>,
                #[serde(flatten)]
                extra: HashMap<String, serde_json::Value>,
            }

            let settings = ClaudeSettingsFile {
                mcp_servers,
                hooks,
                extra,
            };

            let content = serde_json::to_string_pretty(&settings)?;
            map.insert(PathBuf::from("settings.json"), content);
        }

        Ok(map)
    }
}

#[derive(serde::Serialize)]
struct ClaudeMcpConfig<'a> {
    command: &'a String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: &'a Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    env: &'a HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Entity, McpServer, McpServerConfig, McpServerMetadata, Permissions, PermissionsMetadata,
    };
    use serde_json::json;

    #[test]
    fn test_claude_settings_and_rules_emitter() {
        let mcp = Entity::McpServer(McpServer {
            metadata: McpServerMetadata {
                name: "test-server".to_string(),
                extra: HashMap::new(),
            },
            config: McpServerConfig {
                command: "echo".to_string(),
                args: vec!["hello".to_string()],
                env: HashMap::new(),
            },
        });

        let mut extra_perms = HashMap::new();
        extra_perms.insert(
            "permissions".to_string(),
            json!({
                "ask": ["Bash"]
            }),
        );
        let perms = Entity::Permissions(Permissions {
            metadata: PermissionsMetadata {
                name: None,
                tool_access: None,
                settings_overrides: None,
                extra: extra_perms,
            },
        });

        let rule = Entity::Rule(crate::Rule {
            metadata: crate::RuleMetadata::default(),
            body: "Be helpful.".to_string(),
        });

        let doc = crate::RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![mcp, perms, rule],
        };

        let map = ClaudeEmitter.emit(&doc, false).unwrap();
        assert_eq!(map.len(), 2);

        let content = map.get(&PathBuf::from("settings.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content).unwrap();

        let mcp_servers = parsed.get("mcpServers").unwrap().as_object().unwrap();
        assert!(mcp_servers.contains_key("test-server"));
        let test_server = mcp_servers.get("test-server").unwrap().as_object().unwrap();
        assert_eq!(
            test_server.get("command").unwrap().as_str().unwrap(),
            "echo"
        );
        assert_eq!(
            test_server.get("args").unwrap().as_array().unwrap()[0]
                .as_str()
                .unwrap(),
            "hello"
        );

        let perms_val = parsed.get("permissions").unwrap().as_object().unwrap();
        assert!(perms_val.contains_key("ask"));

        let rule_content = map.get(&PathBuf::from("CLAUDE.md")).unwrap();
        assert_eq!(rule_content, "Be helpful.");
    }
}
