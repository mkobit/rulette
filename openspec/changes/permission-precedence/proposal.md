## Why

Multi-entity governance across Rulette's five core harnesses (Claude Code, OpenCode, Cursor, Antigravity, and Codex — see Journey 6 in `docs/2026-08-18-cli-ux-design.md`) is currently blocked on a foundational safety gap: `ToolAccessRule` (`src/ir/mod.rs`) defines no conflict-resolution or precedence model when multiple rules or packages apply to the same tool or overlapping patterns, and permission configurations are currently relegated to opaque `Unsupported` graph packages (`src/ir/graph.rs`).

Each target harness evaluates permissions with incompatible native semantics:

- **Claude Code** evaluates set-based `deny > ask > allow` arrays where action category strictly dominates pattern specificity (`deny: ["Bash(rm *)"]` blocks `allow: ["Bash(rm /tmp/cache)"]`).
- **OpenCode** evaluates `"permission"` rules sequentially in definition order (**last matching rule wins**) without built-in `deny > allow` priority or wildcard specificity scoring.
- **Cursor CLI (`cli.json`) and IDE (`permissions.json`)** lack an explicit `ask` rule list (`cli.json` supports only `allow` and `deny`; IDE `permissions.json` supports only `terminalAllowlist` and `mcpAllowlist`).
- **Antigravity** sorts typed grants (`command(...)`, `read_file(...)`, `write_file(...)`, `mcp(...)`, `read_url(...)`) by descending target length with `deny > allow`, and strictly rejects generic wildcards such as `command(*)`.
- **Codex** enforces Starlark `prefix_rule` decisions (`forbidden > prompt > allow`) for command prefixes in `.codex/rules/*.rules`, while `AGENTS.md` is purely advisory Markdown text with zero runtime enforcement.

Without a canonical, fail-closed precedence engine and capability-reporting contract, translating permissions across harnesses risks **failing open** — silently widening an `allow` rule, dropping a `deny` rule while keeping an overlapping `allow` rule active, emitting OpenCode rules in an order where a broad `allow` overrides a narrow `deny`, or lowering mechanical permissions into advisory Markdown prose.

This change (`rulette-5bk.14`) defines the deterministic permission precedence model, fail-safe lowering invariants, capability reporting (`Emitter::capabilities()` / `emitters::lowering`), and downstream implementation blueprint for `ToolAccessRule` and `PackageKind::Permissions`.

## What changes

- Introduce a pure, deterministic permission precedence and normalization engine in `src/ir/permissions.rs` (re-exported from `src/ir/mod.rs` and `src/lib.rs`), replacing untyped `action: Option<String>` on `ToolAccessRule` with strongly typed `PermissionAction` (`Allow`, `Ask`, `Deny`), `PermissionScope` (`User`, `Project`, `Local`, `Enterprise`), `CanonicalTool`, `NormalizedPattern`, and a 6-tuple `SpecificityKey`.
- Define a strict total ordering for conflict resolution and canonical emission across four tiers (`Tier 1: UniversalBaseline < Tier 2: SpecificAllow < Tier 3: SpecificAsk < Tier 4: HardDeny`), guaranteeing that `Deny` strictly dominates `Ask` and `Allow` across all scopes and overlapping patterns, and making sequential `last-match-wins` targets (OpenCode) semantically isomorphic to set-based `Deny > Ask > Allow` targets (Claude, Codex, Cursor, Antigravity).
- Promote permissions into a first-class compilation graph unit (`PackageKind::Permissions` and `SemanticItem::Permissions` in `src/ir/graph.rs`) with `TargetPermissionPolicy` (`Bare` or `Wrapped` per-target overrides) and commutative, fail-closed cross-package policy merging.
- Enforce the **Fail-Safe Invariant** and **Asymmetric Approximation Law** across all frontends and backends:
  - Missing or unrecognized permission actions fail parsing immediately (never default to `Allow`).
  - Permissions are never lowered to advisory Markdown instructions (`AGENTS.md`, `CLAUDE.md`, `.mdc`, `SKILL.md`).
  - Unrepresentable `Allow` patterns are never widened; under `--allow-lossy`, they are withheld (failing closed).
  - Unrepresentable `Deny` rules that overlap an `Allow` rule cause the overlapping `Allow` rule to be withheld so a target can never auto-approve a denied operation.
  - Unrepresentable `Ask` carve-outs overlapping an `Allow` rule on `Allow`/`Deny`-only targets (Cursor `cli.json`) are promoted to `Deny` under `--allow-lossy` (`permission-ask-promoted-to-deny`).
