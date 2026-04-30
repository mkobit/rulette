use super::Emitter;
use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct CursorEmitter;

impl Emitter for CursorEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        tracing::debug!(
            "Emitting document with {} entities (strict={})",
            doc.entities.len(),
            strict
        );
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
                    content.push_str(&serde_yaml::to_string(&meta)?);
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
                    let yaml = serde_yaml::to_string(&meta)?;
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
