use crate::backend::{
    AgentSkillsEmitter, ClaudeEmitter, CodexEmitter, CopilotEmitter, CursorEmitter, Emitter,
    GeminiEmitter, WindsurfEmitter,
};
use crate::cli::commands::emit::resolve_output_path;
use crate::cli::formats::{InputFormat, OutputFormat};
use crate::frontend::parse;
use anyhow::Result;
use clap::Args;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Source format (auto-detected if omitted)
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Target output format
    #[arg(long, value_enum)]
    pub to: OutputFormat,

    /// Output path (file or directory)
    #[arg(short, long)]
    pub out: Option<String>,

    /// Output scope: project (default) or user
    #[arg(long, default_value = "project")]
    pub scope: String,

    /// Merge multiple rules into a single output file
    #[arg(long)]
    pub merge: bool,

    /// Override name metadata for parsed entities
    #[arg(long)]
    pub name: Option<String>,

    /// Override description metadata for parsed entities
    #[arg(long)]
    pub description: Option<String>,
}

impl ConvertArgs {
    pub fn execute(&self, strict: bool) -> Result<()> {
        let mut combined_entities = vec![];

        for input_path in &self.input {
            if input_path == "-" {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                let doc = parse(&buffer, self.from, None)?;
                combined_entities.extend(doc.entities);
            } else {
                let path = std::path::Path::new(input_path);
                if path.is_dir() {
                    for entry in walkdir::WalkDir::new(path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            let content = fs::read_to_string(entry.path())?;
                            if let Ok(doc) =
                                parse(&content, self.from, Some(entry.path().to_str().unwrap()))
                            {
                                combined_entities.extend(doc.entities);
                            }
                        }
                    }
                } else {
                    let content = fs::read_to_string(input_path)?;
                    let doc = parse(&content, self.from, Some(input_path))?;
                    combined_entities.extend(doc.entities);
                }
            }
        }

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

        let doc = crate::RuletteDocument {
            entities: combined_entities,
        };

        let output_map = match self.to {
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
        };

        let base_path = resolve_output_path(&self.to, &self.scope, self.out.as_ref());

        for (rel_path, content) in &output_map {
            let final_path = if let Some(ref base) = base_path {
                let mut p = base.clone();
                if p.is_dir() || p.extension().is_none() || output_map.len() > 1 {
                    p.push(rel_path);
                } else {
                    // Single file output
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
                println!("Converted and emitted to {}", final_path.display());
            }
        }

        Ok(())
    }
}
