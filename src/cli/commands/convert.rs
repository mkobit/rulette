use crate::backend::{AgentSkillsEmitter, ClaudeEmitter, CodexEmitter, CursorEmitter, Emitter};
use crate::cli::formats::{InputFormat, OutputFormat};
use crate::frontend::parse;
use crate::RuletteDocument;
use anyhow::{anyhow, Result};
use clap::Args;
use std::fs;
use std::io::{self, Read};

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

    /// Override name for rule to skill conversion
    #[arg(long)]
    pub name: Option<String>,

    /// Override description for rule to skill conversion
    #[arg(long)]
    pub description: Option<String>,
}

impl ConvertArgs {
    pub fn execute(&self) -> Result<()> {
        let mut combined_entities = vec![];

        for input_path in &self.input {
            let content = if input_path == "-" {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else {
                fs::read_to_string(input_path)?
            };

            let doc = parse(&content, self.from)?;
            combined_entities.extend(doc.entities);
        }

        let mut doc = RuletteDocument {
            entities: combined_entities,
        };

        // Apply metadata overrides
        if self.name.is_some() || self.description.is_some() {
            for entity in &mut doc.entities {
                match entity {
                    crate::Entity::Skill(skill) => {
                        if let Some(n) = &self.name {
                            skill.metadata.name = n.clone();
                        }
                        if let Some(d) = &self.description {
                            skill.metadata.description = d.clone();
                        }
                    }
                    crate::Entity::Rule(rule) => {
                        if let Some(n) = &self.name {
                            rule.metadata.extra.insert(
                                "rulette_override_name".to_string(),
                                serde_json::Value::String(n.clone()),
                            );
                        }
                        if let Some(d) = &self.description {
                            rule.metadata.extra.insert(
                                "rulette_override_description".to_string(),
                                serde_json::Value::String(d.clone()),
                            );
                        }
                    }
                }
            }
        }

        let strict = false; // Could be passed globally

        let output = match self.to {
            OutputFormat::Claude => ClaudeEmitter.emit(&doc, strict)?,
            OutputFormat::CursorMdc => CursorEmitter.emit(&doc, strict)?,
            OutputFormat::AgentSkills => AgentSkillsEmitter.emit(&doc, strict)?,
            OutputFormat::Codex => CodexEmitter.emit(&doc, strict)?,
            OutputFormat::IrJson => serde_json::to_string_pretty(&doc)?,
            OutputFormat::IrToml => toml::to_string(&doc)?,
            _ => return Err(anyhow!("Target format not yet supported for emitting")),
        };

        if let Some(out_path) = &self.out {
            fs::write(out_path, output)?;
        } else {
            println!("{}", output);
        }

        Ok(())
    }
}
