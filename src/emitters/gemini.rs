use super::Emitter;
use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub struct GeminiEmitter;

impl Emitter for GeminiEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        tracing::debug!(
            "Emitting document with {} entities (strict={})",
            doc.entities.len(),
            strict
        );
        let mut output = String::new();
        let mut map = HashMap::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(hook) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: Hook to Gemini drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Hook '{}' to Gemini drops metadata",
                            hook.metadata.name
                        );
                    }
                }
                crate::Entity::Permissions(perms) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: Permissions to Gemini drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Permissions '{}' to Gemini drops metadata",
                            perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                        );
                    }
                }
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
                    extra.retain(|k, _| !super::is_internal_extra_key(k));

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

                    let subagent_metadata = crate::parsers::gemini::GeminiSubAgentMetadata {
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

#[cfg(test)]
mod tests_gemini {
    use super::*;
    use crate::{Agent, AgentMetadata, Entity, RuletteDocument};
    use std::collections::BTreeMap as HashMap;

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
            ir_version: "0.1".to_string(),
            entities: vec![agent.clone()],
        };

        let emitter = GeminiEmitter;
        let emitted = emitter.emit(&doc, false).unwrap();

        let filename = PathBuf::from("test-agent.md");
        assert!(emitted.contains_key(&filename));

        let content = emitted.get(&filename).unwrap();

        // Use parse_gemini for the round trip back to Entity::Agent
        let parsed_doc = crate::parsers::parse(
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
