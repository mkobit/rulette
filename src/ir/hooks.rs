use std::collections::{BTreeMap, HashSet};

use anyhow::{bail, Result};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Type alias for canonical hook events across tooling harnesses.
pub type CanonicalHookEvent = HookEventKind;

/// Canonical hook event taxonomy covering agent and tool lifecycles.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum HookEventKind {
    /// Triggers before any tool invocation.
    PreToolUse,
    /// Triggers after any tool invocation completes.
    PostToolUse,
    /// Triggers specifically before a shell or terminal command executes.
    BeforeShellExecution,
    /// Triggers before a user prompt submits to the model.
    PromptSubmit,
    /// Triggers when the model completes an interaction turn.
    TurnStop,
    /// Triggers when an agent session initializes.
    SessionStart,
    /// Triggers when an agent session terminates.
    SessionStop,
    /// Triggers when a child subagent spawns.
    SubagentStart,
    /// Triggers when a child subagent completes and returns control.
    SubagentStop,
    /// Triggers when an agent emits a user-facing notification or alert.
    Notification,
}

impl HookEventKind {
    /// Returns the canonical kebab-case name for this hook event.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "pre-tool-use",
            Self::PostToolUse => "post-tool-use",
            Self::BeforeShellExecution => "before-shell-execution",
            Self::PromptSubmit => "prompt-submit",
            Self::TurnStop => "turn-stop",
            Self::SessionStart => "session-start",
            Self::SessionStop => "session-stop",
            Self::SubagentStart => "subagent-start",
            Self::SubagentStop => "subagent-stop",
            Self::Notification => "notification",
        }
    }
}

/// Pattern matcher applied to filter hook execution.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case", tag = "type", content = "value")]
pub enum HookMatcher {
    /// Matches all invocations unconditionally.
    All,
    /// Matches exact tool names (for example, "run_command" or "Bash").
    Tool(String),
    /// Matches tool names via a regular expression.
    Regex(String),
    /// Matches shell command prefixes.
    CommandPrefix(Vec<String>),
}

impl HookMatcher {
    /// Constructs a tool name matcher.
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool(name.into())
    }

    /// Constructs a regex pattern matcher.
    pub fn regex(pattern: impl Into<String>) -> Self {
        Self::Regex(pattern.into())
    }

    /// Constructs a command prefix matcher.
    pub fn command_prefix(prefix: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::CommandPrefix(prefix.into_iter().map(Into::into).collect())
    }

    /// Validates pattern matcher integrity.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::All => Ok(()),
            Self::Tool(tool) => {
                if tool.trim().is_empty() {
                    bail!("tool matcher pattern cannot be empty");
                }
                Ok(())
            }
            Self::Regex(pattern) => {
                if pattern.trim().is_empty() {
                    bail!("regex matcher pattern cannot be empty");
                }
                Regex::new(pattern).map_err(|err| {
                    anyhow::anyhow!("invalid regular expression `{pattern}`: {err}")
                })?;
                Ok(())
            }
            Self::CommandPrefix(prefix) => {
                if prefix.is_empty() {
                    bail!("command prefix matcher cannot be empty");
                }
                if prefix.iter().any(|part| part.trim().is_empty()) {
                    bail!("command prefix matcher contains empty components");
                }
                Ok(())
            }
        }
    }
}

/// Action performed when a hook triggers.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum HookAction {
    /// Executes a shell command with JSON payload on standard input.
    Command {
        command: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            alias = "timeout-seconds"
        )]
        timeout_seconds: Option<u32>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            alias = "status-message"
        )]
        status_message: Option<String>,
    },
    /// An unsupported or platform-specific action (for example, JavaScript mods).
    Unsupported { kind: String, description: String },
}

impl HookAction {
    /// Constructs a shell command hook action without optional timeout or status message.
    pub fn command(command: impl Into<String>) -> Self {
        Self::Command {
            command: command.into(),
            timeout_seconds: None,
            status_message: None,
        }
    }

    /// Constructs a shell command hook action with timeout.
    pub fn command_with_timeout(command: impl Into<String>, timeout_seconds: u32) -> Self {
        Self::Command {
            command: command.into(),
            timeout_seconds: Some(timeout_seconds),
            status_message: None,
        }
    }

    /// Constructs an unsupported hook action placeholder.
    pub fn unsupported(kind: impl Into<String>, description: impl Into<String>) -> Self {
        Self::Unsupported {
            kind: kind.into(),
            description: description.into(),
        }
    }

