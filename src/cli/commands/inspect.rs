use crate::cli::formats::{InputFormat, OutputFormat};
use crate::cli::io::read_inputs;
use crate::emitters::{
    self, AgentSkillsEmitter, AntigravityEmitter, CapabilityEntry, ClaudeEmitter, CodexEmitter,
    CopilotEmitter, CoverageStatus, CursorEmitter, CursorMcpEmitter, Emitter, GeminiEmitter,
    WindsurfEmitter,
};
use crate::parsers::parse;
use clap::Args;
use std::collections::{BTreeMap as HashMap, BTreeSet};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Target format to dry-run emission and show lossy conversion warnings
    #[arg(short, long, value_enum, conflicts_with = "coverage")]
    pub to: Option<OutputFormat>,

    /// Compute a Supported/Lossy/Dropped capability matrix across every registered target
    #[arg(long, conflicts_with = "to")]
    pub coverage: bool,

    /// Render --coverage output as JSON instead of a table (requires --coverage)
    #[arg(long, requires = "coverage")]
    pub json: bool,
}

/// The registered `Emitter` targets `--coverage` probes, in display order.
/// Excludes ir-json/ir-toml/json-schema: those are the lossless IR itself,
/// not lossy targets with a capability story of their own.
fn coverage_targets() -> Vec<(&'static str, Box<dyn Emitter>)> {
    vec![
        ("claude", Box::new(ClaudeEmitter)),
        ("cursor-mdc", Box::new(CursorEmitter)),
        ("cursor-mcp", Box::new(CursorMcpEmitter)),
        ("codex", Box::new(CodexEmitter)),
        ("windsurf", Box::new(WindsurfEmitter)),
        ("copilot", Box::new(CopilotEmitter)),
        ("gemini", Box::new(GeminiEmitter)),
        ("antigravity", Box::new(AntigravityEmitter)),
        ("agent-skills", Box::new(AgentSkillsEmitter)),
    ]
}

#[derive(serde::Serialize)]
struct CoverageJsonEntry {
    target: String,
    entity_kind: String,
    status: CoverageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

impl InspectArgs {
    pub fn execute(&self, strict: bool, quiet: bool) -> anyhow::Result<()> {
        let mut combined_entities = vec![];

        let inputs = read_inputs(&self.input)?;
        for input_file in inputs {
            let doc = parse(
                &input_file.content,
                InputFormat::Auto,
                input_file.filename.as_deref(),
            )?;
            combined_entities.extend(doc.entities);
        }

        let doc = crate::RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: combined_entities,
        };

        if !quiet {
            let ir_json = serde_json::to_string_pretty(&doc)?;
            println!("=== Rulette IR ===");
            println!("{}", ir_json);
        }

        if self.coverage {
            return self.execute_coverage(&doc, strict, quiet);
        }

        if let Some(to) = &self.to {
            if !quiet {
                println!("\n=== Dry-run Emission to {:?} ===", to);
            }

            let output_map = match to {
                OutputFormat::Claude => ClaudeEmitter.emit(&doc, strict)?,
                OutputFormat::CursorMdc => CursorEmitter.emit(&doc, strict)?,
                OutputFormat::CursorMcp => CursorMcpEmitter.emit(&doc, strict)?,
                OutputFormat::AgentSkills => AgentSkillsEmitter.emit(&doc, strict)?,
                OutputFormat::Copilot => CopilotEmitter.emit(&doc, strict)?,
                OutputFormat::Windsurf => WindsurfEmitter.emit(&doc, strict)?,
                OutputFormat::Gemini => GeminiEmitter.emit(&doc, strict)?,
                OutputFormat::Antigravity => AntigravityEmitter.emit(&doc, strict)?,
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
                OutputFormat::JsonSchema => {
                    let mut map = HashMap::new();
                    let schema = schemars::schema_for!(crate::RuletteDocument);
                    map.insert(
                        PathBuf::from("schema.json"),
                        serde_json::to_string_pretty(&schema)?,
                    );
                    map
                }
                OutputFormat::TransformConfig => {
                    anyhow::bail!(
                        "`transform-config` is only a valid target for the `transform` command, not `inspect`"
                    );
                }
            };

            if !quiet {
                println!("\n--- Survived Output ---");
                for (rel_path, content) in &output_map {
                    if output_map.len() > 1 {
                        println!("--- {} ---", rel_path.display());
                    }
                    println!("{}", content);
                }
            }
        }

        Ok(())
    }

    fn execute_coverage(
        &self,
        doc: &crate::RuletteDocument,
        strict: bool,
        quiet: bool,
    ) -> anyhow::Result<()> {
        let targets = coverage_targets();

        // Entity kinds present in the input only (spec: "Coverage matrix
        // reflects actual input"), not every kind the IR schema defines.
        let entity_kinds: BTreeSet<String> = doc
            .entities
            .iter()
            .map(|e| emitters::entity_kind_kebab(e).to_string())
            .collect();

        let mut matrix: HashMap<(String, String), CapabilityEntry> = HashMap::new();
        for (target_name, emitter) in &targets {
            for entry in emitter.capabilities(doc) {
                matrix.insert((target_name.to_string(), entry.entity_kind.clone()), entry);
            }
        }

        let has_failure = matrix
            .values()
            .any(|entry| entry.status != CoverageStatus::Supported);

        if self.json {
            // Unlike the human-readable table, JSON output is never gated by
            // --quiet: its whole purpose is scripting/piping (`--coverage
            // --json | jq ...`), and `-q --coverage --json` is exactly how a
            // caller asks for JSON-only stdout with the IR preamble
            // suppressed. Gating it too would make that combination silent.
            let mut json_entries: Vec<CoverageJsonEntry> = Vec::new();
            for (target_name, _) in &targets {
                for kind in &entity_kinds {
                    let entry = matrix.get(&(target_name.to_string(), kind.clone()));
                    json_entries.push(CoverageJsonEntry {
                        target: target_name.to_string(),
                        entity_kind: kind.clone(),
                        status: entry.map(|e| e.status).unwrap_or(CoverageStatus::Supported),
                        reason: entry.and_then(|e| e.reason.clone()),
                    });
                }
            }
            println!("{}", serde_json::to_string_pretty(&json_entries)?);
        } else if !quiet {
            println!("\n=== Coverage Matrix ===");
            print!("{:<15}", "entity_kind");
            for (target_name, _) in &targets {
                print!(" {target_name:<12}");
            }
            println!();
            for kind in &entity_kinds {
                print!("{kind:<15}");
                for (target_name, _) in &targets {
                    let status = matrix
                        .get(&(target_name.to_string(), kind.clone()))
                        .map(|e| e.status)
                        .unwrap_or(CoverageStatus::Supported);
                    print!(" {:<12}", format!("{status:?}"));
                }
                println!();
            }
        }

        if strict && has_failure {
            anyhow::bail!(
                "Coverage check failed: one or more targets report Lossy or Dropped entity kinds"
            );
        }

        Ok(())
    }
}
