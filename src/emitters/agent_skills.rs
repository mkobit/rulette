use super::{CapabilityEntry, CoverageStatus, Emitter};
use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub struct AgentSkillsEmitter;

impl Emitter for AgentSkillsEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        tracing::debug!(
            "Emitting document with {} entities (strict={})",
            doc.entities.len(),
            strict
        );
        let mut map = HashMap::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(hook) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Hook to Agent Skills drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Hook '{}' to Agent Skills drops metadata",
                            hook.metadata.name
                        );
                    }
                }
                crate::Entity::Agent(agent) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Agent to Agent Skills drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Agent '{}' to Agent Skills drops metadata",
                            agent.metadata.name
                        );
                    }
                }
                crate::Entity::Permissions(perms) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Permissions to Agent Skills drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Permissions '{}' to Agent Skills drops metadata",
                            perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                        );
                    }
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
                        #[serde(skip_serializing_if = "HashMap::is_empty")]
                        extra: HashMap<&'a String, &'a serde_json::Value>,
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
                    let extra: HashMap<_, _> = rule
                        .metadata
                        .extra
                        .iter()
                        .filter(|(k, _)| k.as_str() != "name" && !super::is_internal_extra_key(k))
                        .collect();
                    let meta = AgentSkillRuleMeta {
                        name: name_val,
                        description,
                        extra,
                    };
                    content.push_str(&serde_yaml::to_string(&meta)?);
                    content.push_str("---\n");
                    content.push_str(&rule.body);
                    map.insert(PathBuf::from(format!("{}/SKILL.md", name_val)), content);
                }
            }
        }
        Ok(map)
    }

    fn capabilities(&self, doc: &RuletteDocument) -> Vec<CapabilityEntry> {
        let raw: Vec<CapabilityEntry> = doc
            .entities
            .iter()
            .map(|entity| match entity {
                Entity::Hook(hook) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Hook '{}' to Agent Skills drops metadata",
                        hook.metadata.name
                    ),
                ),
                Entity::Agent(agent) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Agent '{}' to Agent Skills drops metadata",
                        agent.metadata.name
                    ),
                ),
                Entity::Permissions(perms) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Permissions '{}' to Agent Skills drops metadata",
                        perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                    ),
                ),
                Entity::McpServer(mcp) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: McpServer '{}' to target format drops metadata",
                        mcp.metadata.name
                    ),
                ),
                Entity::Skill(_) => CapabilityEntry::supported(entity),
                Entity::Rule(_) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Lossy,
                    "Lossy conversion: Rule to Skill requires default metadata generation",
                ),
            })
            .collect();
        super::aggregate_capabilities(raw)
    }
}
