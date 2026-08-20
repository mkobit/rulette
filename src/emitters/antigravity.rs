use super::{CapabilityEntry, CoverageStatus, Emitter};
use crate::parsers::antigravity::AntigravityTrigger;
use crate::{ActivationMode, Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub struct AntigravityEmitter;

/// Resolves the typed activation model to Antigravity's trigger and globs pair.
fn antigravity_fields_from_activation(
    activation: &crate::Activation,
) -> (Option<AntigravityTrigger>, Option<Vec<String>>) {
    if activation.mode.contains(&ActivationMode::Always) {
        (Some(AntigravityTrigger::AlwaysOn), activation.globs.clone())
    } else if activation.mode.contains(&ActivationMode::Glob) {
        (Some(AntigravityTrigger::Glob), activation.globs.clone())
    } else if activation.mode.contains(&ActivationMode::Model) {
        (Some(AntigravityTrigger::ModelDecision), None)
    } else if activation.mode.contains(&ActivationMode::Manual) {
        (Some(AntigravityTrigger::Manual), None)
    } else if activation.mode.contains(&ActivationMode::Pattern) {
        (Some(AntigravityTrigger::Glob), activation.globs.clone())
    } else {
        (None, None)
    }
}

impl Emitter for AntigravityEmitter {
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
                            "Lossy conversion: Hook to Antigravity drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Hook '{}' to Antigravity drops metadata",
                            hook.metadata.name
                        );
                    }
                }
                crate::Entity::Agent(agent) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Agent to Antigravity drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Agent '{}' to Antigravity drops metadata",
                            agent.metadata.name
                        );
                    }
                }
                crate::Entity::Permissions(perms) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Permissions to Antigravity drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Permissions '{}' to Antigravity drops metadata",
                            perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                        );
                    }
                }
                Entity::Rule(rule) => {
                    let mut content = String::new();
                    content.push_str("---\n");
                    #[derive(serde::Serialize)]
                    struct AntigravityRuleMeta<'a> {
                        #[serde(skip_serializing_if = "Option::is_none")]
                        description: Option<&'a String>,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        trigger: Option<AntigravityTrigger>,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        globs: Option<Vec<String>>,
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
                    let (trigger, globs) = rule
                        .metadata
                        .activation
                        .as_ref()
                        .map(|a| a.resolve("antigravity"))
                        .map(antigravity_fields_from_activation)
                        .unwrap_or((None, None));
                    let meta = AntigravityRuleMeta {
                        description: rule.metadata.description.as_ref(),
                        trigger,
                        globs,
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
                    let path = PathBuf::from(format!("{}.md", name));
                    map.insert(path, content);
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: McpServer '{}' to target format drops metadata",
                            mcp.metadata.name
                        );
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
                        PathBuf::from(format!("skills/{}/SKILL.md", skill.metadata.name)),
                        content,
                    );
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
                        "Lossy conversion: Hook '{}' to Antigravity drops metadata",
                        hook.metadata.name
                    ),
                ),
                Entity::Agent(agent) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Agent '{}' to Antigravity drops metadata",
                        agent.metadata.name
                    ),
                ),
                Entity::Permissions(perms) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Permissions '{}' to Antigravity drops metadata",
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
                activation: Some(activation.into()),
                extra: HashMap::new(),
            },
            body: "Rule body content.".to_string(),
        })
    }

    #[test]
    fn emits_always_on_for_always_mode() {
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![rule_with_activation(Activation {
                mode: vec![ActivationMode::Always],
                globs: None,
                pattern: None,
                description: None,
            })],
        };
        let emitted = AntigravityEmitter.emit(&doc, false).unwrap();
        let content = emitted.values().next().unwrap();
        assert!(content.contains("trigger: always_on"));
    }

    #[test]
    fn emits_glob_trigger_for_glob_mode() {
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![rule_with_activation(Activation {
                mode: vec![ActivationMode::Glob],
                globs: Some(vec!["**/*.ts".to_string(), "**/*.tsx".to_string()]),
                pattern: None,
                description: None,
            })],
        };
        let emitted = AntigravityEmitter.emit(&doc, false).unwrap();
        let content = emitted.values().next().unwrap();
        assert!(content.contains("trigger: glob"));
        assert!(content.contains("- '**/*.ts'") || content.contains("- \"**/*.ts\"") || content.contains("- **/*.ts"));
    }

    #[test]
    fn emits_model_decision_for_model_mode() {
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![rule_with_activation(Activation {
                mode: vec![ActivationMode::Model],
                globs: None,
                pattern: None,
                description: Some("Apply when editing Rust".to_string()),
            })],
        };
        let emitted = AntigravityEmitter.emit(&doc, false).unwrap();
        let content = emitted.values().next().unwrap();
        assert!(content.contains("trigger: model_decision"));
    }

    #[test]
    fn emits_manual_trigger_for_manual_mode() {
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![rule_with_activation(Activation {
                mode: vec![ActivationMode::Manual],
                globs: None,
                pattern: None,
                description: None,
            })],
        };
        let emitted = AntigravityEmitter.emit(&doc, false).unwrap();
        let content = emitted.values().next().unwrap();
        assert!(content.contains("trigger: manual"));
    }

    #[test]
    fn resolves_antigravity_target_override() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "antigravity".to_string(),
            Activation {
                mode: vec![ActivationMode::Model],
                globs: None,
                pattern: None,
                description: Some("Antigravity override description".to_string()),
            },
        );
        overrides.insert(
            "cursor-mdc".to_string(),
            Activation {
                mode: vec![ActivationMode::Always],
                globs: None,
                pattern: None,
                description: None,
            },
        );
        let wrapped = crate::TargetOverrides::Wrapped {
            default: Activation {
                mode: vec![ActivationMode::Glob],
                globs: Some(vec!["**/*.go".to_string()]),
                pattern: None,
                description: None,
            },
            overrides,
        };
        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![Entity::Rule(Rule {
                metadata: RuleMetadata {
                    description: Some("Override rule".to_string()),
                    activation: Some(wrapped),
                    extra: HashMap::new(),
                },
                body: "Body content.".to_string(),
            })],
        };
        let emitted = AntigravityEmitter.emit(&doc, false).unwrap();
        let content = emitted.values().next().unwrap();
        assert!(content.contains("trigger: model_decision"));
    }
}
