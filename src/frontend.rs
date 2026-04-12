use crate::agent_skills::{Skill, SkillMetadata};
use crate::cli::formats::InputFormat;
use crate::{Entity, Rule, RuleMetadata, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub fn parse(input: &str, format: InputFormat) -> Result<RuletteDocument> {
    let entities = match format {
        InputFormat::Auto => {
            if input.starts_with("---\n") {
                if input.contains("name:") && input.contains("description:") {
                    vec![Entity::Skill(parse_agent_skills(input)?)]
                } else {
                    vec![Entity::Rule(parse_cursor_mdc(input)?)]
                }
            } else {
                // Heuristic: If it looks like AGENTS.md format, parse it as Codex
                if input.contains("<context>") || input.contains("You are ") {
                    vec![Entity::Rule(parse_codex(input)?)]
                } else {
                    vec![Entity::Rule(parse_claude(input)?)]
                }
            }
        }
        InputFormat::SkillMd | InputFormat::AgentSkills => {
            vec![Entity::Skill(parse_agent_skills(input)?)]
        }
        InputFormat::CursorMdc => vec![Entity::Rule(parse_cursor_mdc(input)?)],
        InputFormat::Claude => vec![Entity::Rule(parse_claude(input)?)],
        InputFormat::Codex => vec![Entity::Rule(parse_codex(input)?)],
        _ => return Err(anyhow!("Unsupported input format for parsing")),
    };

    Ok(RuletteDocument { entities })
}

fn parse_agent_skills(input: &str) -> Result<Skill> {
    let (frontmatter, body) = extract_frontmatter(input);
    let mut metadata = SkillMetadata {
        name: "unnamed-skill".to_string(),
        description: "No description provided".to_string(),
        version: None,
        license: None,
        compatibility: None,
        metadata: HashMap::new(),
        allowed_tools: None,
        extra: HashMap::new(),
    };

    if let Some(fm) = frontmatter {
        for line in fm.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim();
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                match k {
                    "name" => metadata.name = v.to_string(),
                    "description" => metadata.description = v.to_string(),
                    "version" => metadata.version = Some(v.to_string()),
                    "license" => metadata.license = Some(v.to_string()),
                    "compatibility" => metadata.compatibility = Some(v.to_string()),
                    "allowed-tools" => metadata.allowed_tools = Some(v.to_string()),
                    _ => {
                        metadata
                            .extra
                            .insert(k.to_string(), serde_json::Value::String(v.to_string()));
                    }
                }
            }
        }
    }

    Ok(Skill {
        metadata,
        body: body.to_string(),
    })
}

fn parse_cursor_mdc(input: &str) -> Result<Rule> {
    let (frontmatter, body) = extract_frontmatter(input);
    let mut metadata = RuleMetadata::default();

    if let Some(fm) = frontmatter {
        for line in fm.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim();
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                match k {
                    "description" => metadata.description = Some(v.to_string()),
                    _ => {
                        metadata
                            .extra
                            .insert(k.to_string(), serde_json::Value::String(v.to_string()));
                    }
                }
            }
        }
    }

    Ok(Rule {
        metadata,
        body: body.to_string(),
    })
}

fn parse_claude(input: &str) -> Result<Rule> {
    Ok(Rule {
        metadata: RuleMetadata::default(),
        body: input.to_string(),
    })
}

fn parse_codex(input: &str) -> Result<Rule> {
    // AGENTS.md doesn't have a strict frontmatter format, so we parse it into a plain Rule
    Ok(Rule {
        metadata: RuleMetadata::default(),
        body: input.to_string(),
    })
}

fn extract_frontmatter(input: &str) -> (Option<&str>, &str) {
    if input.starts_with("---\n") || input.starts_with("---\r\n") {
        if let Some(end_idx) = input[4..].find("\n---") {
            let frontmatter = &input[4..4 + end_idx];
            let rest_idx = 4 + end_idx + 4;
            let body = if input.len() > rest_idx && input[rest_idx..].starts_with('\n') {
                &input[rest_idx + 1..]
            } else if input.len() > rest_idx + 1 && input[rest_idx..].starts_with("\r\n") {
                &input[rest_idx + 2..]
            } else {
                &input[rest_idx..]
            };
            return (Some(frontmatter), body);
        }
    }
    (None, input)
}
