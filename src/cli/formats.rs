use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum InputFormat {
    Auto,
    SkillMd,
    AgentSkills,
    Claude,
    ClaudeSettings,
    CursorMdc,
    CursorLegacy,
    CursorMcp,
    Codex,
    Windsurf,
    Copilot,
    Gemini,
    Antigravity,
    IrJson,
    IrToml,
}

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    ValueEnum,
    Debug,
    Deserialize,
    Serialize,
    JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Claude,
    CursorMdc,
    CursorMcp,
    Codex,
    Windsurf,
    Copilot,
    Gemini,
    Antigravity,
    AgentSkills,
    IrJson,
    IrToml,
    JsonSchema,
    /// Scaffold-only target: `transform` writes a transform-config manifest
    /// instead of a real tool output; `inspect` rejects it (see
    /// `src/cli/commands/inspect.rs`).
    TransformConfig,
}