    /// Validates action parameters.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Command {
                command,
                timeout_seconds,
                status_message,
            } => {
                if command.trim().is_empty() {
                    bail!("hook command cannot be empty");
                }
                if let Some(timeout) = timeout_seconds {
                    if *timeout == 0 {
                        bail!("timeout_seconds must be positive");
                    }
                }
                if let Some(msg) = status_message {
                    if msg.trim().is_empty() {
                        bail!("status_message cannot be empty if specified");
                    }
                }
                Ok(())
            }
            Self::Unsupported { kind, description } => {
                if kind.trim().is_empty() {
                    bail!("unsupported hook action kind cannot be empty");
                }
                if description.trim().is_empty() {
                    bail!("unsupported hook action description cannot be empty");
                }
                Ok(())
            }
        }
    }
}

/// Scope and authority level for hook execution.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum HookScope {
    /// Active for the local workspace or project.
    #[default]
    Project,
    /// Active for the current user across workspaces.
    User,
    /// Active as part of a packaged plugin bundle.
    Plugin,
    /// Active only during execution of a specific skill turn.
    SkillTurn,
    /// Mandated by enterprise administrative policy.
    Enterprise,
}

impl HookScope {
    /// Returns the authority tier rank for ordering.
    pub const fn rank(self) -> u8 {
        match self {
            Self::SkillTurn => 1,
            Self::Plugin => 2,
            Self::User => 3,
            Self::Project => 4,
            Self::Enterprise => 5,
        }
    }
}

fn default_fail_closed() -> bool {
    true
}

fn default_hook_scope() -> HookScope {
    HookScope::Project
}

/// Individual canonical hook definition.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct HookDefinition {
    pub id: String,
    pub event: HookEventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<HookMatcher>,
    pub action: HookAction,
    #[serde(default = "default_hook_scope")]
    pub scope: HookScope,
    #[serde(default = "default_fail_closed", alias = "fail-closed")]
    pub fail_closed: bool,
    #[serde(default)]
    pub priority: i32,
}

impl HookDefinition {
    /// Creates a new hook definition with default scope, fail-closed policy, and priority 0.
    pub fn new(id: impl Into<String>, event: HookEventKind, action: HookAction) -> Self {
        Self {
            id: id.into(),
            event,
            matcher: None,
            action,
            scope: HookScope::Project,
            fail_closed: true,
            priority: 0,
        }
    }

    /// Sets pattern matcher.
    pub fn with_matcher(mut self, matcher: HookMatcher) -> Self {
        self.matcher = Some(matcher);
        self
    }

    /// Sets scope.
    pub fn with_scope(mut self, scope: HookScope) -> Self {
        self.scope = scope;
        self
    }

    /// Sets fail-closed behavior.
    pub fn with_fail_closed(mut self, fail_closed: bool) -> Self {
        self.fail_closed = fail_closed;
        self
    }

    /// Sets priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Validates the hook definition.
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            bail!("hook id cannot be empty");
        }
        if self.id.chars().any(char::is_control) {
            bail!("hook id cannot contain control characters");
        }
        if let Some(ref matcher) = self.matcher {
            matcher.validate()?;
        }
        self.action.validate()?;
        Ok(())
    }
}

/// Target-specific overrides or bare hook policy.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TargetHookPolicy {
    Wrapped {
        default: Vec<HookDefinition>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        overrides: BTreeMap<String, Vec<HookDefinition>>,
    },
    Bare(Vec<HookDefinition>),
}

impl TargetHookPolicy {
    /// Creates a bare hook policy with no target overrides.
    pub fn bare(hooks: Vec<HookDefinition>) -> Self {
        Self::Bare(hooks)
    }

    /// Creates a wrapped hook policy with a default set and target overrides.
    pub fn wrapped(
        default: Vec<HookDefinition>,
        overrides: BTreeMap<String, Vec<HookDefinition>>,
    ) -> Self {
        Self::Wrapped { default, overrides }
    }

    /// Resolves active hook definitions for the given target identifier.
    ///
    /// Resolution attempts:
    /// 1. Exact target match in `overrides`.
    /// 2. Target prefix match before first hyphen (e.g. `cursor-fast` -> `cursor`).
    /// 3. Fallback to `default` (or bare list).
    pub fn resolve(&self, target: &str) -> &[HookDefinition] {
        match self {
            Self::Bare(hooks) => hooks.as_slice(),
            Self::Wrapped { default, overrides } => {
                if let Some(hooks) = overrides.get(target) {
                    return hooks.as_slice();
                }
                if let Some((prefix, _)) = target.split_once('-') {
                    if let Some(hooks) = overrides.get(prefix) {
                        return hooks.as_slice();
                    }
                }
                default.as_slice()
            }
        }
    }

