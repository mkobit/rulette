use clap::ValueEnum;
use schemars::JsonSchema;
use serde::Deserialize;

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
    IrJson,
    IrToml,
}

#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Deserialize, JsonSchema,
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
    AgentSkills,
    IrJson,
    IrToml,
    JsonSchema,
}
