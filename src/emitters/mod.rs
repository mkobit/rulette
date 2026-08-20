use crate::{Entity, RuletteDocument};
use anyhow::Result;
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub mod agent_skills;
pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod cursor_mcp;
pub mod gemini;
pub mod windsurf;

/// Whether an entity kind survives conversion to a target format, computed
/// per `RuletteDocument` (not a static per-target table) since fidelity can
/// depend on which fields are actually populated. Ranked `Supported <
/// Lossy < Dropped` (declaration order) so aggregating multiple instances of
/// the same kind can simply take the maximum -- see `aggregate_capabilities`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CoverageStatus {
    Supported,
    Lossy,
    Dropped,
}

/// One row of a target's capability report: how a given entity kind fared,
/// and -- for anything other than `Supported` -- why, using the same message
/// text the emitter's own `eprintln!`/error path uses, so a `--coverage`
/// consumer never needs a second call to learn what was lost.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapabilityEntry {
    pub entity_kind: String,
    pub status: CoverageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CapabilityEntry {
    /// A fully-represented entity: no loss, no reason to report.
    pub fn supported(entity: &Entity) -> Self {
        Self {
            entity_kind: entity_kind_kebab(entity).to_string(),
            status: CoverageStatus::Supported,
            reason: None,
        }
    }

    /// A `Lossy` or `Dropped` entity, carrying the same message text the
    /// emitter's own `eprintln!`/error path reports for this condition.
    pub fn lossy_or_dropped(
        entity: &Entity,
        status: CoverageStatus,
        reason: impl Into<String>,
    ) -> Self {
        debug_assert_ne!(
            status,
            CoverageStatus::Supported,
            "use CapabilityEntry::supported for the Supported case"
        );
        Self {
            entity_kind: entity_kind_kebab(entity).to_string(),
            status,
            reason: Some(reason.into()),
        }
    }
}

pub trait Emitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>>;

    /// Computes this target's capability report for `doc` without performing
    /// a real emission. Implementations must derive each entity's status from
    /// the same determination logic `emit()` uses to decide whether to warn
    /// or error, so the two can never silently disagree (enforced by a
    /// parity test, not just convention -- see openspec/changes/coverage-reporting).
    fn capabilities(&self, doc: &RuletteDocument) -> Vec<CapabilityEntry>;
}

/// Kebab-case entity-kind label matching the IR's `#[serde(tag = "kind")]`
/// values (as seen in `ir.json` output), used for `CapabilityEntry::entity_kind`
/// and coverage JSON output -- distinct from the PascalCase labels some
/// emitters use in human-readable warning text.
pub fn entity_kind_kebab(entity: &Entity) -> &'static str {
    match entity {
        Entity::Rule(_) => "rule",
        Entity::Skill(_) => "skill",
        Entity::McpServer(_) => "mcp-server",
        Entity::Hook(_) => "hook",
        Entity::Agent(_) => "agent",
        Entity::Permissions(_) => "permissions",
    }
}

