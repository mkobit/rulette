## Context

Antigravity uses Markdown-based rules (typically placed in `.antigravity/` or discovered hierarchically) and skills (under `skills/<name>/SKILL.md`).
Rule activation in Antigravity is controlled via frontmatter `trigger` fields with four modes: `always_on`, `glob`, `manual`, and `model_decision`.
`docs/2026-08-18-cli-ux-design.md` §3 defines the mapping between canonical `ActivationMode` and Antigravity frontmatter.
With `rulette-5bk.8` landed, `TargetOverrides<Activation>` allows per-target activation configurations to resolve cleanly for the `antigravity` target identifier.

## Goals / Non-goals

**Goals:**

- Introduce `InputFormat::Antigravity` and `OutputFormat::Antigravity` across the CLI and library.
- Implement `AntigravityRuleFrontmatter` and parsing logic in `src/parsers/antigravity.rs` and `src/parsers/frontend.rs`.
- Implement `AntigravityEmitter` in `src/emitters/antigravity.rs` supporting rules (`.antigravity/<name>.md`) and skills (`skills/<name>/SKILL.md`).
- Map `ActivationMode` to Antigravity's `trigger` values (`always_on`, `glob`, `manual`, `model_decision`) bidirectionally.
- Register Antigravity in `TOOL_PATH_CONVENTIONS` for manifest scaffolding.
- Implement capability reporting in `AntigravityEmitter::capabilities` with strict parity testing.

**Non-goals:**

- Sub-agent and hook translation for Antigravity in this change (hooks and permissions will be dropped with standard warnings / strict errors).
- Changing the IR data model or `TargetOverrides` implementation (already completed in `rulette-5bk.8`).

## Decisions

**1. Data structures and frontmatter modeling for Antigravity rules.**

Antigravity rule frontmatter is modeled in `src/parsers/antigravity.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AntigravityTrigger {
    AlwaysOn,
    Glob,
    Manual,
    ModelDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AntigravityRuleFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<AntigravityTrigger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub globs: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
```

**2. Bidirectional mapping between `Activation` and Antigravity frontmatter.**

- `ActivationMode::Always` maps to `trigger: always_on`.
- `ActivationMode::Glob` maps to `trigger: glob`, with `globs` serialized as a list of strings.
- `ActivationMode::Manual` maps to `trigger: manual`.
- `ActivationMode::Model` maps to `trigger: model_decision`, with description preserved.
- When parsing, `trigger` is converted into a canonical `Activation` unless an explicit `rulette:activation` block is already present in frontmatter.
- When emitting, `rule.metadata.activation.resolve("antigravity")` is used to determine the emitted `trigger` and `globs`.

**3. Output file layout in `AntigravityEmitter`.**

- Rule entities emit to `<name>.md` (defaulting to `.antigravity/<name>.md` when target path is a directory).
- Skill entities emit to `skills/<name>/SKILL.md`.
- Unhandled entity kinds (`Hook`, `Agent`, `McpServer`, `Permissions`) report `Dropped` in `capabilities()` and error in strict mode with standard lossy conversion warnings in non-strict mode.

**4. Auto-detection and path conventions.**

- Input files under `.antigravity/` or containing Antigravity trigger frontmatter are detected as `InputFormat::Antigravity`.
- `TOOL_PATH_CONVENTIONS` in `src/cli/commands/transform.rs` adds a matcher for path components containing `.antigravity` mapping to `OutputFormat::Antigravity` with default path `.antigravity/`.

## Adversarial review

**1. Ambiguity when rule frontmatter contains both `trigger` and canonical `rulette:activation`.**

- *Issue*: A file could contain both legacy/native `trigger: manual` and `rulette:activation: { default: { mode: [always] } }`.
- *Mitigation*: The parser prioritizes explicit `rulette:activation` blocks when present, mirroring Cursor MDC's precedence over raw `alwaysApply`/`globs`.

**2. Multiple activation modes in canonical IR.**

- *Issue*: Canonical `Activation` allows `mode: Vec<ActivationMode>`, but Antigravity has a scalar `trigger` enum.
- *Mitigation*: The emitter checks modes in priority order: `Always` > `Glob` > `Model` > `Manual`. This is deterministic and matches the precedence used across other emitters.

**3. Frontmatter formatting and list globs vs comma-delimited strings.**

- *Issue*: Users might provide `globs` as a single comma-delimited string or a YAML sequence.
- *Mitigation*: Deserialization uses an untagged enum helper (similar to Cursor MDC parser) accepting both single strings and sequence arrays, normalizing to `Vec<String>`.

**4. Capabilities parity with strict emission.**

- *Issue*: If `capabilities()` reports `Supported` for an entity kind that `emit(strict=true)` rejects, coverage checking fails.
- *Mitigation*: `AntigravityEmitter` will be added to `assert_parity("antigravity", &AntigravityEmitter)` in `src/emitters/mod.rs` to guarantee strict agreement across all six entity kinds.

## Risks / Trade-offs

- **[Trade-off]** Dropping `Hook`, `Agent`, `McpServer`, and `Permissions` entities in `AntigravityEmitter`.
  → Mitigation: Antigravity-specific sub-agent, hook, and MCP configurations will be addressed in dedicated follow-up epics (`rulette-5bk.11`–`14`).
- **[Risk]** Target alias mismatch during override resolution.
  → Mitigation: `TargetOverrides::resolve` normalizes aliases so both `antigravity` and potential tool prefixes resolve to the same override block.

## Open Questions

None.
