use crate::cli::formats::InputFormat;
use crate::frontend::parse;
use clap::Args;
use std::fs;
use std::io::{self, Read};

#[derive(Args, Debug)]
pub struct ParseArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Force input format detection
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    pub out: Option<String>,

    /// Fail on parse warnings
    #[arg(long)]
    pub strict: bool,

    /// Override name metadata for parsed entities
    #[arg(long)]
    pub name: Option<String>,

    /// Override description metadata for parsed entities
    #[arg(long)]
    pub description: Option<String>,
}

impl ParseArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
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
                    for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                        if entry.file_type().is_file() {
                            let content = fs::read_to_string(entry.path())?;
                            if let Ok(doc) = parse(&content, self.from, Some(entry.path().to_str().unwrap())) {
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

        let output_json = serde_json::to_string_pretty(&doc)?;

        if let Some(out_path) = &self.out {
            fs::write(out_path, output_json)?;
        } else {
            println!("{}", output_json);
        }

        Ok(())
    }
}
