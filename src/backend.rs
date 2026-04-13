use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};

pub trait Emitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<String>;
}

pub struct ClaudeEmitter;
pub struct CursorEmitter;
pub struct AgentSkillsEmitter;

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
                        if k == "name" {
                            continue;
                        }
                        if let serde_json::Value::String(s) = v {
                            output.push_str(&format!("{}: {}\n", k, s));
                        }
                    }
                    output.push_str("---\n");
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
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
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Rule to Skill requires default metadata generation"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: Rule to Skill requires default metadata generation");
                    }
                    output.push_str("---\n");
                    if let Some(serde_json::Value::String(name)) = rule.metadata.extra.get("name") {
                        output.push_str(&format!("name: {}\n", name));
                    } else {
                        output.push_str("name: generated-skill\n");
                    }
                    if let Some(desc) = &rule.metadata.description {
                        output.push_str(&format!("description: {}\n", desc));
                    } else {
                        output.push_str("description: Generated from rule\n");
                    }
                    for (k, v) in &rule.metadata.extra {
                        if k == "name" {
                            continue;
                        }
                        if let serde_json::Value::String(s) = v {
                            output.push_str(&format!("{}: {}\n", k, s));
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

pub struct CopilotEmitter;
pub struct WindsurfEmitter;
pub struct GeminiEmitter;

impl Emitter for CopilotEmitter {
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
        Ok(output.trim_end().to_string())
    }
}

impl Emitter for WindsurfEmitter {
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
        Ok(output.trim_end().to_string())
    }
}

impl Emitter for GeminiEmitter {
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
        Ok(output.trim_end().to_string())
    }
}

pub struct CodexEmitter;
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
        Ok(output.trim_end().to_string())
    }
}