    /// Returns the default hook definitions.
    pub fn default_hooks(&self) -> &[HookDefinition] {
        match self {
            Self::Bare(hooks) => hooks.as_slice(),
            Self::Wrapped { default, .. } => default.as_slice(),
        }
    }

    /// Validates all hook definitions and verifies that hook IDs are unique.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Bare(hooks) => validate_unique_hook_ids(hooks),
            Self::Wrapped { default, overrides } => {
                validate_unique_hook_ids(default)?;
                for (target, hooks) in overrides {
                    if target.trim().is_empty() {
                        bail!("target override name cannot be empty");
                    }
                    validate_unique_hook_ids(hooks)?;
                }
                Ok(())
            }
        }
    }
}

fn validate_unique_hook_ids(hooks: &[HookDefinition]) -> Result<()> {
    let mut seen = HashSet::new();
    for hook in hooks {
        hook.validate()?;
        if !seen.insert(&hook.id) {
            bail!("duplicate hook id `{}` in hook policy", hook.id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_hook_events_roundtrip_json_and_toml() {
        let events = [
            (HookEventKind::PreToolUse, "\"pre-tool-use\""),
            (HookEventKind::PostToolUse, "\"post-tool-use\""),
            (
                HookEventKind::BeforeShellExecution,
                "\"before-shell-execution\"",
            ),
            (HookEventKind::PromptSubmit, "\"prompt-submit\""),
            (HookEventKind::TurnStop, "\"turn-stop\""),
            (HookEventKind::SessionStart, "\"session-start\""),
            (HookEventKind::SessionStop, "\"session-stop\""),
            (HookEventKind::SubagentStart, "\"subagent-start\""),
            (HookEventKind::SubagentStop, "\"subagent-stop\""),
            (HookEventKind::Notification, "\"notification\""),
        ];

        for (event, expected_json) in events {
            assert_eq!(format!("\"{}\"", event.as_str()), expected_json);

            let json = serde_json::to_string(&event).expect("json serialization");
            assert_eq!(json, expected_json);

            let deserialized: HookEventKind =
                serde_json::from_str(&json).expect("json deserialization");
            assert_eq!(deserialized, event);

            #[derive(Debug, PartialEq, Serialize, Deserialize)]
            struct EventWrapper {
                event: HookEventKind,
            }

            let wrapper = EventWrapper { event };
            let toml_str = toml::to_string(&wrapper).expect("toml serialization");
            let toml_deserialized: EventWrapper =
                toml::from_str(&toml_str).expect("toml deserialization");
            assert_eq!(toml_deserialized, wrapper);
        }
    }

    #[test]
    fn test_canonical_matchers_roundtrip_json_and_toml() {
        let matchers = vec![
            HookMatcher::All,
            HookMatcher::tool("Bash"),
            HookMatcher::regex("^git.*"),
            HookMatcher::command_prefix(vec!["git", "commit"]),
        ];

        for matcher in matchers {
            matcher.validate().expect("matcher should validate");

            let json = serde_json::to_string(&matcher).expect("json serialization");
            let deserialized: HookMatcher =
                serde_json::from_str(&json).expect("json deserialization");
            assert_eq!(deserialized, matcher);

            #[derive(Debug, PartialEq, Serialize, Deserialize)]
            struct MatcherWrapper {
                matcher: HookMatcher,
            }

            let wrapper = MatcherWrapper { matcher };
            let toml_str = toml::to_string(&wrapper).expect("toml serialization");
            let toml_deserialized: MatcherWrapper =
                toml::from_str(&toml_str).expect("toml deserialization");
            assert_eq!(toml_deserialized, wrapper);
        }
    }

    #[test]
    fn test_canonical_actions_roundtrip_json_and_toml() {
        let actions = vec![
            HookAction::command("echo allow"),
            HookAction::Command {
                command: "echo test".to_string(),
                timeout_seconds: Some(30),
                status_message: Some("Validating shell command".to_string()),
            },
            HookAction::unsupported("javascript", "Custom JavaScript in-process hook"),
        ];

        for action in actions {
            action.validate().expect("action should validate");

            let json = serde_json::to_string(&action).expect("json serialization");
            let deserialized: HookAction =
                serde_json::from_str(&json).expect("json deserialization");
            assert_eq!(deserialized, action);

            #[derive(Debug, PartialEq, Serialize, Deserialize)]
            struct ActionWrapper {
                action: HookAction,
            }

            let wrapper = ActionWrapper { action };
            let toml_str = toml::to_string(&wrapper).expect("toml serialization");
            let toml_deserialized: ActionWrapper =
                toml::from_str(&toml_str).expect("toml deserialization");
            assert_eq!(toml_deserialized, wrapper);
        }
    }

    #[test]
    fn test_hook_scopes_ordering_and_defaults() {
        assert_eq!(HookScope::default(), HookScope::Project);
        assert!(HookScope::SkillTurn.rank() < HookScope::Plugin.rank());
        assert!(HookScope::Plugin.rank() < HookScope::User.rank());
        assert!(HookScope::User.rank() < HookScope::Project.rank());
        assert!(HookScope::Project.rank() < HookScope::Enterprise.rank());

        let scopes = [
            HookScope::Project,
            HookScope::User,
            HookScope::Plugin,
            HookScope::SkillTurn,
            HookScope::Enterprise,
        ];

        for scope in scopes {
            let json = serde_json::to_string(&scope).expect("json serialize");
            let deserialized: HookScope = serde_json::from_str(&json).expect("json deserialize");
            assert_eq!(deserialized, scope);
        }
    }

    #[test]
    fn test_hook_definition_roundtrip_json_and_toml() {
        let hook = HookDefinition::new(
            "security-guard",
            HookEventKind::BeforeShellExecution,
            HookAction::Command {
                command: "rulette hook-eval --event before-shell-execution".to_string(),
                timeout_seconds: Some(15),
                status_message: Some("Evaluating command security policy".to_string()),
            },
        )
        .with_matcher(HookMatcher::command_prefix(vec!["rm", "rf"]))
        .with_scope(HookScope::Enterprise)
        .with_fail_closed(true)
        .with_priority(100);

        hook.validate().expect("hook should validate");

        let json = serde_json::to_string_pretty(&hook).expect("json serialization");
        let deserialized: HookDefinition =
            serde_json::from_str(&json).expect("json deserialization");
        assert_eq!(deserialized, hook);

        let toml_str = toml::to_string(&hook).expect("toml serialization");
        let toml_deserialized: HookDefinition =
            toml::from_str(&toml_str).expect("toml deserialization");
        assert_eq!(toml_deserialized, hook);
    }

    #[test]
    fn test_hook_definition_defaults() {
        let minimal_json = r#"{
            "id": "minimal-hook",
            "event": "pre-tool-use",
            "action": {
                "type": "command",
                "command": "check.sh"
            }
        }"#;

        let hook: HookDefinition =
            serde_json::from_str(minimal_json).expect("deserialize minimal hook");
        assert_eq!(hook.id, "minimal-hook");
        assert_eq!(hook.event, HookEventKind::PreToolUse);
        assert_eq!(hook.matcher, None);
        assert_eq!(hook.scope, HookScope::Project);
        assert!(hook.fail_closed);
        assert_eq!(hook.priority, 0);
        hook.validate().expect("minimal hook validates");
    }

    #[test]
    fn test_target_hook_policy_roundtrip_and_resolution() {
        let default_hook = HookDefinition::new(
            "default-gate",
            HookEventKind::PreToolUse,
            HookAction::command("default-check.sh"),
        );
        let cursor_hook = HookDefinition::new(
            "cursor-gate",
            HookEventKind::BeforeShellExecution,
            HookAction::command("cursor-check.sh"),
        );
        let antigravity_hook = HookDefinition::new(
            "ag-gate",
            HookEventKind::SessionStart,
            HookAction::command("ag-check.sh"),
        );

        let mut overrides = BTreeMap::new();
        overrides.insert("cursor".to_string(), vec![cursor_hook.clone()]);
        overrides.insert("antigravity".to_string(), vec![antigravity_hook.clone()]);

        let policy = TargetHookPolicy::wrapped(vec![default_hook.clone()], overrides);
        policy.validate().expect("policy should validate");

        // Exact match
        assert_eq!(policy.resolve("cursor"), &[cursor_hook.clone()]);
        assert_eq!(policy.resolve("antigravity"), &[antigravity_hook.clone()]);

        // Prefix match
        assert_eq!(policy.resolve("cursor-small"), &[cursor_hook]);

        // Fallback to default
        assert_eq!(policy.resolve("claude"), &[default_hook.clone()]);
        assert_eq!(policy.resolve("codex"), &[default_hook.clone()]);

        // JSON roundtrip
        let json = serde_json::to_string(&policy).expect("serialize policy");
        let deserialized: TargetHookPolicy =
            serde_json::from_str(&json).expect("deserialize policy");
        assert_eq!(deserialized, policy);

        // Bare policy
        let bare = TargetHookPolicy::bare(vec![default_hook.clone()]);
        bare.validate().expect("bare policy should validate");
        assert_eq!(bare.resolve("cursor"), &[default_hook.clone()]);
        assert_eq!(bare.resolve("antigravity"), &[default_hook]);

        let bare_json = serde_json::to_string(&bare).expect("serialize bare");
        let bare_deserialized: TargetHookPolicy =
            serde_json::from_str(&bare_json).expect("deserialize bare");
        assert_eq!(bare_deserialized, bare);
    }

    #[test]
    fn test_fail_closed_validation_empty_id() {
        let hook = HookDefinition::new(
            "   ",
            HookEventKind::PreToolUse,
            HookAction::command("run.sh"),
        );
        assert!(hook.validate().is_err());
    }

    #[test]
    fn test_fail_closed_validation_control_char_id() {
        let hook = HookDefinition::new(
            "hook\nid",
            HookEventKind::PreToolUse,
            HookAction::command("run.sh"),
        );
        assert!(hook.validate().is_err());
    }

    #[test]
    fn test_fail_closed_validation_invalid_regex() {
        let matcher = HookMatcher::regex("[unclosed-group");
        assert!(matcher.validate().is_err());

        let empty_matcher = HookMatcher::regex("   ");
        assert!(empty_matcher.validate().is_err());
    }

    #[test]
    fn test_fail_closed_validation_empty_tool_matcher() {
        let matcher = HookMatcher::tool("  ");
        assert!(matcher.validate().is_err());
    }

    #[test]
    fn test_fail_closed_validation_empty_command_prefix() {
        let empty_prefix = HookMatcher::command_prefix(Vec::<String>::new());
        assert!(empty_prefix.validate().is_err());

        let blank_component = HookMatcher::command_prefix(vec!["git", "  "]);
        assert!(blank_component.validate().is_err());
    }

    #[test]
    fn test_fail_closed_validation_empty_command() {
        let action = HookAction::command("  ");
        assert!(action.validate().is_err());
    }

    #[test]
    fn test_fail_closed_validation_zero_timeout() {
        let action = HookAction::Command {
            command: "echo 1".to_string(),
            timeout_seconds: Some(0),
            status_message: None,
        };
        assert!(action.validate().is_err());
    }

    #[test]
    fn test_fail_closed_validation_negative_timeout_deserialization() {
        let json = r#"{
            "type": "command",
            "command": "echo test",
            "timeout_seconds": -5
        }"#;
        let res: Result<HookAction, _> = serde_json::from_str(json);
        assert!(res.is_err());
    }

    #[test]
    fn test_fail_closed_validation_duplicate_hook_ids() {
        let hook1 = HookDefinition::new(
            "dup-id",
            HookEventKind::PreToolUse,
            HookAction::command("echo 1"),
        );
        let hook2 = HookDefinition::new(
            "dup-id",
            HookEventKind::PostToolUse,
            HookAction::command("echo 2"),
        );

        let bare = TargetHookPolicy::bare(vec![hook1.clone(), hook2.clone()]);
        assert!(bare.validate().is_err());

        let mut overrides = BTreeMap::new();
        overrides.insert("cursor".to_string(), vec![hook1, hook2]);
        let wrapped = TargetHookPolicy::wrapped(vec![], overrides);
        assert!(wrapped.validate().is_err());
    }

    #[test]
    fn test_fail_closed_unknown_event_rejected() {
        let json = r#"{
            "id": "test",
            "event": "unknown-event",
            "action": {
                "type": "command",
                "command": "test.sh"
            }
        }"#;
        let res: Result<HookDefinition, _> = serde_json::from_str(json);
        assert!(res.is_err());
    }

    #[test]
    fn test_fail_closed_unknown_fields_rejected() {
        let json = r#"{
            "id": "test",
            "event": "pre-tool-use",
            "action": {
                "type": "command",
                "command": "test.sh"
            },
            "unsupported_extra_field": true
        }"#;
        let res: Result<HookDefinition, _> = serde_json::from_str(json);
        assert!(res.is_err());
    }

    #[test]
    fn test_json_schema_generation() {
        let hook_schema = schemars::schema_for!(HookDefinition);
        let schema_json = serde_json::to_string(&hook_schema).expect("schema serialization");
        assert!(schema_json.contains("HookDefinition"));

        let policy_schema = schemars::schema_for!(TargetHookPolicy);
        let policy_json =
            serde_json::to_string(&policy_schema).expect("policy schema serialization");
        assert!(policy_json.contains("TargetHookPolicy"));
    }
}