- Extend structured capability reporting (`Emitter::capabilities()` / `emitters::lowering::capabilities()` and `lower()`) with seven stable permission reason codes (`permission-target-unsupported`, `permission-tool-unrepresentable`, `permission-action-unrepresentable`, `permission-ask-promoted-to-deny`, `permission-pattern-unrepresentable`, `permission-precedence-unrepresentable`, and `permission-scope-unrepresentable`).
- Provide a step-by-step downstream implementation blueprint in `tasks.md` and a companion Beads workflow formula (`.beads/formulas/permission-precedence-implementation.formula.toml`) tailored for execution by downstream models.

## Scope

- Pure permission precedence, specificity scoring, shadowing detection, and conflict-resolution engine in `src/ir/permissions.rs`.
- Strongly-typed `ToolAccessRule`, `PermissionAction`, `PermissionScope`, `PermissionPolicy`, and `TargetPermissionPolicy` in `src/ir/mod.rs` and `src/ir/graph.rs`.
- Native permission ingestion in `src/parsers/{claude,opencode,cursor,antigravity,codex}.rs`.
- Capability-aware permission lowering and `NativeArtifactClass::PermissionConfig` mappings in `src/emitters/lowering.rs` and `src/publication/mapping.rs`.
- Structured capability reporting in `inspect --to`, `inspect --coverage`, strict `transform` rejection, and `--allow-lossy` staged publication plans.

## Non-goals

- Emitting OS-level sandbox flags (`sandbox_mode` in Codex) or cloud-managed enterprise dashboard configurations (`enterprise` and `local` scopes remain unpublishable in staged publication mappings and report `Dropped` via `permission-scope-unrepresentable`).
- Synthesizing executable shell hook scripts (`PreToolUse`) to emulate unrepresentable permission patterns (Rulette is a static configuration compiler and never synthesizes runtime code or scripts).
- Generalized hook event taxonomy (`rulette-5bk.12`) and MCP server tool-filtering (`rulette-5bk.13`), which are sibling epics designed separately.

## Capabilities

### Modified capabilities

- `ir-core`: Add typed `ToolAccessRule`, `PermissionAction`, `PermissionScope`, `TargetPermissionPolicy`, `PackageKind::Permissions`, and the deterministic 4-tier conflict-resolution and shadowing model.
- `frontends-and-backends`: Define native permission ingestion, fail-closed target lowering, the Asymmetric Approximation Law, and structured `Emitter::capabilities()` / `lowering::capabilities()` permission loss reporting across Claude, OpenCode, Cursor, Antigravity, Codex, and AgentPlugin.
- `transform-pipeline`: Enforce strict-by-default fail-closed rejection of permission capability losses before staging or publication, and retain structured permission loss findings in staged plans under `--allow-lossy`.

## Impact

- `src/ir/permissions.rs` & `src/ir/mod.rs`: New pure permission domain and precedence engine module; re-export `ToolAccessRule`, `PermissionAction`, `PermissionScope`, `PermissionPolicy`, `TargetPermissionPolicy`, and `ResolvedPermissionPolicy`.
- `src/ir/graph.rs`: Add `PackageKind::Permissions` and `SemanticItem::Permissions`.
- `src/parsers/{claude,opencode,cursor,antigravity,codex}.rs`: Parse native permission configurations into `PackageKind::Permissions` packages instead of opaque unsupported packages.
- `src/emitters/lowering.rs`: Add `NativeArtifactClass::PermissionConfig`, seven permission `CapabilityReasonCode` variants, `capabilities()` analysis, and target-specific fail-closed permission renderers.
- `src/publication/mapping.rs` & `src/publication/plan.rs`: Register verified `PermissionConfig` paths for project and user scopes and serialize the new permission `CapabilityReasonCode` tokens.
- `src/cli/commands/inspect.rs`: Report `permissions` package kind in `inspect --to` and `inspect --coverage`.
- `.beads/formulas/permission-precedence-implementation.formula.toml`: Add deterministic downstream agent execution formula.
- Tracking bead: `rulette-5bk.14`.
