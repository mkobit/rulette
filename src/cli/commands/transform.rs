use clap::Args;
use std::collections::HashSet;

use crate::cli::formats::InputFormat;
use crate::RuletteDocument;
use std::fs;
use std::io::{self, Read};

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Keep only rules matching expression
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

    /// Pipe each rule body through a shell command
    #[arg(long)]
    pub shell: Option<String>,

    /// Remove duplicate entities
    #[arg(long)]
    pub dedup: bool,

    /// Target output format (currently only IrJson is fully supported here)
    #[arg(short, long)]
    pub out: Option<String>,
}

impl TransformArgs {
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

            let filename = if input_path == "-" {
                None
            } else {
                Some(input_path.as_str())
            };
            let doc = crate::frontend::parse(&content, InputFormat::Auto, filename)?;
            combined_entities.extend(doc.entities);
        }

        if self.dedup {
            let mut seen = HashSet::new();
            combined_entities.retain(|entity| {
                // Determine uniqueness using JSON representation
                let json = serde_json::to_string(entity).unwrap();
                seen.insert(json)
            });
        }

        let doc = RuletteDocument {
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
