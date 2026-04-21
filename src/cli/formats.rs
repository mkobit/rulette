use clap::ValueEnum;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputFormat {
    Claude,
    ClaudeSettings,
    CursorMdc,
    Codex,
    Windsurf,
    Copilot,
    Gemini,
    AgentSkills,
    IrJson,
    IrToml,
}
