use crate::backend::{AgentSkillsEmitter, ClaudeEmitter, CursorEmitter, Emitter};
use crate::cli::commands::emit::resolve_output_path;
use crate::cli::formats::{InputFormat, OutputFormat};
use crate::frontend::parse;
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

        let doc = crate::RuletteDocument {
            entities: combined_entities,
        };

        let strict = false;
        let output = match self.to {
            OutputFormat::Claude => ClaudeEmitter.emit(&doc, strict)?,
            OutputFormat::CursorMdc => CursorEmitter.emit(&doc, strict)?,
            OutputFormat::AgentSkills => AgentSkillsEmitter.emit(&doc, strict)?,
            OutputFormat::IrJson => serde_json::to_string_pretty(&doc)?,
            OutputFormat::IrToml => toml::to_string(&doc)?,
            _ => return Err(anyhow!("Target format not yet supported for emitting")),
        };

        if let Some(mut path) = resolve_output_path(&self.to, &self.scope, self.out.as_ref()) {
            if path.extension().is_none() && path.to_string_lossy().ends_with("skills")
                || path.to_string_lossy().ends_with("rules")
            {
                let default_ext = match self.to {
                    OutputFormat::Claude => "md",
                    OutputFormat::CursorMdc => "mdc",
                    _ => "txt",
                };
                path.push(format!("rulette_generated.{}", default_ext));
            }

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, output)?;
            println!("Converted and emitted to {}", path.display());
        } else {
            println!("{}", output);
        }

        Ok(())
    }
}
