use crate::agent_skills;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as HashMap;

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

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema, Eq)]
#[serde(untagged)]
pub enum TargetOverrides<T> {
    Wrapped {
        default: T,
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        overrides: std::collections::BTreeMap<String, T>,
    },
    Bare(T),
}

impl<T> TargetOverrides<T> {
    pub fn resolve(&self, target: &str) -> &T {
        match self {
            TargetOverrides::Bare(val) => val,
            TargetOverrides::Wrapped { default, overrides } => {
                let target_clean = target.trim().to_lowercase();

                // 1. Exact match (case/whitespace normalized)
                for (k, v) in overrides {
                    if k.trim().to_lowercase() == target_clean {
                        return v;
                    }
                }

                // 2. Tool family alias/prefix match (e.g. "cursor" for "cursor-mdc")
                if let Some(prefix) = target_clean.split(['-', '_']).next() {
                    if prefix != target_clean {
                        for (k, v) in overrides {
                            if k.trim().to_lowercase() == prefix {
                                return v;
                            }
                        }
                    }
                }

                // 3. Fallback to default
                default
            }
        }
    }
}

impl<T> From<T> for TargetOverrides<T> {
    fn from(val: T) -> Self {
        TargetOverrides::Bare(val)
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct RuleMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "rulette:activation", skip_serializing_if = "Option::is_none")]
    pub activation: Option<TargetOverrides<Activation>>,

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
    use super::*;

    #[test]
    fn test_target_overrides_deserialization_bare() {
        let yaml_str = r#"
mode:
  - always
"#;
        let parsed: TargetOverrides<Activation> =
            serde_yaml::from_str(yaml_str).expect("should parse bare activation");
        match &parsed {
            TargetOverrides::Bare(activation) => {
                assert_eq!(activation.mode, vec![ActivationMode::Always]);
                assert_eq!(activation.globs, None);
            }
            TargetOverrides::Wrapped { .. } => panic!("expected Bare variant"),
        }
        assert_eq!(
            parsed.resolve("cursor-mdc").mode,
            vec![ActivationMode::Always]
        );
    }

    #[test]
    fn test_target_overrides_deserialization_wrapped() {
        let yaml_str = r#"
default:
  mode:
    - manual
overrides:
  cursor-mdc:
    mode:
      - glob
    globs:
      - "**/*.rs"
  antigravity:
    mode:
      - model
    description: "activate for rust projects"
"#;
        let parsed: TargetOverrides<Activation> =
            serde_yaml::from_str(yaml_str).expect("should parse wrapped activation");

        match &parsed {
            TargetOverrides::Wrapped { default, overrides } => {
                assert_eq!(default.mode, vec![ActivationMode::Manual]);
                assert_eq!(overrides.len(), 2);
            }
            TargetOverrides::Bare(_) => panic!("expected Wrapped variant"),
        }

        // Exact match
        let cursor_res = parsed.resolve("cursor-mdc");
        assert_eq!(cursor_res.mode, vec![ActivationMode::Glob]);
        assert_eq!(cursor_res.globs, Some(vec!["**/*.rs".to_string()]));

        // Exact match for antigravity
        let agy_res = parsed.resolve("antigravity");
        assert_eq!(agy_res.mode, vec![ActivationMode::Model]);
        assert_eq!(
            agy_res.description.as_deref(),
            Some("activate for rust projects")
        );

        // Fallback to default
        let claude_res = parsed.resolve("claude");
        assert_eq!(claude_res.mode, vec![ActivationMode::Manual]);
        assert_eq!(claude_res.globs, None);
    }

    #[test]
    fn test_target_overrides_tool_alias_precedence() {
        let mut overrides = std::collections::BTreeMap::new();
        overrides.insert(
            "cursor".to_string(),
            Activation {
                mode: vec![ActivationMode::Always],
                globs: None,
                pattern: None,
                description: None,
            },
        );
        overrides.insert(
            "cursor-mdc".to_string(),
            Activation {
                mode: vec![ActivationMode::Glob],
                globs: Some(vec!["src/**".to_string()]),
                pattern: None,
                description: None,
            },
        );

        let wrapped = TargetOverrides::Wrapped {
            default: Activation {
                mode: vec![ActivationMode::Manual],
                globs: None,
                pattern: None,
                description: None,
            },
            overrides,
        };

        // Exact match wins over tool alias
        assert_eq!(
            wrapped.resolve("cursor-mdc").mode,
            vec![ActivationMode::Glob]
        );

        // Tool alias matches prefix for cursor-mcp
        assert_eq!(
            wrapped.resolve("cursor-mcp").mode,
            vec![ActivationMode::Always]
        );
        assert_eq!(
            wrapped.resolve("cursor_rules").mode,
            vec![ActivationMode::Always]
        );

        // Unrelated target falls back to default
        assert_eq!(wrapped.resolve("gemini").mode, vec![ActivationMode::Manual]);
    }

    #[test]
    fn test_target_overrides_normalization() {
        let mut overrides = std::collections::BTreeMap::new();
        overrides.insert(
            "  CURSOR-MDC  ".to_string(),
            Activation {
                mode: vec![ActivationMode::Always],
                globs: None,
                pattern: None,
                description: None,
            },
        );

        let wrapped = TargetOverrides::Wrapped {
            default: Activation {
                mode: vec![ActivationMode::Manual],
                globs: None,
                pattern: None,
                description: None,
            },
            overrides,
        };

        assert_eq!(
            wrapped.resolve("cursor-mdc").mode,
            vec![ActivationMode::Always]
        );
        assert_eq!(
            wrapped.resolve("  Cursor-Mdc  ").mode,
            vec![ActivationMode::Always]
        );
    }

    #[test]
    fn test_target_overrides_serialization_bare() {
        let activation = TargetOverrides::Bare(Activation {
            mode: vec![ActivationMode::Always],
            globs: None,
            pattern: None,
            description: None,
        });

        let json = serde_json::to_string(&activation).expect("serialize json");
        assert_eq!(json, r#"{"mode":["always"]}"#);
    }

    #[test]
    fn test_target_overrides_serialization_wrapped() {
        let mut overrides = std::collections::BTreeMap::new();
        overrides.insert(
            "cursor-mdc".to_string(),
            Activation {
                mode: vec![ActivationMode::Glob],
                globs: Some(vec!["*.md".to_string()]),
                pattern: None,
                description: None,
            },
        );

        let activation = TargetOverrides::Wrapped {
            default: Activation {
                mode: vec![ActivationMode::Manual],
                globs: None,
                pattern: None,
                description: None,
            },
            overrides,
        };

        let json = serde_json::to_string(&activation).expect("serialize json");
        assert!(json.contains(r#""default":{"mode":["manual"]}"#));
        assert!(json.contains(r#""cursor-mdc":{"mode":["glob"],"globs":["*.md"]}"#));
    }
}

#[cfg(test)]
mod generated_schema_tests {
    use super::*;
    #[test]
    fn test_schema_generation_for_new_entities() {
        let _ = schemars::schema_for!(Hook);
        let _ = schemars::schema_for!(Agent);
        let _ = schemars::schema_for!(Permissions);
        let _ = schemars::schema_for!(Activation);
        let _ = schemars::schema_for!(TargetOverrides<Activation>);
    }
}
