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
                    #[derive(serde::Serialize)]
                    struct CursorRuleMeta<'a> {
                        #[serde(skip_serializing_if = "Option::is_none")]
                        description: Option<&'a String>,
                        #[serde(flatten)]
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
                    output.push_str(&serde_yaml::to_string(&meta).unwrap());
                    output.push_str("---\n");
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
                    #[derive(serde::Serialize)]
                    struct CursorSkillMeta<'a> {
                        description: &'a String,
                        #[serde(flatten)]
                        extra: &'a std::collections::HashMap<String, serde_json::Value>,
                    }
                    let meta = CursorSkillMeta {
                        description: &skill.metadata.description,
                        extra: &skill.metadata.extra,
                    };
                    let yaml = serde_yaml::to_string(&meta).unwrap();
                    // serde_yaml outputs `extra: {}` when flattened field is empty instead of omitting it. Let's fix this for simple empty extra.
                    let yaml = yaml.replace(
                        "
{}", "",
                    );
                    output.push_str(&yaml);
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
                    output.push_str("---\n");
                    output.push_str(&serde_yaml::to_string(&skill.metadata).unwrap());
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
                    #[derive(serde::Serialize)]
                    struct AgentSkillRuleMeta<'a> {
                        name: &'a str,
                        description: &'a str,
                        #[serde(flatten)]
                        extra: std::collections::HashMap<&'a String, &'a serde_json::Value>,
                    }
                    let name = if let Some(serde_json::Value::String(n)) =
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
                        name,
                        description,
                        extra,
                    };
                    output.push_str(&serde_yaml::to_string(&meta).unwrap());
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
        Ok(output.trim_end().to_string())
    }
}
