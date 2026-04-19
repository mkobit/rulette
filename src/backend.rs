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
        let mut map = HashMap::new();
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
