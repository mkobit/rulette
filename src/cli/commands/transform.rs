use clap::Args;
use serde::Deserialize;


use crate::cli::formats::InputFormat;
use crate::cli::io::read_inputs;
use crate::{Entity, RuletteDocument};
use std::fs;

use clap::ValueEnum;

#[derive(ValueEnum, Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictResolution {
    #[default]
    Error,
    TakeFirst,
    TakeLast,
}

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Keep only rules matching expression (e.g., 'license == "MIT"')
    #[arg(long)]
    pub filter: Option<String>,

    /// Remove rules matching expression
    #[arg(long)]
    pub exclude: Option<String>,

    /// Rename a metadata field value (from=to)
    #[arg(long)]
    pub rename: Option<String>,

    /// Set a metadata field (field=value)
    #[arg(long)]
    pub set: Option<String>,

    /// Load transform pipeline from TOML file
    #[arg(long)]
    pub config: Option<String>,

    /// Remove duplicate entities
    #[arg(long)]
    pub dedup: bool,

    /// How to handle duplicate entities with the same identity but different content
    #[arg(long, default_value = "error")]
    pub on_conflict: ConflictResolution,

    /// Target output format (currently only IrJson is fully supported here)
    #[arg(short, long)]
    pub out: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TransformConfig {
    #[serde(default)]
    filter: Option<String>,
    #[serde(default)]
    exclude: Option<String>,
    #[serde(default)]
    rename: Option<String>,
    #[serde(default)]
    set: Option<String>,
    #[serde(default)]
    dedup: Option<bool>,
    #[serde(default)]
    on_conflict: Option<ConflictResolution>,
}

