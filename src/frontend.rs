use crate::agent_skills::{Skill, SkillMetadata};
use crate::cli::formats::InputFormat;
use crate::{Entity, Rule, RuleMetadata, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;

pub fn parse(input: &str, format: InputFormat, filename: Option<&str>) -> Result<RuletteDocument> {
    let entities = match format {
        InputFormat::Auto => {
            // Attempt basic detection based on input content
            // We can't rely on filename here since we get the string content directly.
            // But we'll try to guess based on frontmatter or tags.
            // If it has frontmatter, guess MDC/AgentSkills.
            if input.starts_with("---\n") {
                if input.contains("name:") && input.contains("description:") {
                    vec![Entity::Skill(parse_agent_skills(input, filename)?)]
                } else {
                    vec![Entity::Rule(parse_cursor_mdc(input, filename)?)]
                }
            } else {
                // Default to Claude rule if we can't tell, or just plain rule
                vec![Entity::Rule(parse_claude(input, filename)?)]
            }
        }
        InputFormat::SkillMd | InputFormat::AgentSkills => {
            vec![Entity::Skill(parse_agent_skills(input, filename)?)]
        }
        InputFormat::CursorMdc => vec![Entity::Rule(parse_cursor_mdc(input, filename)?)],
        InputFormat::Claude => vec![Entity::Rule(parse_claude(input, filename)?)],
        _ => return Err(anyhow!("Unsupported input format for parsing")),
    };

    Ok(RuletteDocument { entities })
}

fn parse_agent_skills(input: &str, filename: Option<&str>) -> Result<Skill> {
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

    if metadata.name == "unnamed-skill" {
        if let Some(name) = extract_name_from_filename(filename) {
            metadata.name = name;
        }
    }
    if metadata.description == "No description provided" {
        if let Some(desc) = extract_description_from_body(body) {
            metadata.description = desc;
        }
    }

    Ok(Skill {
        metadata,
        body: body.to_string(),
    })
}

fn parse_cursor_mdc(input: &str, filename: Option<&str>) -> Result<Rule> {
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

    if !metadata.extra.contains_key("name") {
        if let Some(name) = extract_name_from_filename(filename) {
            metadata
                .extra
                .insert("name".to_string(), serde_json::Value::String(name));
        }
    }
    if metadata.description.is_none() {
        if let Some(desc) = extract_description_from_body(body) {
            metadata.description = Some(desc);
        }
    }

    Ok(Rule {
        metadata,
        body: body.to_string(),
    })
}

fn parse_claude(input: &str, filename: Option<&str>) -> Result<Rule> {
    // CLAUDE.md generally doesn't use frontmatter natively in the same structured way
    let mut metadata = RuleMetadata::default();
    if let Some(name) = extract_name_from_filename(filename) {
        metadata
            .extra
            .insert("name".to_string(), serde_json::Value::String(name));
    }
    if let Some(desc) = extract_description_from_body(input) {
        metadata.description = Some(desc);
    }
    Ok(Rule {
        metadata,
        body: input.to_string(),
    })
}

fn extract_frontmatter(input: &str) -> (Option<&str>, &str) {
    if input.starts_with("---\n") || input.starts_with("---\r\n") {
        if let Some(end_idx) = input[4..].find("\n---") {
            let frontmatter = &input[4..4 + end_idx];
            let rest_idx = 4 + end_idx + 4;
            // Skip the newline after ---
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

fn extract_name_from_filename(filename: Option<&str>) -> Option<String> {
    filename
        .and_then(|f| Path::new(f).file_stem())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

fn extract_description_from_body(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("---") {
            // Take first non-empty, non-heading line
            // Limit to 100 chars
            let truncated = if trimmed.len() > 100 {
                &trimmed[..100]
            } else {
                trimmed
            };
            return Some(truncated.to_string());
        }
    }
    None
}
