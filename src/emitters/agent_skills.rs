use super::Emitter;
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
                    skill.metadata.validate()?;
                    let mut content = String::new();
                    content.push_str("---\n");
                    content.push_str(&serde_yaml::to_string(&skill.metadata)?);
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
                        .filter(|(k, _)| k.as_str() != "name")
                        .collect();
                    let meta = AgentSkillRuleMeta {
                        name: name_val,
                        description,
                        extra,
                    };
                    content.push_str(&serde_yaml::to_string(&meta)?);
                    content.push_str("---\n");
                    content.push_str(&rule.body);
                    map.insert(PathBuf::from(format!("{}.skill.md", name_val)), content);
                }
            }
        }
        Ok(map)
    }
}
