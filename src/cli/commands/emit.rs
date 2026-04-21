use crate::backend::{
    AgentSkillsEmitter, ClaudeEmitter, CodexEmitter, CopilotEmitter, CursorEmitter, Emitter,
    GeminiEmitter, WindsurfEmitter,
};
use crate::cli::formats::OutputFormat;
use crate::cli::io::read_inputs;
use crate::RuletteDocument;
use anyhow::Result;
use clap::Args;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

pub fn resolve_output_path(
    to: &OutputFormat,
    scope: &str,
    out: Option<&String>,
) -> Option<PathBuf> {
    if let Some(path) = out {
        return Some(PathBuf::from(path));
    }

    if scope == "user" {
        // Simple mock of user home directory since we don't have dirs crate
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(home_dir);

        let path = match to {
            OutputFormat::Claude => home_path.join(".claude").join("skills"),
            OutputFormat::CursorMdc => home_path.join(".cursor").join("rules"),
            OutputFormat::Copilot => home_path
                .join(".config")
                .join("github-copilot")
                .join("instructions.md"),
            OutputFormat::Codex => home_path.join(".codex").join("AGENTS.md"),
            OutputFormat::Gemini => home_path.join(".gemini").join("GEMINI.md"),
            OutputFormat::Windsurf => home_path.join(".windsurf").join("windsurfrules"),
            _ => return None,
        };
        return Some(path);
    }
    None
}

impl EmitArgs {
    pub fn execute(&self, strict: bool) -> Result<()> {
        let mut combined_entities = vec![];

        // Parse IR JSON from inputs
        let inputs = read_inputs(&self.input)?;
        for input_file in inputs {
            let doc = crate::frontend::parse(
                &input_file.content,
                crate::cli::formats::InputFormat::Auto,
                input_file.filename.as_deref(),
            )?;
            combined_entities.extend(doc.entities);
        }

        let doc = RuletteDocument {
            entities: combined_entities,
        };

        // Emit based on target format
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
            OutputFormat::ClaudeSettings => {
                anyhow::bail!("Emitting to ClaudeSettings is not supported yet");
            }
        };

        let base_path = resolve_output_path(&self.to, &self.scope, self.out.as_ref());

        for (rel_path, content) in &output_map {
            let final_path = if let Some(ref base) = base_path {
                let mut p = base.clone();
                if p.is_dir() || p.extension().is_none() || output_map.len() > 1 {
                    p.push(rel_path);
                } else {
                    // Single file output, keep as is (unless rel_path is different?)
                    // If base_path was provided as a specific file, we honor it for single-file output
                }
                p
            } else {
                rel_path.clone()
            };

            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent)?;
            }

            if base_path.is_none() {
                // If no output path, print to stdout (with headers if multiple)
                if output_map.len() > 1 {
                    println!("--- {} ---", final_path.display());
                }
                println!("{}", content);
            } else {
                fs::write(&final_path, content)?;
                println!("Emitted to {}", final_path.display());
            }
        }

        Ok(())
    }
}
