use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;

pub trait Emitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>>;
}

pub struct ClaudeEmitter;
pub struct CursorEmitter;
pub struct AgentSkillsEmitter;

impl Emitter for ClaudeEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut rules_output = String::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                Entity::Rule(rule) => {
                    rules_output.push_str(&rule.body);
                    rules_output.push_str("\n\n");
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    // Lossy conversion warning: Skills lose some metadata when converted to basic rules
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Skill to Claude Rule drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Claude Rule drops metadata",
                            skill.metadata.name
                        );
                    }
                    rules_output.push_str(&skill.body);
                    rules_output.push_str("\n\n");
                }
            }
        }

        let mut map = HashMap::new();
        if !rules_output.is_empty() {
            map.insert(
                PathBuf::from("CLAUDE.md"),
                rules_output.trim_end().to_string(),
            );
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Entity, McpServer, McpServerConfig, McpServerMetadata, Permissions, PermissionsMetadata,
    };
    use serde_json::json;

    #[test]
    fn test_claude_settings_emitter() {
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

        let doc = crate::RuletteDocument {
            entities: vec![mcp, perms],
        };

        let map = ClaudeSettingsEmitter.emit(&doc, false).unwrap();
        assert_eq!(map.len(), 1);

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
    }
}

impl Emitter for CursorEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut map = HashMap::new();
        for (i, entity) in doc.entities.iter().enumerate() {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                Entity::Rule(rule) => {
                    let mut content = String::new();
                    content.push_str("---\n");
                    #[derive(serde::Serialize)]
                    struct CursorRuleMeta<'a> {
                        #[serde(skip_serializing_if = "Option::is_none")]
                        description: Option<&'a String>,
                        #[serde(flatten)]
                        #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
                        extra: std::collections::HashMap<&'a String, &'a serde_json::Value>,
                    }
                    let extra: std::collections::HashMap<_, _> = rule
                        .metadata
                        .extra
                        .iter()
                        .filter(|(k, _)| k.as_str() != "name")
                        .collect();
                    let meta = CursorRuleMeta {
                        description: rule.metadata.description.as_ref(),
                        extra,
                    };
                    content.push_str(&serde_yaml::to_string(&meta).unwrap());
                    content.push_str("---\n");
                    content.push_str(&rule.body);

                    let name = if let Some(serde_json::Value::String(n)) =
                        rule.metadata.extra.get("name")
                    {
                        n.clone()
                    } else {
                        format!("rule_{}", i)
                    };
                    let path = PathBuf::from(format!("{}.mdc", name));
                    map.insert(path, content);
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    // Lossy conversion warning
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Skill to Cursor MDC drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Cursor MDC drops metadata",
                            skill.metadata.name
                        );
                    }
                    let mut content = String::new();
                    content.push_str("---\n");
                    #[derive(serde::Serialize)]
                    struct CursorSkillMeta<'a> {
                        description: &'a String,
                        #[serde(flatten)]
                        #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
                        extra: std::collections::HashMap<&'a String, &'a serde_json::Value>,
                    }
                    let extra: std::collections::HashMap<_, _> =
                        skill.metadata.extra.iter().collect();
                    let meta = CursorSkillMeta {
                        description: &skill.metadata.description,
                        extra,
                    };
                    let yaml = serde_yaml::to_string(&meta).unwrap();
                    content.push_str(&yaml);
                    content.push_str("---\n");
                    content.push_str(&skill.body);

                    let path = PathBuf::from(format!("{}.mdc", skill.metadata.name));
                    map.insert(path, content);
                }
            }
        }
        Ok(map)
    }
}