/// Rolls up per-entity-instance capability determinations into one entry per
/// entity kind, keeping the worst status seen (`Dropped` > `Lossy` >
/// `Supported`) and that entry's reason. See coverage-reporting design.md
/// Decision 1b: a cell that hid one Dropped instance behind a Supported
/// summary would be a false-clean signal for a `--coverage --strict` CI gate.
pub fn aggregate_capabilities(raw: Vec<CapabilityEntry>) -> Vec<CapabilityEntry> {
    let mut by_kind: HashMap<String, CapabilityEntry> = HashMap::new();
    for entry in raw {
        by_kind
            .entry(entry.entity_kind.clone())
            .and_modify(|existing| {
                if entry.status > existing.status {
                    *existing = entry.clone();
                }
            })
            .or_insert(entry);
    }
    by_kind.into_values().collect()
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
pub use antigravity::AntigravityEmitter;
pub use claude::ClaudeEmitter;
pub use codex::CodexEmitter;
pub use copilot::CopilotEmitter;
pub use cursor::CursorEmitter;
pub use cursor_mcp::CursorMcpEmitter;
pub use gemini::GeminiEmitter;
pub use windsurf::WindsurfEmitter;

#[cfg(test)]
mod capabilities_parity_tests {
    use super::*;
    use crate::agent_skills::{Skill, SkillMetadata};
    use crate::{
        Activation, ActivationMode, Agent, AgentMetadata, Hook, HookEvent, HookEventKind,
        HookMetadata, McpServer, McpServerConfig, McpServerMetadata, Permissions,
        PermissionsMetadata, Rule, RuleMetadata,
    };

    /// One representative, non-degenerate entity per kind. Populated (not
    /// empty/default) so that emitters whose fidelity depends on content --
    /// e.g. Claude's Hook/Permissions branches, which contribute nothing and
    /// warn nothing for a truly empty entity -- exercise their real
    /// "does this produce output" path rather than a hollow edge case.
    fn one_of_every_kind() -> Vec<Entity> {
        vec![
            Entity::Rule(Rule {
                metadata: RuleMetadata {
                    description: Some("A test rule".to_string()),
                    activation: Some(
                        Activation {
                            mode: vec![ActivationMode::Glob],
                            globs: Some(vec!["**/*.rs".to_string()]),
                            pattern: None,
                            description: None,
                        }
                        .into(),
                    ),
                    extra: HashMap::new(),
                },
                body: "Test rule body.".to_string(),
            }),
            Entity::Skill(Skill {
                metadata: SkillMetadata {
                    name: "test-skill".to_string(),
                    description: "A test skill".to_string(),
                    version: Some("1.0.0".to_string()),
                    license: None,
                    compatibility: None,
                    metadata: std::collections::BTreeMap::new(),
                    allowed_tools: None,
                    extra: HashMap::new(),
                },
                body: "Test skill body.".to_string(),
            }),
            Entity::McpServer(McpServer {
                metadata: McpServerMetadata {
                    name: "test-server".to_string(),
                    extra: HashMap::new(),
                },
                config: McpServerConfig {
                    command: "echo".to_string(),
                    args: vec!["hello".to_string()],
                    env: HashMap::new(),
                },
            }),
            Entity::Hook(Hook {
                metadata: HookMetadata {
                    name: "PreToolUse".to_string(),
                    hook_event: Some(HookEvent {
                        event: HookEventKind::PreToolUse,
                        matcher: None,
                        command: Some("echo test".to_string()),
                    }),
                    extra: HashMap::new(),
                },
            }),
            Entity::Agent(Agent {
                metadata: AgentMetadata {
                    name: "test-agent".to_string(),
                    description: Some("A test agent".to_string()),
                    tool_access: None,
                    agent_tools: None,
                    models: None,
                    extra: HashMap::new(),
                },
                body: "Test agent body.".to_string(),
            }),
            Entity::Permissions(Permissions {
                metadata: PermissionsMetadata {
                    name: Some("test-perms".to_string()),
                    tool_access: None,
                    settings_overrides: None,
                    extra: {
                        let mut extra = HashMap::new();
                        extra.insert("ask".to_string(), serde_json::json!(["Bash"]));
                        extra
                    },
                },
            }),
        ]
    }

    fn single_entity_doc(entity: Entity) -> RuletteDocument {
        RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: vec![entity],
        }
    }

    /// For every emitter and every entity kind, `capabilities()`'s status
    /// must agree with what `emit(doc, strict=true)` actually does for a
    /// document containing just that one entity: Lossy/Dropped implies
    /// `emit(strict=true)` errors, Supported implies it succeeds. A single
    /// multi-kind document isn't used here because `emit()` short-circuits
    /// on the first strict violation, which would only ever validate the
    /// first offending kind and silently skip the rest.
    fn assert_parity(target_name: &str, emitter: &dyn Emitter) {
        for entity in one_of_every_kind() {
            let kind = entity_kind_kebab(&entity).to_string();
            let doc = single_entity_doc(entity);

            let capabilities = emitter.capabilities(&doc);
            let status = capabilities
                .iter()
                .find(|c| c.entity_kind == kind)
                .unwrap_or_else(|| {
                    panic!("{target_name}: capabilities() reported nothing for kind '{kind}'")
                })
                .status;

            let emit_result = emitter.emit(&doc, true);

            match status {
                CoverageStatus::Supported => assert!(
                    emit_result.is_ok(),
                    "{target_name}/{kind}: capabilities() says Supported but emit(strict=true) errored: {:?}",
                    emit_result.err()
                ),
                CoverageStatus::Lossy | CoverageStatus::Dropped => assert!(
                    emit_result.is_err(),
                    "{target_name}/{kind}: capabilities() says {status:?} but emit(strict=true) succeeded"
                ),
            }
        }
    }

    #[test]
    fn claude_capabilities_agree_with_strict_emit() {
        assert_parity("claude", &ClaudeEmitter);
    }

    #[test]
    fn cursor_capabilities_agree_with_strict_emit() {
        assert_parity("cursor-mdc", &CursorEmitter);
    }

    #[test]
    fn cursor_mcp_capabilities_agree_with_strict_emit() {
        assert_parity("cursor-mcp", &CursorMcpEmitter);
    }

    #[test]
    fn codex_capabilities_agree_with_strict_emit() {
        assert_parity("codex", &CodexEmitter);
    }

    #[test]
    fn copilot_capabilities_agree_with_strict_emit() {
        assert_parity("copilot", &CopilotEmitter);
    }

    #[test]
    fn gemini_capabilities_agree_with_strict_emit() {
        assert_parity("gemini", &GeminiEmitter);
    }

    #[test]
    fn windsurf_capabilities_agree_with_strict_emit() {
        assert_parity("windsurf", &WindsurfEmitter);
    }

    #[test]
    fn antigravity_capabilities_agree_with_strict_emit() {
        assert_parity("antigravity", &AntigravityEmitter);
    }

    #[test]
    fn agent_skills_capabilities_agree_with_strict_emit() {
        assert_parity("agent-skills", &AgentSkillsEmitter);
    }
}
