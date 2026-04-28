use clap::Args;
use schemars::schema_for;

#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Format to output schema for (ir, claude, cursor-mdc, etc.)
    #[arg(short, long, default_value = "ir")]
    pub format: String,

    /// Extension key to output schema for (e.g., rulette:activation)
    #[arg(long)]
    pub extension: Option<String>,
}

impl SchemaArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        if let Some(ext) = &self.extension {
            let schema = match ext.as_str() {
                "rulette:activation" => schema_for!(crate::Activation),
                "rulette:hook-event" => schema_for!(crate::HookEvent),
                "rulette:tool-access" => schema_for!(Vec<crate::ToolAccessRule>),
                _ => anyhow::bail!("Unsupported extension key: {}. Try 'rulette:activation', 'rulette:hook-event', 'rulette:tool-access'.", ext),
            };
            let schema_json = serde_json::to_string_pretty(&schema)?;
            println!("{}", schema_json);
            return Ok(());
        }

        let schema = match self.format.as_str() {
            "ir" => schema_for!(crate::RuletteDocument),
            "claude" => schema_for!(crate::formats::claude::ClaudeSkill),
            "cursor-mdc" => schema_for!(crate::formats::cursor::CursorSkill),
            "agent-skills" => schema_for!(crate::formats::agent_skills::Skill),
            "gemini" => schema_for!(crate::formats::gemini::GeminiSkill),
            "codex" => schema_for!(crate::formats::codex::CodexSkill),
            _ => anyhow::bail!("Unsupported schema format: {}. Try 'ir', 'claude', 'cursor-mdc', 'agent-skills', 'gemini', 'codex'.", self.format),
        };

        let schema_json = serde_json::to_string_pretty(&schema)?;
        println!("{}", schema_json);
        Ok(())
    }
}
