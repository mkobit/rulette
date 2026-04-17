use clap::Args;
use serde::Deserialize;
use std::collections::HashSet;

use crate::cli::formats::InputFormat;
use crate::{Entity, RuletteDocument};
use std::fs;
use std::io::{self, Read};

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

        for input_path in &self.input {
            let content = if input_path == "-" {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else {
                fs::read_to_string(input_path)?
            };

            let filename = if input_path == "-" {
                None
            } else {
                Some(input_path.as_str())
            };
            let doc = crate::frontend::parse(&content, InputFormat::Auto, filename)?;
            combined_entities.extend(doc.entities);
        }

        let mut run_filter = self.filter.clone();
        let mut run_exclude = self.exclude.clone();
        let mut run_rename = self.rename.clone();
        let mut run_set = self.set.clone();
        let mut run_dedup = self.dedup;

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
            let mut seen = HashSet::new();
            combined_entities.retain(|entity| {
                if let Ok(json) = serde_json::to_string(entity) {
                    seen.insert(json)
                } else {
                    false
                }
            });
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
