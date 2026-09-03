use crate::parsers::{DecoderSelection, NativeFrontend};
use clap::ValueEnum;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum InputFormat {
    Auto,
    Claude,
    CursorMdc,
    Codex,
    Antigravity,
    Opencode,
    GraphJson,
    GraphToml,
}

impl From<InputFormat> for DecoderSelection {
    fn from(value: InputFormat) -> Self {
        match value {
            InputFormat::Auto => Self::Auto,
            InputFormat::Codex => Self::Native(NativeFrontend::Codex),
            InputFormat::Claude => Self::Native(NativeFrontend::Claude),
            InputFormat::CursorMdc => Self::Native(NativeFrontend::CursorMdc),
            InputFormat::Opencode => Self::Native(NativeFrontend::Opencode),
            InputFormat::Antigravity => Self::Native(NativeFrontend::Antigravity),
            InputFormat::GraphJson => Self::GraphJson,
            InputFormat::GraphToml => Self::GraphToml,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputFormat;

    #[test]
    fn accepts_only_core_graph_frontends() {
        for format in [
            "auto",
            "codex",
            "claude",
            "cursor-mdc",
            "opencode",
            "antigravity",
            "graph-json",
            "graph-toml",
        ] {
            serde_json::from_str::<InputFormat>(&format!("\"{format}\""))
                .expect("core frontend remains selectable");
        }

        for format in [
            "skill-md",
            "agent-skills",
            "claude-settings",
            "cursor-legacy",
            "cursor-mcp",
            "windsurf",
            "copilot",
            "gemini",
            "ir-json",
            "ir-toml",
        ] {
            assert!(
                serde_json::from_str::<InputFormat>(&format!("\"{format}\"")).is_err(),
                "legacy frontend `{format}` must not remain selectable"
            );
        }
    }
}
