use crate::backend::{
    AgentSkillsEmitter, ClaudeEmitter, CodexEmitter, CopilotEmitter, CursorEmitter, Emitter,
    GeminiEmitter, WindsurfEmitter,
};
use crate::cli::formats::{InputFormat, OutputFormat};
use crate::cli::io::read_inputs;
use crate::frontend::parse;
use clap::Args;

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
            entities: combined_entities,
        };

        let ir_json = serde_json::to_string_pretty(&doc)?;
        println!("=== Rulette IR ===");
        println!("{}", ir_json);

        if let Some(target) = &self.target {
            println!("\n=== Dry-run Emission to {:?} ===", target);

            let output = match target {
                OutputFormat::Claude => ClaudeEmitter.emit(&doc, strict)?,
                OutputFormat::CursorMdc => CursorEmitter.emit(&doc, strict)?,
                OutputFormat::AgentSkills => AgentSkillsEmitter.emit(&doc, strict)?,
                OutputFormat::Copilot => CopilotEmitter.emit(&doc, strict)?,
                OutputFormat::Windsurf => WindsurfEmitter.emit(&doc, strict)?,
                OutputFormat::Gemini => GeminiEmitter.emit(&doc, strict)?,
                OutputFormat::Codex => CodexEmitter.emit(&doc, strict)?,
                OutputFormat::IrJson => serde_json::to_string_pretty(&doc)?,
                OutputFormat::IrToml => toml::to_string(&doc)?,
            };

            println!("\n--- Survived Output ---");
            println!("{}", output);
        }

        Ok(())
    }
}
