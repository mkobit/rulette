use crate::agent_skills::{Skill, SkillMetadata};
use crate::cli::formats::InputFormat;
use crate::{Entity, Rule, RuleMetadata, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;

pub fn parse(input: &str, format: InputFormat, filename: Option<&str>) -> Result<RuletteDocument> {
    let entities = match format {
        InputFormat::Auto => {
            if input.trim_start().starts_with('{') {
                if let Ok(doc) = serde_json::from_str::<RuletteDocument>(input) {
                    return Ok(doc);
                }
            }
            if input.starts_with("---\n") {
                if input.contains("name:") && input.contains("description:") {
                    vec![Entity::Skill(parse_agent_skills(input, filename)?)]
                } else {
                    vec![Entity::Rule(parse_cursor_mdc(input, filename)?)]
                }
            } else {
                vec![Entity::Rule(parse_claude(input, filename)?)]
            }
        }
        InputFormat::IrJson => {
            let doc: RuletteDocument = serde_json::from_str(input)?;
            return Ok(doc);
        }
        InputFormat::IrToml => {
            let doc: RuletteDocument = toml::from_str(input)?;
            return Ok(doc);
        }
        InputFormat::SkillMd | InputFormat::AgentSkills => {
            vec![Entity::Skill(parse_agent_skills(input, filename)?)]
        }
        InputFormat::CursorMdc => vec![Entity::Rule(parse_cursor_mdc(input, filename)?)],
        InputFormat::Claude | InputFormat::Codex => {
            vec![Entity::Rule(parse_claude(input, filename)?)]
        }
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
        #[derive(serde::Deserialize)]
        struct FmParse {
            name: Option<String>,
            description: Option<String>,
            version: Option<String>,
            license: Option<String>,
            compatibility: Option<String>,
            #[serde(rename = "allowed-tools")]
            allowed_tools: Option<String>,
            #[serde(flatten)]
            extra: HashMap<String, serde_json::Value>,
        }
        if let Ok(parsed_fm) = serde_yaml::from_str::<FmParse>(fm) {
            if let Some(name) = parsed_fm.name {
                metadata.name = name;
            }
            if let Some(desc) = parsed_fm.description {
                metadata.description = desc;
            }
            metadata.version = parsed_fm.version;
            metadata.license = parsed_fm.license;
            metadata.compatibility = parsed_fm.compatibility;
            metadata.allowed_tools = parsed_fm.allowed_tools;
            metadata.extra = parsed_fm.extra;
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
        #[derive(serde::Deserialize)]
        struct FmParse {
            description: Option<String>,
            #[serde(flatten)]
            extra: HashMap<String, serde_json::Value>,
        }
        if let Ok(parsed_fm) = serde_yaml::from_str::<FmParse>(fm) {
            metadata.description = parsed_fm.description;
            metadata.extra = parsed_fm.extra;
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
