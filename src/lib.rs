pub mod agent_skills;
pub mod backend;
pub mod claude;
pub mod cli;
pub mod codex;
pub mod cursor;
pub mod frontend;
pub mod gemini;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    Always,
    Glob,
    Pattern,
    Manual,
    Model,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Activation {
    pub mode: Vec<ActivationMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub globs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema, Eq)]
pub enum HookEventKind {
    PreToolUse,
    PostToolUse,
    Notification,
    Stop,
    SubagentStop,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HookEvent {
    pub event: HookEventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolAccessRule {
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuletteDocument {
    #[serde(default = "default_ir_version")]
    pub ir_version: String,
    pub entities: Vec<Entity>,
}

fn default_ir_version() -> String {
    "0.1".to_string()
}

impl Default for RuletteDocument {
    fn default() -> Self {
        Self {
            ir_version: "0.1".to_string(),
            entities: Vec::new(),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum Entity {
    #[serde(rename = "rule")]
    Rule(Rule),
    #[serde(rename = "skill")]
    Skill(agent_skills::Skill),
    #[serde(rename = "mcp-server")]
    McpServer(McpServer),
    #[serde(rename = "hook")]
    Hook(Hook),
    #[serde(rename = "agent")]
    Agent(Agent),
    #[serde(rename = "permissions")]
    Permissions(Permissions),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpServer {
    pub metadata: McpServerMetadata,
    pub config: McpServerConfig,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpServerMetadata {
    pub name: String,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    pub metadata: RuleMetadata,
    pub body: String,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct RuleMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "rulette:activation", skip_serializing_if = "Option::is_none")]
    pub activation: Option<Activation>,

    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Hook {
    pub metadata: HookMetadata,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HookMetadata {
    pub name: String,
    #[serde(rename = "rulette:hook-event", skip_serializing_if = "Option::is_none")]
    pub hook_event: Option<HookEvent>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Agent {
    pub metadata: AgentMetadata,
    pub body: String,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentMetadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        rename = "rulette:tool-access",
        skip_serializing_if = "Option::is_none"
    )]
    pub tool_access: Option<Vec<ToolAccessRule>>,
    #[serde(
        rename = "rulette:agent-tools",
        skip_serializing_if = "Option::is_none"
    )]
    pub agent_tools: Option<Vec<String>>,
    #[serde(rename = "rulette:models", skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Permissions {
    pub metadata: PermissionsMetadata,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionsMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        rename = "rulette:tool-access",
        skip_serializing_if = "Option::is_none"
    )]
    pub tool_access: Option<Vec<ToolAccessRule>>,
    #[serde(
        rename = "rulette:settings-overrides",
        skip_serializing_if = "Option::is_none"
    )]
    pub settings_overrides: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[cfg(test)]
mod generated_schema_tests {
    #[test]
    fn test_schema_generation_for_new_entities() {
        let _ = schemars::schema_for!(crate::Hook);
        let _ = schemars::schema_for!(crate::Agent);
        let _ = schemars::schema_for!(crate::Permissions);
        let _ = schemars::schema_for!(crate::Activation);
    }
}