impl Emitter for AgentSkillsEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut map = HashMap::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    let mut content = String::new();
                    content.push_str("---\n");
                    content.push_str(&serde_yaml::to_string(&skill.metadata).unwrap());
                    content.push_str("---\n");
                    content.push_str(&skill.body);
                    map.insert(
                        PathBuf::from(format!("{}.skill.md", skill.metadata.name)),
                        content,
                    );
                }
                Entity::Rule(rule) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Rule to Skill requires default metadata generation"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: Rule to Skill requires default metadata generation");
                    }
                    let mut content = String::new();
                    content.push_str("---\n");
                    #[derive(serde::Serialize)]
                    struct AgentSkillRuleMeta<'a> {
                        name: &'a str,
                        description: &'a str,
                        #[serde(flatten)]
                        #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
                        extra: std::collections::HashMap<&'a String, &'a serde_json::Value>,
                    }
                    let name_val = if let Some(serde_json::Value::String(n)) =
                        rule.metadata.extra.get("name")
                    {
                        n.as_str()
                    } else {
                        "generated-skill"
                    };
                    let description = if let Some(desc) = &rule.metadata.description {
                        desc.as_str()
                    } else {
                        "Generated from rule"
                    };
                    let extra: std::collections::HashMap<_, _> = rule
                        .metadata
                        .extra
                        .iter()
                        .filter(|(k, _)| k.as_str() != "name")
                        .collect();
                    let meta = AgentSkillRuleMeta {
                        name: name_val,
                        description,
                        extra,
                    };
                    content.push_str(&serde_yaml::to_string(&meta).unwrap());
                    content.push_str("---\n");
                    content.push_str(&rule.body);
                    map.insert(PathBuf::from(format!("{}.skill.md", name_val)), content);
                }
            }
        }
        Ok(map)
    }
}

pub struct ClaudeSettingsEmitter;

impl Emitter for ClaudeSettingsEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut mcp_servers = HashMap::new();
        let mut hooks = HashMap::new();
        let mut extra = HashMap::new();

        for entity in &doc.entities {
            match entity {
                crate::Entity::Rule(_) | crate::Entity::Skill(_) | crate::Entity::Agent(_) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: Rule/Skill/Agent to ClaudeSettings drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: Rule/Skill/Agent to ClaudeSettings drops metadata");
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
                    for (k, v) in &hook.metadata.extra {
                        hooks.insert(k.clone(), v.clone());
                    }
                }
                Entity::Permissions(perms) => {
                    for (k, v) in &perms.metadata.extra {
                        extra.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        let mut map = HashMap::new();

        if mcp_servers.is_empty() && hooks.is_empty() && extra.is_empty() {
            return Ok(map);
        }

        #[derive(serde::Serialize)]
        struct ClaudeMcpConfig<'a> {
            command: &'a String,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            args: &'a Vec<String>,
            #[serde(skip_serializing_if = "HashMap::is_empty")]
            env: &'a HashMap<String, String>,
        }

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

        Ok(map)
    }
}

pub struct CopilotEmitter;
pub struct WindsurfEmitter;
pub struct GeminiEmitter;

impl Emitter for CopilotEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                Entity::Rule(rule) => {
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Skill to Copilot drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Copilot drops metadata",
                            skill.metadata.name
                        );
                    }
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        let mut map = HashMap::new();
        if !output.is_empty() {
            map.insert(
                PathBuf::from("copilot-instructions.md"),
                output.trim_end().to_string(),
            );
        }
        Ok(map)
    }
}

impl Emitter for WindsurfEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                Entity::Rule(rule) => {
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Skill to Windsurf drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Windsurf drops metadata",
                            skill.metadata.name
                        );
                    }
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        let mut map = HashMap::new();
        if !output.is_empty() {
            map.insert(
                PathBuf::from(".windsurfrules"),
                output.trim_end().to_string(),
            );
        }
        Ok(map)
    }
}

