use crate::RuletteDocument;
use anyhow::Result;
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub mod agent_skills;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod cursor_mcp;
pub mod gemini;
pub mod windsurf;

pub trait Emitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>>;
}

/// Extra-map keys injected internally by Rulette for its own bookkeeping
/// (e.g. `rulette:source_file` for identity-collision diagnostics). These
/// must never be serialized into a target format's emitted output -- only
/// the IR JSON/TOML backends, which are the lossless native representation,
/// should ever see them. Leaking them would also break determinism, since
/// the value is a local filesystem path that varies by machine/checkout.
pub const INTERNAL_EXTRA_KEYS: &[&str] = &["rulette:source_file"];

pub fn is_internal_extra_key(key: &str) -> bool {
    INTERNAL_EXTRA_KEYS.contains(&key)
}

pub use agent_skills::AgentSkillsEmitter;
pub use claude::ClaudeEmitter;
pub use codex::CodexEmitter;
pub use copilot::CopilotEmitter;
pub use cursor::CursorEmitter;
pub use cursor_mcp::CursorMcpEmitter;
pub use gemini::GeminiEmitter;
pub use windsurf::WindsurfEmitter;
