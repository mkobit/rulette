use crate::cli::formats::{InputFormat, OutputFormat};
use crate::cli::io::read_inputs;
use crate::emitters::{
    AgentSkillsEmitter, ClaudeEmitter, CodexEmitter, CopilotEmitter, CursorEmitter, Emitter,
    GeminiEmitter, WindsurfEmitter,
};
use crate::parsers::parse;
use crate::pipeline;
use crate::{Entity, RuletteDocument};
use anyhow::Result;
use clap::Args;
use serde::Deserialize;
use std::collections::BTreeMap as HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Source format (auto-detected if omitted)
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Target output format
    #[arg(long, value_enum)]
    pub to: Option<OutputFormat>,

    /// Output path (file or directory) or multiple targets via format:path
    #[arg(short, long)]
    pub out: Vec<String>,

    /// Override name metadata for parsed entities
    #[arg(long)]
    pub name: Option<String>,

    /// Override description metadata for parsed entities
    #[arg(long)]
    pub description: Option<String>,

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
    to: Option<OutputFormat>,
    #[serde(default)]
    out: Vec<String>,
}

pub struct OutputTarget {
    pub format: OutputFormat,
    pub path: Option<String>,
}

pub fn resolve_output_path(_to: &OutputFormat, out: Option<&String>) -> Option<PathBuf> {
    if let Some(path) = out {
        return Some(PathBuf::from(path));
    }

    None
}

pub fn parse_targets(
    out_args: &[String],
    to_arg: Option<OutputFormat>,
) -> Result<Vec<OutputTarget>> {
    let mut targets = Vec::new();

    if out_args.is_empty() {
        if let Some(format) = to_arg {
            targets.push(OutputTarget { format, path: None });
            return Ok(targets);
        } else {
            // Default to IrJson to stdout
            targets.push(OutputTarget {
                format: OutputFormat::IrJson,
                path: None,
            });
            return Ok(targets);
        }
    }

    for arg in out_args {
        let parts: Vec<&str> = arg.splitn(2, ':').collect();
        if parts.len() == 2 {
            let format_str = parts[0];
            let path_str = parts[1];

            let format_opt = match format_str {
                "claude" => Some(OutputFormat::Claude),
                "cursor-mdc" => Some(OutputFormat::CursorMdc),
                "codex" => Some(OutputFormat::Codex),
                "windsurf" => Some(OutputFormat::Windsurf),
                "copilot" => Some(OutputFormat::Copilot),
                "gemini" => Some(OutputFormat::Gemini),
                "agent-skills" => Some(OutputFormat::AgentSkills),
                "ir-json" => Some(OutputFormat::IrJson),
                "ir-toml" => Some(OutputFormat::IrToml),
                _ => None,
            };

            if let Some(format) = format_opt {
                targets.push(OutputTarget {
                    format,
                    path: if path_str.is_empty() || path_str == "-" {
                        None
                    } else {
                        Some(path_str.to_string())
                    },
                });
                continue;
            }
        }

        if let Some(format) = to_arg {
            targets.push(OutputTarget {
                format,
                path: if arg == "-" {
                    None
                } else {
                    Some(arg.to_string())
                },
            });
        } else {
            // If it's not format:path and no --to, maybe it's just a path for IrJson?
            // The instruction says: "Default output should be IrJson to stdout if no --to or --out is provided."
            // If --out is provided, we should probably follow the old logic or refine it.
            // Old logic bailed if no --to.
            anyhow::bail!(
                "Could not parse {} as format:path and no --to format specified",
                arg
            );
        }
    }

    Ok(targets)
}

impl TransformArgs {
    pub fn execute(&self, strict: bool) -> Result<()> {
        let mut combined_entities = vec![];

        let inputs = read_inputs(&self.input)?;
        for input_file in inputs {
            let doc = parse(
                &input_file.content,
                self.from,
                input_file.filename.as_deref(),
            )?;
            combined_entities.extend(doc.entities);
        }

        // Apply metadata overrides (from Parse/Convert)
        for entity in &mut combined_entities {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                crate::Entity::Rule(rule) => {
                    if let Some(name) = &self.name {
                        rule.metadata
                            .extra
                            .insert("name".to_string(), serde_json::Value::String(name.clone()));
                    }
                    if let Some(desc) = &self.description {
                        rule.metadata.description = Some(desc.clone());
                    }
                }
                crate::Entity::Skill(skill) => {
                    if let Some(name) = &self.name {
                        skill.metadata.name = name.clone();
                    }
                    if let Some(desc) = &self.description {
                        skill.metadata.description = desc.clone();
                    }
                }
                crate::Entity::McpServer(mcp) => {
                    if let Some(name) = &self.name {
                        mcp.metadata.name = name.clone();
                    }
                }
            }
        }