impl Emitter for GeminiEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut output = String::new();
        let mut map = HashMap::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(_) | crate::Entity::Permissions(_) => {}
                Entity::Agent(agent) => {
                    let mut extra = agent.metadata.extra.clone();
                    let kind = extra
                        .remove("kind")
                        .and_then(|v| v.as_str().map(String::from));
                    let mcp_servers = extra
                        .remove("mcpServers")
                        .and_then(|v| serde_json::from_value(v).ok());
                    let temperature = extra.remove("temperature").and_then(|v| v.as_f64());
                    let max_turns = extra
                        .remove("max_turns")
                        .and_then(|v| v.as_u64().map(|n| n as u32));
                    let timeout_mins = extra
                        .remove("timeout_mins")
                        .and_then(|v| v.as_u64().map(|n| n as u32));

                    if strict {
                        if agent.metadata.tool_access.is_some() {
                            return Err(anyhow::anyhow!("Lossy conversion: Agent '{}' has tool_access which Gemini does not support", agent.metadata.name));
                        }
                        if !extra.is_empty() {
                            return Err(anyhow::anyhow!("Lossy conversion: Agent '{}' has extra metadata that Gemini does not support", agent.metadata.name));
                        }
                    } else {
                        if agent.metadata.tool_access.is_some() {
                            eprintln!("Warning: Lossy conversion: Agent '{}' to Gemini drops tool_access metadata", agent.metadata.name);
                        }
                        if !extra.is_empty() {
                            eprintln!("Warning: Lossy conversion: Agent '{}' to Gemini drops extra metadata", agent.metadata.name);
                        }
                    }

                    let subagent_metadata = crate::gemini::GeminiSubAgentMetadata {
                        name: agent.metadata.name.clone(),
                        description: agent.metadata.description.clone().unwrap_or_default(),
                        kind,
                        tools: agent.metadata.agent_tools.clone(),
                        mcp_servers,
                        model: agent
                            .metadata
                            .models
                            .as_ref()
                            .and_then(|m| m.first().cloned()),
                        temperature,
                        max_turns,
                        timeout_mins,
                        extra,
                    };

                    let yaml = serde_yaml::to_string(&subagent_metadata)?;
                    let subagent_str = format!("---\n{}---\n\n{}", yaml, agent.body);
                    map.insert(
                        PathBuf::from(format!("{}.md", agent.metadata.name)),
                        subagent_str.trim_end().to_string(),
                    );
                }
                Entity::Rule(rule) => {
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Skill to Gemini drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Gemini drops metadata",
                            skill.metadata.name
                        );
                    }
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        if !output.is_empty() {
            map.insert(PathBuf::from("GEMINI.md"), output.trim_end().to_string());
        }
        Ok(map)
    }
}

pub struct CodexEmitter;
impl Emitter for CodexEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                Entity::Rule(rule) => {
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Skill to Codex drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Codex drops metadata",
                            skill.metadata.name
                        );
                    }
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        let mut map = HashMap::new();
        if !output.is_empty() {
            map.insert(PathBuf::from("AGENTS.md"), output.trim_end().to_string());
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests_gemini {
    use super::*;
    use crate::{Agent, AgentMetadata, Entity, RuletteDocument};
    use std::collections::HashMap;

    #[test]
    fn test_gemini_emitter_agent_roundtrip() {
        let mut extra = HashMap::new();
        extra.insert("kind".to_string(), serde_json::json!("local"));
        extra.insert("temperature".to_string(), serde_json::json!(0.5));

        let agent = Entity::Agent(Agent {
            metadata: AgentMetadata {
                name: "test-agent".to_string(),
                description: Some("A test agent".to_string()),
                tool_access: None,
                agent_tools: Some(vec!["grep".to_string()]),
                models: Some(vec!["gemini-pro".to_string()]),
                extra,
            },
            body: "You are a test agent.".to_string(),
        });

        let agent_inner = match &agent {
            Entity::Agent(a) => a.clone(),
            _ => panic!("Expected Agent"),
        };

        let doc = RuletteDocument {
            entities: vec![agent.clone()],
        };

        let emitter = GeminiEmitter;
        let emitted = emitter.emit(&doc, false).unwrap();

        let filename = PathBuf::from("test-agent.md");
        assert!(emitted.contains_key(&filename));

        let content = emitted.get(&filename).unwrap();

        // Use parse_gemini for the round trip back to Entity::Agent
        let parsed_doc = crate::frontend::parse(
            content,
            crate::cli::formats::InputFormat::Gemini,
            Some("test-agent"),
        )
        .unwrap();
        assert_eq!(parsed_doc.entities.len(), 1);

        let parsed_agent = match &parsed_doc.entities[0] {
            Entity::Agent(a) => a,
            _ => panic!("Expected Agent entity"),
        };

        // Assert structural equality with the original agent
        assert_eq!(parsed_agent.metadata.name, agent_inner.metadata.name);
        assert_eq!(
            parsed_agent.metadata.description,
            agent_inner.metadata.description
        );
        assert_eq!(
            parsed_agent.metadata.agent_tools,
            agent_inner.metadata.agent_tools
        );
        assert_eq!(parsed_agent.metadata.models, agent_inner.metadata.models);

        // Assert the extra map correctly restored `kind` and `temperature` fields
        assert_eq!(
            parsed_agent.metadata.extra.get("kind"),
            agent_inner.metadata.extra.get("kind")
        );
        assert_eq!(
            parsed_agent
                .metadata
                .extra
                .get("temperature")
                .unwrap()
                .as_f64()
                .unwrap(),
            agent_inner
                .metadata
                .extra
                .get("temperature")
                .unwrap()
                .as_f64()
                .unwrap()
        );
        assert_eq!(parsed_agent.body, agent_inner.body);
    }
}
