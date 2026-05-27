use crate::RuletteDocument;
use anyhow::Result;
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub mod agent_skills;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod gemini;
pub mod windsurf;

pub trait Emitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>>;
}

pub use agent_skills::AgentSkillsEmitter;
pub use claude::ClaudeEmitter;
pub use codex::CodexEmitter;
pub use copilot::CopilotEmitter;
pub use cursor::CursorEmitter;
pub use gemini::GeminiEmitter;
pub use windsurf::WindsurfEmitter;