        let mut run_filter = self.filter.clone();
        let mut run_exclude = self.exclude.clone();
        let mut run_rename = self.rename.clone();
        let mut run_set = self.set.clone();
        let mut run_to = self.to;
        let mut run_out = self.out.clone();

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
            if run_to.is_none() {
                run_to = config.to;
            }
            if run_out.is_empty() {
                run_out = config.out;
            }
        }

        let run_targets = parse_targets(&run_out, run_to)?;

        if let Some(filter_expr) = &run_filter {
            combined_entities.retain(|entity| pipeline::match_expr(entity, filter_expr));
        }

        if let Some(exclude_expr) = &run_exclude {
            combined_entities.retain(|entity| !pipeline::match_expr(entity, exclude_expr));
        }

        if let Some(rename_expr) = &run_rename {
            let parts: Vec<&str> = rename_expr.split('=').collect();
            if parts.len() == 2 {
                let from = parts[0].trim();
                let to = parts[1].trim();
                for entity in &mut combined_entities {
                    pipeline::rename_field(entity, from, to);
                }
            }
        }

        if let Some(set_expr) = &run_set {
            let parts: Vec<&str> = set_expr.split('=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let val = parts[1].trim();
                for entity in &mut combined_entities {
                    pipeline::set_field(entity, key, val);
                }
            }
        }

        // Strict Identity Collision Detection
        {
            let mut seen: std::collections::HashMap<String, &Entity> =
                std::collections::HashMap::new();

            for entity in &combined_entities {
                let name = match entity {
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

                let filename = match entity {
                    Entity::Rule(rule) => rule
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Skill(skill) => skill
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::McpServer(mcp) => mcp
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Hook(hook) => hook
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Agent(agent) => agent
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Permissions(perms) => perms
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                let id = if let (Some(n), Some(f)) = (name, filename) {
                    format!("{}:{}", f, n)
                } else {
                    match entity {
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
                    }
                    .unwrap_or_else(|| {
                        serde_json::to_string(&entity).unwrap_or_else(|_| "unknown".to_string())
                    })
                };

                if let Some(_existing) = seen.get(&id) {
                    anyhow::bail!("Identity collision detected: entity '{}' already exists. Rulette requires unique identities across all inputs.", id);
                }
                seen.insert(id, entity);
            }
        }

        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: combined_entities,
        };

        // Emission logic
        let mut generated_outputs = Vec::new();

        for target in run_targets {
            let output_map = match target.format {
                OutputFormat::Claude => ClaudeEmitter.emit(&doc, strict)?,
                OutputFormat::CursorMdc => CursorEmitter.emit(&doc, strict)?,
                OutputFormat::AgentSkills => AgentSkillsEmitter.emit(&doc, strict)?,
                OutputFormat::Copilot => CopilotEmitter.emit(&doc, strict)?,
                OutputFormat::Windsurf => WindsurfEmitter.emit(&doc, strict)?,
                OutputFormat::Gemini => GeminiEmitter.emit(&doc, strict)?,
                OutputFormat::Codex => CodexEmitter.emit(&doc, strict)?,
                OutputFormat::IrJson => {
                    let mut map = HashMap::new();
                    map.insert(
                        PathBuf::from("ir.json"),
                        serde_json::to_string_pretty(&doc)?,
                    );
                    map
                }
                OutputFormat::IrToml => {
                    let mut map = HashMap::new();
                    map.insert(PathBuf::from("ir.toml"), toml::to_string(&doc)?);
                    map
                }
                OutputFormat::JsonSchema => {
                    let mut map = HashMap::new();
                    let schema = schemars::schema_for!(crate::RuletteDocument);
                    map.insert(
                        PathBuf::from("schema.json"),
                        serde_json::to_string_pretty(&schema)?,
                    );
                    map
                }
            };
            generated_outputs.push((target, output_map));
        }

        for (target, output_map) in generated_outputs {
            let base_path = resolve_output_path(&target.format, target.path.as_ref());

            let mut sorted_paths: Vec<_> = output_map.keys().collect();
            sorted_paths.sort();

            for rel_path in sorted_paths {
                let content = &output_map[rel_path];
                let final_path = if let Some(ref base) = base_path {
                    let mut p = base.clone();
                    if p.is_dir() || p.extension().is_none() || output_map.len() > 1 {
                        p.push(rel_path);
                    }
                    p
                } else {
                    rel_path.clone()
                };

                if let Some(parent) = final_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                if base_path.is_none() {
                    if output_map.len() > 1 {
                        println!("--- {} ---", final_path.display());
                    }
                    println!("{}", content);
                } else {
                    fs::write(&final_path, content)?;
                    println!("Emitted to {}", final_path.display());
                }
            }
        }

        Ok(())
    }
}
