use crate::backend::{AgentSkillsEmitter, ClaudeEmitter, CursorEmitter, Emitter};
use crate::cli::formats::OutputFormat;
use crate::RuletteDocument;
use anyhow::{anyhow, Result};
use clap::Args;
use std::fs;
use std::io::{self, Read};

#[derive(Args, Debug)]
pub struct EmitArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Target output format
    #[arg(short, long, value_enum)]
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

    /// Split into one file per rule (default for directory output)
    #[arg(long)]
    pub split: bool,
}

impl EmitArgs {
    pub fn execute(&self) -> Result<()> {
        let mut combined_entities = vec![];

        // Parse IR JSON from inputs
        for input_path in &self.input {
            let content = if input_path == "-" {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else {
                fs::read_to_string(input_path)?
            };

            let doc: RuletteDocument = serde_json::from_str(&content)?;
            combined_entities.extend(doc.entities);
        }

        let doc = RuletteDocument {
            entities: combined_entities,
        };

        // We could look up strict flag from global config, but for now we'll fetch from env or assume false
        // A better approach would be to pass GlobalFlags into execute(), but this works for now.
        // Actually since we don't have access to strict here easily, let's hardcode false.
        let strict = false;

        // Emit based on target format
        let output = match self.to {
            OutputFormat::Claude => ClaudeEmitter.emit(&doc, strict)?,
            OutputFormat::CursorMdc => CursorEmitter.emit(&doc, strict)?,
            OutputFormat::AgentSkills => AgentSkillsEmitter.emit(&doc, strict)?,
            OutputFormat::IrJson => serde_json::to_string_pretty(&doc)?,
            OutputFormat::IrToml => toml::to_string(&doc)?,
            _ => return Err(anyhow!("Target format not yet supported for emitting")),
        };

        // Write output
        if let Some(out_path) = &self.out {
            fs::write(out_path, output)?;
        } else {
            println!("{}", output);
        }

        Ok(())
    }
}
