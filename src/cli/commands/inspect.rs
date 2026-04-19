use crate::backend::{
    AgentSkillsEmitter, ClaudeEmitter, CodexEmitter, CopilotEmitter, CursorEmitter, Emitter,
    GeminiEmitter, WindsurfEmitter,
};
use crate::cli::formats::{InputFormat, OutputFormat};
use crate::frontend::parse;
use clap::Args;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

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
    pub fn execute(&self, strict: bool) -> anyhow::Result<()> {
        let mut combined_entities = vec![];

        for input_path in &self.input {
            if input_path == "-" {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                let doc = parse(&buffer, InputFormat::Auto, None)?;
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
                            if let Ok(doc) = parse(
                                &content,
                                InputFormat::Auto,
                                Some(entry.path().to_str().unwrap()),
                            ) {
                                combined_entities.extend(doc.entities);
                            }
                        }
                    }
                } else {
                    let content = fs::read_to_string(input_path)?;
                    let doc = parse(&content, InputFormat::Auto, Some(input_path))?;
                    combined_entities.extend(doc.entities);
                }
            }
        }

        let doc = crate::RuletteDocument {
            entities: combined_entities,
        };

        let ir_json = serde_json::to_string_pretty(&doc)?;
        println!("=== Rulette IR ===");
        println!("{}", ir_json);

        if let Some(target) = &self.target {
            println!("\n=== Dry-run Emission to {:?} ===", target);

            let output_map = match target {
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

            println!("\n--- Survived Output ---");
            for (rel_path, content) in &output_map {
                if output_map.len() > 1 {
                    println!("--- {} ---", rel_path.display());
                }
                println!("{}", content);
            }
        }

        Ok(())
    }
}
