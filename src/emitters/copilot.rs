use super::{CapabilityEntry, CoverageStatus, Emitter};
use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub struct CopilotEmitter;

impl Emitter for CopilotEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        tracing::debug!(
            "Emitting document with {} entities (strict={})",
            doc.entities.len(),
            strict
        );
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(hook) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Hook to Copilot drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Hook '{}' to Copilot drops metadata",
                            hook.metadata.name
                        );
                    }
                }
                crate::Entity::Agent(agent) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Agent to Copilot drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Agent '{}' to Copilot drops metadata",
                            agent.metadata.name
                        );
                    }
                }
                crate::Entity::Permissions(perms) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Permissions to Copilot drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Permissions '{}' to Copilot drops metadata",
                            perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                        );
                    }
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

    fn capabilities(&self, doc: &RuletteDocument) -> Vec<CapabilityEntry> {
        let raw: Vec<CapabilityEntry> = doc
            .entities
            .iter()
            .map(|entity| match entity {
                Entity::Hook(hook) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Hook '{}' to Copilot drops metadata",
                        hook.metadata.name
                    ),
                ),
                Entity::Agent(agent) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Agent '{}' to Copilot drops metadata",
                        agent.metadata.name
                    ),
                ),
                Entity::Permissions(perms) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Permissions '{}' to Copilot drops metadata",
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
                Entity::Skill(skill) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Lossy,
                    format!(
                        "Lossy conversion: Skill '{}' to Copilot drops metadata",
                        skill.metadata.name
                    ),
                ),
                Entity::Rule(_) => CapabilityEntry::supported(entity),
            })
            .collect();
        super::aggregate_capabilities(raw)
    }
}
