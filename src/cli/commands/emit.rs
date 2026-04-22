use crate::backend::{
    AgentSkillsEmitter, ClaudeEmitter, ClaudeSettingsEmitter, CodexEmitter, CopilotEmitter,
    CursorEmitter, Emitter, GeminiEmitter, WindsurfEmitter,
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
    pub to: Option<OutputFormat>,

    /// Output path (file or directory) or multiple targets via format:path
    #[arg(short, long)]
    pub out: Vec<String>,

    /// Output scope: project (default) or user
    #[arg(long, default_value = "project")]
    pub scope: String,
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
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(home_dir);

        let path = match to {
            OutputFormat::Claude => home_path.join(".claude").join("skills"),
            OutputFormat::ClaudeSettings => home_path.join(".claude"),
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

pub struct OutputTarget {
    pub format: OutputFormat,
    pub path: Option<String>,
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
            anyhow::bail!("Must specify a target format via --to or format:path in --out");
        }
    }

    for arg in out_args {
        let parts: Vec<&str> = arg.splitn(2, ':').collect();
        if parts.len() == 2 {
            let format_str = parts[0];
            let path_str = parts[1];

            let format_opt = match format_str {
                "claude" => Some(OutputFormat::Claude),
                "claude-settings" => Some(OutputFormat::ClaudeSettings),
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
            anyhow::bail!(
                "Could not parse {} as format:path and no --to format specified",
                arg
            );
        }
    }

    Ok(targets)
}

impl EmitArgs {
    pub fn execute(&self, strict: bool) -> Result<()> {
        let mut combined_entities = vec![];

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

        let targets = parse_targets(&self.out, self.to)?;

        let mut generated_outputs = Vec::new();

        for target in targets {
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
                OutputFormat::ClaudeSettings => ClaudeSettingsEmitter.emit(&doc, strict)?,
            };

            generated_outputs.push((target, output_map));
        }

        for (target, output_map) in generated_outputs {
            let base_path = resolve_output_path(&target.format, &self.scope, target.path.as_ref());

            for (rel_path, content) in &output_map {
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
