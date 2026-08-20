use clap::Args;
use schemars::schema_for;

#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Format to output schema for (ir, claude, cursor-mdc, etc.)
    #[arg(short, long, default_value = "ir")]
    pub to: String,

    /// Extension key to output schema for (e.g., rulette:activation)
    #[arg(long)]
    pub extension: Option<String>,
}

impl SchemaArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        if let Some(ext) = &self.extension {
            let schema = match ext.as_str() {
                "rulette:activation" => schema_for!(crate::TargetOverrides<crate::Activation>),
                "rulette:hook-event" => schema_for!(crate::HookEvent),
                "rulette:tool-access" => schema_for!(Vec<crate::ToolAccessRule>),
                "rulette:agent-tools" => schema_for!(Vec<String>),
                "rulette:models" => schema_for!(Vec<String>),
                "rulette:directory-scope" => schema_for!(String),
                "rulette:settings-overrides" => schema_for!(serde_json::Value),
                _ => anyhow::bail!("Unsupported extension key: {}. Try 'rulette:activation', 'rulette:hook-event', 'rulette:tool-access', 'rulette:agent-tools', 'rulette:models', 'rulette:directory-scope', 'rulette:settings-overrides'.", ext),
            };
            let schema_json = serde_json::to_string_pretty(&schema)?;
            println!("{}", schema_json);
            return Ok(());
        }

        let schema = match self.to.as_str() {
            "ir" => schema_for!(crate::RuletteDocument),
            "claude" => schema_for!(crate::parsers::claude::ClaudeSkill),
            "cursor-mdc" => schema_for!(crate::parsers::cursor::CursorSkill),
            "cursor-mcp" => schema_for!(crate::emitters::cursor_mcp::CursorMcpFileSchema),
            "agent-skills" => schema_for!(crate::agent_skills::Skill),
            "gemini" => schema_for!(crate::parsers::gemini::GeminiSkill),
            "codex" => schema_for!(crate::parsers::codex::CodexSkill),
            // Windsurf and Copilot share Claude's plain-body parsing (parsers::frontend::parse_claude),
            // so they share its schema too rather than duplicating an identical placeholder type.
            "windsurf" | "copilot" => schema_for!(crate::parsers::claude::ClaudeSkill),
            _ => anyhow::bail!("Unsupported schema format: {}. Try 'ir', 'claude', 'cursor-mdc', 'cursor-mcp', 'agent-skills', 'gemini', 'codex', 'windsurf', 'copilot'.", self.to),
        };

        let schema_json = serde_json::to_string_pretty(&schema)?;
        println!("{}", schema_json);
        Ok(())
    }
}
