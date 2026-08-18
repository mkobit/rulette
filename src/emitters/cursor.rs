use super::{CapabilityEntry, CoverageStatus, Emitter};
use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub struct CursorEmitter;

/// Inverse of `frontend::activation_from_cursor`: resolves the typed
/// activation model back to Cursor's own `alwaysApply`/`globs` pair.
fn cursor_fields_from_activation(activation: &crate::Activation) -> (Option<bool>, Option<String>) {
    use crate::ActivationMode;
    if activation.mode.contains(&ActivationMode::Always) {
        let globs = activation.globs.as_ref().map(|g| g.join(","));
        (Some(true), globs)
    } else if activation.mode.contains(&ActivationMode::Glob) {
        let globs = activation.globs.as_ref().map(|g| g.join(","));
        (Some(false), globs)
    } else {
        (Some(false), None)
    }
}

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
                crate::Entity::Hook(hook) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Hook to Cursor MDC drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Hook '{}' to Cursor MDC drops metadata",
                            hook.metadata.name
                        );
                    }
                }
                crate::Entity::Agent(agent) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Agent to Cursor MDC drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Agent '{}' to Cursor MDC drops metadata",
                            agent.metadata.name
                        );
                    }
                }
                crate::Entity::Permissions(perms) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Permissions to Cursor MDC drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Permissions '{}' to Cursor MDC drops metadata",
                            perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                        );
                    }
                }
                Entity::Rule(rule) => {
                    let mut content = String::new();
                    content.push_str("---\n");
                    #[derive(serde::Serialize)]
                    struct CursorRuleMeta<'a> {
                        #[serde(skip_serializing_if = "Option::is_none")]
                        description: Option<&'a String>,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        globs: Option<String>,
                        #[serde(rename = "alwaysApply", skip_serializing_if = "Option::is_none")]
                        always_apply: Option<bool>,
                        #[serde(flatten)]
                        #[serde(skip_serializing_if = "HashMap::is_empty")]
                        extra: HashMap<&'a String, &'a serde_json::Value>,
                    }
                    let extra: HashMap<_, _> = rule
                        .metadata
                        .extra
                        .iter()
                        .filter(|(k, _)| k.as_str() != "name" && !super::is_internal_extra_key(k))
                        .collect();
                    let (always_apply, globs) = rule
                        .metadata
                        .activation
                        .as_ref()
                        .map(cursor_fields_from_activation)
                        .unwrap_or((None, None));
                    let meta = CursorRuleMeta {
                        description: rule.metadata.description.as_ref(),
                        globs,
                        always_apply,
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
                        #[serde(skip_serializing_if = "HashMap::is_empty")]
                        extra: HashMap<&'a String, &'a serde_json::Value>,
                    }
                    let extra: HashMap<_, _> = skill
                        .metadata
                        .extra
                        .iter()
                        .filter(|(k, _)| !super::is_internal_extra_key(k))
                        .collect();
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

    fn capabilities(&self, doc: &RuletteDocument) -> Vec<CapabilityEntry> {
        let raw: Vec<CapabilityEntry> = doc
            .entities
            .iter()
            .map(|entity| match entity {
                Entity::Hook(hook) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Hook '{}' to Cursor MDC drops metadata",
                        hook.metadata.name
                    ),
                ),
                Entity::Agent(agent) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Agent '{}' to Cursor MDC drops metadata",
                        agent.metadata.name
                    ),
                ),
                Entity::Permissions(perms) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Permissions '{}' to Cursor MDC drops metadata",
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
                        "Lossy conversion: Skill '{}' to Cursor MDC drops metadata",
                        skill.metadata.name
                    ),
                ),
                Entity::Rule(_) => CapabilityEntry::supported(entity),
            })
            .collect();
        super::aggregate_capabilities(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Activation, ActivationMode, Rule, RuleMetadata};

    fn rule_with_activation(activation: Activation) -> Entity {
        Entity::Rule(Rule {
            metadata: RuleMetadata {
                description: Some("test rule".to_string()),
                activation: Some(activation),
                extra: HashMap::new(),
            },
            body: "Body.".to_string(),
        })
    }

    #[test]
    fn emits_always_apply_true_for_always_mode() {
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![rule_with_activation(Activation {
                mode: vec![ActivationMode::Always],
                globs: None,
                pattern: None,
                description: None,
            })],
        };
        let output = CursorEmitter.emit(&doc, false).unwrap();
        let content = output.values().next().unwrap();
        assert!(content.contains("alwaysApply: true"));
        assert!(!content.contains("globs:"));
    }

    #[test]
    fn emits_comma_joined_globs_for_glob_mode() {
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![rule_with_activation(Activation {
                mode: vec![ActivationMode::Glob],
                globs: Some(vec!["src/**/*.ts".to_string(), "src/**/*.tsx".to_string()]),
                pattern: None,
                description: None,
            })],
        };
        let output = CursorEmitter.emit(&doc, false).unwrap();
        let content = output.values().next().unwrap();
        assert!(content.contains("alwaysApply: false"));
        assert!(content.contains("src/**/*.ts,src/**/*.tsx"));
    }

    #[test]
    fn omits_activation_fields_when_none() {
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![Entity::Rule(Rule {
                metadata: RuleMetadata {
                    description: Some("test rule".to_string()),
                    activation: None,
                    extra: HashMap::new(),
                },
                body: "Body.".to_string(),
            })],
        };
        let output = CursorEmitter.emit(&doc, false).unwrap();
        let content = output.values().next().unwrap();
        assert!(!content.contains("alwaysApply"));
        assert!(!content.contains("globs"));
    }
}
