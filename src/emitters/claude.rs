use super::{CapabilityEntry, CoverageStatus, Emitter};
use crate::{Entity, HookEventKind, RuletteDocument};
use anyhow::Result;
use serde_json::json;
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub struct ClaudeEmitter;

/// Agent sub-agent definitions are unconditionally dropped: Claude Code
/// supports them natively, but `ClaudeEmitter` doesn't implement that yet, so
/// nothing is ever written for an `Agent` entity here.
fn classify_agent() -> &'static str {
    "Lossy conversion: Agent to Claude format drops metadata"
}

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
                    skill.metadata.validate()?;
                    let mut content = String::new();
                    content.push_str("---\n");
                    let mut metadata_for_output = skill.metadata.clone();
                    metadata_for_output
                        .extra
                        .retain(|k, _| !super::is_internal_extra_key(k));
                    content.push_str(&serde_yaml::to_string(&metadata_for_output)?);
                    content.push_str("---\n");
                    content.push_str(&skill.body);
                    map.insert(
                        PathBuf::from(format!("{}/SKILL.md", skill.metadata.name)),
                        content,
                    );
                }
                Entity::Agent(_) => {
                    let reason = classify_agent();
                    if strict {
                        return Err(anyhow::anyhow!("{reason}"));
                    } else {
                        eprintln!("Warning: {reason}");
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
                                if !super::is_internal_extra_key(k) {
                                    hooks.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    } else {
                        // Passthrough for hooks without semantic mapping
                        for (k, v) in &hook.metadata.extra {
                            if !super::is_internal_extra_key(k) {
                                hooks.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
                Entity::Permissions(perms) => {
                    for (k, v) in &perms.metadata.extra {
                        if !super::is_internal_extra_key(k) {
                            extra.insert(k.clone(), v.clone());
                        }
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

    fn capabilities(&self, doc: &RuletteDocument) -> Vec<CapabilityEntry> {
        let raw: Vec<CapabilityEntry> = doc
            .entities
            .iter()
            .map(|entity| match entity {
                Entity::Agent(_) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    classify_agent(),
                ),
                Entity::Rule(_)
                | Entity::Skill(_)
                | Entity::McpServer(_)
                | Entity::Hook(_)
                | Entity::Permissions(_) => CapabilityEntry::supported(entity),
            })
            .collect();
        super::aggregate_capabilities(raw)
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

    #[test]
    fn test_skill_emission_preserves_frontmatter_and_writes_skill_md() {
        use crate::agent_skills::{Skill, SkillMetadata};

        let skill = Entity::Skill(Skill {
            metadata: SkillMetadata {
                name: "example-skill".to_string(),
                description: "An example skill".to_string(),
                version: Some("1.0.0".to_string()),
                license: Some("MIT".to_string()),
                compatibility: None,
                metadata: HashMap::new(),
                allowed_tools: None,
                extra: HashMap::new(),
            },
            body: "# Example Skill\n\nContent.".to_string(),
        });
        let doc = crate::RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![skill],
        };

        let map = ClaudeEmitter.emit(&doc, false).unwrap();
        let content = map
            .get(&PathBuf::from("example-skill/SKILL.md"))
            .expect("expected output at <name>/SKILL.md, matching the format's namesake file");

        assert!(content.contains("name: example-skill"));
        assert!(content.contains("description: An example skill"));
        assert!(content.contains("version: 1.0.0"));
        assert!(content.contains("license: MIT"));
        assert!(content.contains("# Example Skill\n\nContent."));
    }

    #[test]
    fn test_skill_emission_drops_internal_source_file_key() {
        use crate::agent_skills::{Skill, SkillMetadata};

        let mut extra = HashMap::new();
        extra.insert(
            "rulette:source_file".to_string(),
            serde_json::Value::String("some/path.md".to_string()),
        );
        let skill = Entity::Skill(Skill {
            metadata: SkillMetadata {
                name: "example-skill".to_string(),
                description: "An example skill".to_string(),
                version: None,
                license: None,
                compatibility: None,
                metadata: HashMap::new(),
                allowed_tools: None,
                extra,
            },
            body: "Content.".to_string(),
        });
        let doc = crate::RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![skill],
        };

        let map = ClaudeEmitter.emit(&doc, false).unwrap();
        let content = map.get(&PathBuf::from("example-skill/SKILL.md")).unwrap();
        assert!(!content.contains("rulette:source_file"));
    }
}
