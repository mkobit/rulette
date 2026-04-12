use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};

pub trait Emitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<String>;
}

pub struct ClaudeEmitter;
pub struct CursorEmitter;
pub struct AgentSkillsEmitter;
pub struct CodexEmitter;

impl Emitter for ClaudeEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<String> {
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                Entity::Rule(rule) => {
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::Skill(skill) => {
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
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        Ok(output.trim_end().to_string())
    }
}

impl Emitter for CursorEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<String> {
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                Entity::Rule(rule) => {
                    output.push_str("---\n");
                    if let Some(desc) = &rule.metadata.description {
                        output.push_str(&format!("description: {}\n", desc));
                    }
                    for (k, v) in &rule.metadata.extra {
                        if !k.starts_with("rulette_override") {
                            if let serde_json::Value::String(s) = v {
                                output.push_str(&format!("{}: {}\n", k, s));
                            }
                        }
                    }
                    output.push_str("---\n");
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::Skill(skill) => {
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
                    output.push_str("---\n");
                    output.push_str(&format!("description: {}\n", skill.metadata.description));
                    for (k, v) in &skill.metadata.extra {
                        if let serde_json::Value::String(s) = v {
                            output.push_str(&format!("{}: {}\n", k, s));
                        }
                    }
                    output.push_str("---\n");
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        Ok(output.trim_end().to_string())
    }
}

impl Emitter for AgentSkillsEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<String> {
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                Entity::Skill(skill) => {
                    output.push_str("---\n");
                    output.push_str(&format!("name: {}\n", skill.metadata.name));
                    output.push_str(&format!("description: {}\n", skill.metadata.description));
                    if let Some(v) = &skill.metadata.version {
                        output.push_str(&format!("version: {}\n", v));
                    }
                    if let Some(l) = &skill.metadata.license {
                        output.push_str(&format!("license: {}\n", l));
                    }
                    if let Some(c) = &skill.metadata.compatibility {
                        output.push_str(&format!("compatibility: {}\n", c));
                    }
                    if let Some(a) = &skill.metadata.allowed_tools {
                        output.push_str(&format!("allowed-tools: {}\n", a));
                    }
                    for (k, v) in &skill.metadata.extra {
                        if let serde_json::Value::String(s) = v {
                            output.push_str(&format!("{}: {}\n", k, s));
                        }
                    }
                    output.push_str("---\n");
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
                Entity::Rule(rule) => {
                    let mut name = "generated-skill".to_string();
                    let mut description = rule
                        .metadata
                        .description
                        .clone()
                        .unwrap_or_else(|| "Generated from rule".to_string());

                    let has_name_override =
                        rule.metadata.extra.contains_key("rulette_override_name");
                    let has_desc_override = rule
                        .metadata
                        .extra
                        .contains_key("rulette_override_description");

                    if has_name_override {
                        if let Some(serde_json::Value::String(s)) =
                            rule.metadata.extra.get("rulette_override_name")
                        {
                            name = s.clone();
                        }
                    }
                    if has_desc_override {
                        if let Some(serde_json::Value::String(s)) =
                            rule.metadata.extra.get("rulette_override_description")
                        {
                            description = s.clone();
                        }
                    }

                    if strict && (!has_name_override || !has_desc_override) {
                        return Err(anyhow!("Lossy conversion: Rule to Skill requires default metadata generation (or --name/--description overrides)"));
                    } else if !has_name_override || !has_desc_override {
                        eprintln!("Warning: Lossy conversion: Rule to Skill requires default metadata generation. Use --name and --description to avoid this warning.");
                    }

                    output.push_str("---\n");
                    output.push_str(&format!("name: {}\n", name));
                    output.push_str(&format!("description: {}\n", description));

                    for (k, v) in &rule.metadata.extra {
                        if !k.starts_with("rulette_override") {
                            if let serde_json::Value::String(s) = v {
                                output.push_str(&format!("{}: {}\n", k, s));
                            }
                        }
                    }
                    output.push_str("---\n");
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
            }
        }
        Ok(output.trim_end().to_string())
    }
}

impl Emitter for CodexEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<String> {
        let mut output = String::new();
        for entity in &doc.entities {
            match entity {
                Entity::Rule(rule) => {
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::Skill(skill) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Skill to Codex Rule drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Codex Rule drops metadata",
                            skill.metadata.name
                        );
                    }
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        Ok(output.trim_end().to_string())
    }
}