fn match_expr(entity: &Entity, expr: &str) -> bool {
    let parts: Vec<&str> = expr.split("==").collect();
    if parts.len() == 2 {
        let key = parts[0].trim();
        let val = parts[1].trim().trim_matches(|c| c == '"' || c == '\'');

        if let Ok(json_val) = serde_json::to_value(entity) {
            if let Some(metadata) = json_val.get("metadata") {
                if let Some(field) = metadata.get(key) {
                    if field.as_str() == Some(val) {
                        return true;
                    }
                }
                if let Some(extra) = metadata.get("extra") {
                    if let Some(field) = extra.get(key) {
                        if field.as_str() == Some(val) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    if let Ok(json) = serde_json::to_string(entity) {
        if json.contains(expr) {
            return true;
        }
    }
    false
}

fn rename_field(entity: &mut Entity, from: &str, to: &str) {
    match entity {
        crate::Entity::Hook(_) | crate::Entity::Agent(_) | crate::Entity::Permissions(_) => {}
        Entity::Rule(rule) => {
            if let Some(val) = rule.metadata.extra.remove(from) {
                rule.metadata.extra.insert(to.to_string(), val);
            }
        }
        Entity::Skill(skill) => {
            if let Some(val) = skill.metadata.extra.remove(from) {
                skill.metadata.extra.insert(to.to_string(), val);
            }
        }
        Entity::McpServer(mcp) => {
            if let Some(val) = mcp.metadata.extra.remove(from) {
                mcp.metadata.extra.insert(to.to_string(), val);
            }
        }
    }
}

fn set_field(entity: &mut Entity, key: &str, value: &str) {
    let json_val = serde_json::Value::String(value.to_string());
    match entity {
        crate::Entity::Hook(_) | crate::Entity::Agent(_) | crate::Entity::Permissions(_) => {}
        Entity::Rule(rule) => {
            rule.metadata.extra.insert(key.to_string(), json_val);
        }
        Entity::Skill(skill) => {
            skill.metadata.extra.insert(key.to_string(), json_val);
        }
        Entity::McpServer(mcp) => {
            mcp.metadata.extra.insert(key.to_string(), json_val);
        }
    }
}

impl TransformArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        let mut combined_entities = vec![];

        let inputs = read_inputs(&self.input)?;
        for input_file in inputs {
            let doc = crate::frontend::parse(
                &input_file.content,
                InputFormat::Auto,
                input_file.filename.as_deref(),
            )?;
            combined_entities.extend(doc.entities);
        }

        let mut run_filter = self.filter.clone();
        let mut run_exclude = self.exclude.clone();
        let mut run_rename = self.rename.clone();
        let mut run_set = self.set.clone();
        let mut run_dedup = self.dedup;
        let mut run_on_conflict = self.on_conflict.clone();

        if let Some(config_path) = &self.config {
            let config_str = fs::read_to_string(config_path)?;
            let config: TransformConfig = toml::from_str(&config_str)?;
            if run_filter.is_none() {
                run_filter = config.filter;
            }
            if run_exclude.is_none() {
                run_exclude = config.exclude;
            }
            if run_rename.is_none() {
                run_rename = config.rename;
            }
            if run_set.is_none() {
                run_set = config.set;
            }
            if !run_dedup {
                run_dedup = config.dedup.unwrap_or(false);
            }
            if let Some(oc) = config.on_conflict {
                run_on_conflict = oc;
            }
        }

        if let Some(filter_expr) = &run_filter {
            combined_entities.retain(|entity| match_expr(entity, filter_expr));
        }

        if let Some(exclude_expr) = &run_exclude {
            combined_entities.retain(|entity| !match_expr(entity, exclude_expr));
        }

        if let Some(rename_expr) = &run_rename {
            let parts: Vec<&str> = rename_expr.split('=').collect();
            if parts.len() == 2 {
                let from = parts[0].trim();
                let to = parts[1].trim();
                for entity in &mut combined_entities {
                    rename_field(entity, from, to);
                }
            }
        }

        if let Some(set_expr) = &run_set {
            let parts: Vec<&str> = set_expr.split('=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let val = parts[1].trim();
                for entity in &mut combined_entities {
                    set_field(entity, key, val);
                }
            }
        }

        if run_dedup {
            let mut result_entities = vec![];
            // map from identity to index in result_entities
            let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

            for entity in combined_entities.into_iter() {
                let identity = match &entity {
                    Entity::Rule(rule) => rule
                        .metadata
                        .extra
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Skill(skill) => Some(skill.metadata.name.clone()),
                    Entity::McpServer(mcp) => Some(mcp.metadata.name.clone()),
                    Entity::Hook(hook) => Some(hook.metadata.name.clone()),
                    Entity::Agent(agent) => Some(agent.metadata.name.clone()),
                    Entity::Permissions(perms) => perms.metadata.name.clone(),
                };

                let identity_key = identity.unwrap_or_else(|| {
                    // For unnamed entities, format the `Value` using Debug which will be consistent enough
                    // or serialize it. Using `format!("{:?}", val)` guarantees a stable deterministic string
                    // if BTreeMap was used, but since we use HashMap it's not strictly deterministic.
                    // However, it's better to just hash the entity content robustly.
                    // Since it's a fallback for unnamed entities, standard JSON stringification is often the only way,
                    // though it remains technically non-deterministic for HashMaps.
                    serde_json::to_string(&entity).unwrap_or_else(|_| "unknown".to_string())
                });

                if let Some(&index) = seen.get(&identity_key) {
                    let existing_entity = &result_entities[index];

                    let new_val = serde_json::to_value(&entity).unwrap_or(serde_json::Value::Null);
                    let existing_val = serde_json::to_value(existing_entity).unwrap_or(serde_json::Value::Null);

                    if new_val != existing_val {
                        // Conflict!
                        match run_on_conflict {
                            ConflictResolution::Error => {
                                anyhow::bail!("Conflict detected for entity '{}'. Entities have the same identity but different content.", identity_key);
                            }
                            ConflictResolution::TakeFirst => {
                                // Do nothing, keep the existing one
                            }
                            ConflictResolution::TakeLast => {
                                // Replace the existing one
                                result_entities[index] = entity;
                            }
                        }
                    }
                    // If they are exactly the same, do nothing (deduplicated)
                } else {
                    seen.insert(identity_key, result_entities.len());
                    result_entities.push(entity);
                }
            }
            combined_entities = result_entities;
        }

        let doc = RuletteDocument {
            entities: combined_entities,
        };

        let output_json = serde_json::to_string_pretty(&doc)?;

        if let Some(out_path) = &self.out {
            fs::write(out_path, output_json)?;
        } else {
            println!("{}", output_json);
        }

        Ok(())
    }
}
