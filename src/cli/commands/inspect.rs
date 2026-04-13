use crate::backend::{AgentSkillsEmitter, ClaudeEmitter, CursorEmitter, Emitter};
use crate::cli::formats::{InputFormat, OutputFormat};
use crate::frontend::parse;
use clap::Args;
use std::fs;
use std::io::{self, Read};

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Target format to dry-run emission and show lossy conversion warnings
    #[arg(short, long, value_enum)]
    pub target: Option<OutputFormat>,
}

impl InspectArgs {
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

            let doc = parse(&content, InputFormat::Auto)?;
            combined_entities.extend(doc.entities);
        }

        let doc = crate::RuletteDocument {
            entities: combined_entities,
        };

        let ir_json = serde_json::to_string_pretty(&doc)?;
        println!("=== Rulette IR ===");
        println!("{}", ir_json);

        if let Some(target) = &self.target {
            println!("\n=== Dry-run Emission to {:?} ===", target);
            let strict = false;
            let output = match target {
                OutputFormat::Claude => ClaudeEmitter.emit(&doc, strict)?,
                OutputFormat::CursorMdc => CursorEmitter.emit(&doc, strict)?,
                OutputFormat::AgentSkills => AgentSkillsEmitter.emit(&doc, strict)?,
                OutputFormat::IrJson => serde_json::to_string_pretty(&doc)?,
                OutputFormat::IrToml => toml::to_string(&doc)?,
                _ => anyhow::bail!("Target format not yet supported for emitting"),
            };

            println!("\n--- Survived Output ---");
            println!("{}", output);
        }

        Ok(())
    }
}
