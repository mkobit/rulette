use clap::Args;
use schemars::schema_for;

#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Format to output schema for (ir, claude, cursor-mdc, etc.)
    #[arg(short, long, default_value = "ir")]
    pub format: String,
}

impl SchemaArgs {

    pub fn execute(&self) -> anyhow::Result<()> {
        let schema = match self.format.as_str() {
            "ir" => schema_for!(crate::RuletteDocument),
            "claude" => schema_for!(crate::claude::ClaudeSkill),
            "cursor-mdc" => schema_for!(crate::cursor::CursorSkill),
            "agent-skills" => schema_for!(crate::agent_skills::Skill),
            "gemini" => schema_for!(crate::gemini::GeminiSkill),
            "codex" => schema_for!(crate::codex::CodexSkill),
            _ => anyhow::bail!("Unsupported schema format: {}. Try 'ir', 'claude', 'cursor-mdc', 'agent-skills', 'gemini', 'codex'.", self.format),
        };

        let schema_json = serde_json::to_string_pretty(&schema)?;
        println!("{}", schema_json);
        Ok(())
    }
}
