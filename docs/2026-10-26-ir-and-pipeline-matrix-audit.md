# Rulette IR and pipeline matrix audit (`rulette-o7t.1`)

## Executive summary

This audit compares current Rulette Intermediate Representation (IR), OpenSpec specifications, and parser/emitter implementations against Section 2 in [docs/2026-10-24-gemini-argon-external-research.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-24-gemini-argon-external-research.md).
It incorporates the architectural decisions from [docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md).
The audit covers five primary harnesses: Claude Code, Cursor, Google Antigravity, OpenAI Codex, and OpenCode.
It identifies implementation gaps across rules, skills, subagents, permissions, hooks, and plugins.

---

## 1. Specification and implementation audit matrix

| Primitive | Target matrix specification | Current OpenSpec status | Current Rust implementation status | Identified gap and required action |
| :--- | :--- | :--- | :--- | :--- |
| **Skills (Project & `$HOME` paths)** | Claude Code: `.claude/skills/<id>/SKILL.md`, `~/.claude/skills/<id>/SKILL.md`.<br>Cursor: `.agents/skills/<id>/SKILL.md` or `.cursor/skills/`.<br>Antigravity: `.agents/skills/<id>/SKILL.md`, CLI: `~/.gemini/antigravity-cli/skills/`.<br>Codex: `.agents/skills/<id>/SKILL.md`, `~/.agents/skills/<id>/SKILL.md`.<br>OpenCode: `.agents/skills/<id>/SKILL.md` or `.opencode/skills/`. | Specified in [openspec/specs/frontends-and-backends/spec.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/specs/frontends-and-backends/spec.md) and [openspec/changes/skill-fidelity-and-layout/](file:///home/mkobit/workspace/mkobit/rulette/openspec/changes/skill-fidelity-and-layout/). | Parsers in [src/parsers/skills.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/skills.rs) parse shared `.agents/skills/`.<br>Target lowering in [src/emitters/lowering.rs](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L293) emits `skills/<id>/SKILL.md`.<br>Publication in [src/publication/mapping.rs](file:///home/mkobit/workspace/mkobit/rulette/src/publication/mapping.rs#L130-L217) maps targets. | Cursor project mapping lacks native skill path and lowers skills to rules with `SkillLoweredAsRule`.<br>Codex project mapping maps to `.codex/skills/` instead of `.agents/skills/`.<br>Update [src/publication/mapping.rs](file:///home/mkobit/workspace/mkobit/rulette/src/publication/mapping.rs) to map `.agents/skills/` for Cursor and Codex. |
| **Skill frontmatter & sidecars** | Claude Code: `name`, `description`, `allowed-tools`, `paths`, `disable-model-invocation`, `user-invocable`, `context`, `agent`.<br>Cursor: `name`, `description`, `paths`/`globs`, `disable-model-invocation`, `icon`, `color`, `metadata`.<br>Antigravity: strictly `name` and `description`.<br>Codex: `name`, `description`, plus `agents/openai.yaml` sidecar.<br>OpenCode: `name`, `description`, `metadata`. | OpenSpec lacks schema definitions for skill frontmatter extensions and Codex sidecars. | [src/ir/graph.rs](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs#L501-L506) stores only `primary_instruction`, `description`, and untyped `frontend_payload`.<br>[src/emitters/lowering.rs](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L328-L375) drops cross-domain frontmatter with `SkillFrontmatterUnrepresentable`. | No typed intermediate representation exists for `allowed-tools`, `paths`, or invocation flags.<br>No emitter exists for Codex `agents/openai.yaml` sidecar.<br>Promote skill metadata fields to typed properties on `SemanticItem::Skill` in [src/ir/graph.rs](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs). |
| **Always-on rules** | Claude Code: `CLAUDE.md` with `@AGENTS.md` import bridge.<br>Cursor: `AGENTS.md` (root and subdirectories).<br>Antigravity: `AGENTS.md` or `GEMINI.md`.<br>Codex: `AGENTS.md` or `AGENTS.override.md`.<br>OpenCode: `AGENTS.md` or `opencode.json` instructions. | [openspec/specs/ir-core/spec.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/specs/ir-core/spec.md) requires `rule` entity.<br>[openspec/changes/rule-bundling-and-marketplace-transforms/](file:///home/mkobit/workspace/mkobit/rulette/openspec/changes/rule-bundling-and-marketplace-transforms/) specifies bundling into `AGENTS.md` and `CLAUDE.md`. | [src/emitters/lowering.rs](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L781-L788) maps `AGENTS.md` (Codex) and `CLAUDE.md` (Claude).<br>Single-rule constraint causes `artifact-collision` on multi-rule inputs when bundling is incomplete. | Claude emitter does not emit `@AGENTS.md` bridge.<br>Cursor emitter does not emit `AGENTS.md`.<br>Antigravity emitter does not emit root `AGENTS.md` or `GEMINI.md`.<br>Codex does not ingest `AGENTS.override.md`. |
| **Scoped / conditional rules** | Claude Code: `.claude/rules/<id>.md` with `paths: [...]`.<br>Cursor: `.cursor/rules/<id>.mdc` with `globs: [...]` and `alwaysApply: false`.<br>Antigravity: `.agents/rules/<id>.md` with `trigger: glob` and `globs: [...]`.<br>Codex: Longest Common Directory Prefix (LCDP) projection to `<subdir>/AGENTS.md`.<br>OpenCode: `.opencode/rules/<id>.md`. | [openspec/specs/ir-core/spec.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/specs/ir-core/spec.md) specifies `ActivationMode`.<br>No OpenSpec requirement exists for Codex LCDP subtree projection. | [src/emitters/lowering.rs](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L821-L860) renders Cursor `.mdc` and Antigravity `.md` frontmatter.<br>Claude target forces all rules into `CLAUDE.md` and does not emit `.claude/rules/<id>.md`.<br>Codex emitter does not implement LCDP projection. | Claude target drops scoped `paths` metadata and lacks `.claude/rules/` path mappings in [src/publication/mapping.rs](file:///home/mkobit/workspace/mkobit/rulette/src/publication/mapping.rs).<br>Codex lacks LCDP directory partitioning for subtree globs.<br>Add `.claude/rules/<id>.md` emitter and Codex LCDP subdirectory projection. |
| **Subagents (custom agents)** | Claude Code: `.claude/agents/<id>.md` (`tools`, `model`, `isolation`).<br>Cursor: `.cursor/agents/<id>.md` (`readonly`, `model`).<br>Antigravity: `.agents/agents/<id>.md` (`tools`, `model`, `commandExecutionPolicy`).<br>Codex: `.codex/agents/<id>.toml` (`developer_instructions`, `model`, `[features] shell_tool`).<br>OpenCode: `.opencode/agents/<id>.md`. | [openspec/specs/ir-core/spec.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/specs/ir-core/spec.md) lists `agent` entity kind without schema.<br>No backend specification exists for subagent lowering. | Parsers in [src/parsers/claude.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/claude.rs#L45), [src/parsers/cursor.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/cursor.rs#L59), [src/parsers/antigravity.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/antigravity.rs#L101), and [src/parsers/opencode.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/opencode.rs#L54) mark subagents as `Unsupported`.<br>Codex parser ignores `.codex/agents/*.toml`. | `SemanticItem` in [src/ir/graph.rs](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs) lacks `Agent` variant.<br>No capability mapping table exists (`targets.v1.json`).<br>No emitter exists for any target subagent format. |
| **Static permission configs** | Claude Code: `.claude/settings.json` (`permissions.allow`/`ask`/`deny`).<br>Cursor: `.cursor/permissions.json` and `.cursor/cli.json`.<br>Antigravity: `~/.gemini/antigravity-cli/settings.json`.<br>Codex: `.codex/rules/default.rules` (Starlark `prefix_rule`).<br>OpenCode: `opencode.json` ordered `permission.shell` map. | [openspec/changes/permission-precedence/](file:///home/mkobit/workspace/mkobit/rulette/openspec/changes/permission-precedence/) proposes complete permission model (0/24 tasks complete).<br>[openspec/specs/ir-core/spec.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/specs/ir-core/spec.md) has no permission schema. | [src/parsers/claude.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/claude.rs#L53) reads settings but discards permissions.<br>[src/parsers/opencode.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/opencode.rs#L228) marks permissions as `Unsupported`.<br>Cursor, Antigravity, and Codex parsers ignore permission files. | No typed intermediate representation exists in `main`.<br>Join-semilattice normalizer ($\text{Deny} \succ \text{Ask} \succ \text{Allow}$) is not implemented.<br>No static permission emitter exists.<br>Implement `rulette-5bk.14` / `rulette-o7t.3`. |
| **Runtime hook shim & command approval** | Self-invoking static binary hook (`rulette hook-eval --policy .rulette/policy.json`) for zero-mutation approval.<br>Interception events: Antigravity `PreToolUse`, Cursor `beforeShellExecution`/`preToolUse`, Claude Code `PreToolUse`, Codex `PreToolUse`. | No OpenSpec specification exists for `rulette hook-eval` or polyglot hook manifests.<br>`rulette-guard.py` rejected per decision `rulette-o7t.6`. | Rulette CLI in [src/cli/](file:///home/mkobit/workspace/mkobit/rulette/src/cli/) has no `hook-eval` subcommand.<br>No parser or emitter handles `hooks.json`. | Add `rulette hook-eval` subcommand to evaluate permission semilattice on standard input.<br>Add hook manifest emitter for `.agents/hooks.json`, `.cursor/hooks.json`, `.claude/settings.json`, and `.codex/hooks.json`. |
| **Plugin & marketplace files** | Claude Code: `.claude-plugin/plugin.json` + `marketplace.json` + `.mcp.json`.<br>Cursor: `.cursor-plugin/plugin.json` + `marketplace.json` + `mcp.json`.<br>Antigravity: strict root `plugin.json` (`$schema`, `name`, `description` only) + `mcp_config.json`.<br>Codex: root `plugin.json` (Agent Plugins 1.0) + `.codex-plugin/plugin.json`.<br>OpenCode: `.opencode/plugins/` or `opencode.json`. | [openspec/specs/frontends-and-backends/spec.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/specs/frontends-and-backends/spec.md) includes Agent Plugins 1.0.0.<br>[openspec/changes/rule-bundling-and-marketplace-transforms/](file:///home/mkobit/workspace/mkobit/rulette/openspec/changes/rule-bundling-and-marketplace-transforms/) specifies Claude marketplace transforms. | [src/parsers/agent_plugin/](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/agent_plugin/) parses Agent Plugins 1.0.<br>Lowering emits Agent Plugins 1.0 manifests.<br>Antigravity strict root `plugin.json` schema filtering is not implemented. | Antigravity rejects root `plugin.json` with extra fields (`additionalProperties: false`).<br>Rulette must emit strict sanitized `plugin.json` for Antigravity targets.<br>Vendor-specific plugin manifests (`.claude-plugin/`, `.cursor-plugin/`, `.codex-plugin/`) are not emitted. |

---

## 2. Detailed gap analysis by domain

### A. Intermediate representation (`ir-core`)

1. **Current status:**
   [src/ir/graph.rs](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs#L198-L202) defines `PackageKind` with only `Rule`, `Skill`, and `Unsupported`.
   [src/ir/graph.rs](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs#L491-L510) defines `SemanticItem` with only `Rule`, `Skill`, and `Unsupported`.
2. **Missing entities:**
   - `SemanticItem::Agent`: captures `name`, `description`, `instructions`, `model`, `tools`/`capabilities`, `readonly`, `background`, and `isolation`.
   - `SemanticItem::Permissions`: captures `PermissionPolicy`, `ToolAccessRule`, `PermissionAction` (`Allow`, `Ask`, `Deny`), `PermissionScope`, and pattern matching keys.
   - `SemanticItem::Hook`: captures event name, target tool matcher, execution mode, and policy reference.
3. **Skill metadata limitations:**
   `SemanticItem::Skill` contains only `description` and untyped `frontend_payload`.
   It lacks typed fields for `allowed_tools` / `allowed_commands`, `paths`, and invocation policy flags.

### B. Frontends and ingestion

1. **Subagents:**
   Every native frontend classifies agent definitions as `Unsupported` or ignores them:
   - Claude Code: [src/parsers/claude.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/claude.rs#L45) labels `.claude/agents/*.md` as unsupported `agent`.
   - Cursor: [src/parsers/cursor.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/cursor.rs#L59) labels agent paths as unsupported `cursor-agent`.
   - Antigravity: [src/parsers/antigravity.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/antigravity.rs#L101) labels agent paths as unsupported `antigravity-agent`.
   - Codex: [src/parsers/codex.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/codex.rs#L53) ignores `.codex/agents/*.toml` as unrecognized input.
   - OpenCode: [src/parsers/opencode.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/opencode.rs#L54) labels agent paths as unsupported `opencode-agent`.
2. **Permissions:**
   No frontend parses permission rules into a queryable data structure:
   - Claude Code: [src/parsers/claude.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/claude.rs#L287) reads `.claude/settings.json` but extracts only `mcpServers`.
   - OpenCode: [src/parsers/opencode.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/opencode.rs#L228) drops `permission` blocks into an unsupported package.
   - Cursor: ignores `.cursor/permissions.json` and `.cursor/cli.json`.
   - Antigravity: ignores `settings.json`.
   - Codex: ignores `.codex/rules/*.rules`.
3. **Skill sidecars:**
   [src/parsers/codex.rs](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/codex.rs) ignores `agents/openai.yaml` sidecar files.
4. **Hooks:**
   No frontend parses `hooks.json` or native hook configurations.

### C. Emitters and lowering (`frontends-and-backends`)

1. **Subagents:**
   [src/emitters/lowering.rs](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs) contains zero agent lowering functions.
   Rulette cannot emit `.claude/agents/<id>.md`, `.cursor/agents/<id>.md`, `.agents/agents/<id>.md`, `.codex/agents/<id>.toml`, or `.opencode/agents/<id>.md`.
2. **Permissions:**
   [src/emitters/lowering.rs](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs) contains zero permission lowering functions.
   Rulette cannot emit `.claude/settings.json`, `.cursor/permissions.json`, `.cursor/cli.json`, Antigravity `settings.json`, Starlark `.codex/rules/default.rules`, or `opencode.json` sorted maps.
3. **Rules:**
   - Claude target: forces all rules into a single `CLAUDE.md` and cannot emit scoped `.claude/rules/<id>.md` files.
   - Cursor target: cannot emit root `AGENTS.md`.
   - Antigravity target: cannot emit root `AGENTS.md` or `GEMINI.md`.
   - Codex target: cannot project subtree globs into subdirectory `<subdir>/AGENTS.md` files.
4. **Skills:**
   - Cursor target: converts skills to `.cursor/rules/<name>.mdc` with `SkillLoweredAsRule` instead of emitting `.agents/skills/<id>/SKILL.md` or `.cursor/skills/<id>/SKILL.md`.
   - Codex target: does not emit `agents/openai.yaml` sidecars.
   - Cross-domain frontmatter: drops `allowed-tools` and `paths` across different harnesses.

### D. Publication mapping and filesystem layout

1. **Missing target mappings in [src/publication/mapping.rs](file:///home/mkobit/workspace/mkobit/rulette/src/publication/mapping.rs):**
   - Cursor project scope: lacks mappings for `.agents/skills/` and `.cursor/skills/`.
   - Claude project scope: lacks mappings for `.claude/rules/` and `.claude/agents/`.
   - Codex project scope: maps skills to `.codex/skills/` instead of canonical `.agents/skills/`; lacks mappings for `.codex/agents/` and subdirectory `*/AGENTS.md`.
   - Antigravity project scope: lacks mappings for `.agents/agents/` and root `AGENTS.md` / `GEMINI.md`.
   - User scope: Cursor and AgentPlugin return hard errors (`user mapping is unavailable`); Antigravity maps flat `skills/` instead of `~/.gemini/antigravity-cli/skills/`.

### E. CLI pipeline and runtime architecture

1. **CLI command model:**
   [src/cli/](file:///home/mkobit/workspace/mkobit/rulette/src/cli/) exposes `transform`, `inspect`, and `schema`.
   Decision 1 in [docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md) establishes primary verbs:
   - `sync` (build and emit target outputs)
   - `plan` (calculate dry-run diff and lossiness report)
   - `check` (validate lint, security rules, and lossiness limits)
   - `revert` (restore state from `.rulette/ledger.json`)
2. **Runtime boundary:**
   Decision 2 in [docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md) rejects `rulette-guard.py`.
   It specifies:
   - Static config emission by default.
   - `rulette hook-eval --policy .rulette/policy.json` for zero-mutation hook interception.
   Rulette CLI currently lacks the `hook-eval` subcommand and policy evaluator.

---

## 3. Prioritized resolution roadmap

The gaps align with the P0–P3 roadmap defined in [docs/2026-10-24-gemini-argon-external-research.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-24-gemini-argon-external-research.md) and unblocked tasks in [docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-25-cli-ux-and-bazel-style-build-pipeline.md):

1. **P0: Rules, skills, and CLI UX (`rulette-o7t.5`, `rulette-4dq.2`, `rulette-4dq.4`)**
   - Emit `.claude/rules/<id>.md` with `paths` frontmatter.
   - Emit `CLAUDE.md` with `@AGENTS.md` import bridge.
   - Implement Longest Common Directory Prefix (LCDP) projection for Codex subdirectory `AGENTS.md` files.
   - Update Cursor project mapping to emit `.agents/skills/<id>/SKILL.md` natively instead of demoting skills to rules.
   - Update Codex project mapping to emit `.agents/skills/<id>/SKILL.md` with `agents/openai.yaml` sidecar.
   - Reshape CLI to support `sync`, `plan`, `check`, and `revert` verbs with optional `rulette.toml`/`rulette.yaml` workspace config.
2. **P1: Permission semilattice and static/hook emission (`rulette-o7t.3`, `rulette-o7t.6`, `rulette-5bk.14`, `rulette-5bk.14.1`)**
   - Implement `src/ir/permissions.rs` with typed `ToolAccessRule`, `PermissionAction`, `PermissionScope`, and the $\text{Deny} \succ \text{Ask} \succ \text{Allow}$ semilattice.
   - Ingest native permissions from Claude, Cursor, Antigravity, Codex, and OpenCode.
   - Emit static permission configs for all five targets.
   - Implement `rulette hook-eval` static binary subcommand and generate `hooks.json` manifests.
   - Map skill-scoped `allowed-tools` to session-wide permissions with Tier 2 scope-widening diagnostics.
3. **P2: Cross-harness subagents (`rulette-o7t.4`)**
   - Add `SemanticItem::Agent` to `CompilationGraph`.
   - Create embedded `profiles/targets.v1.json` mapping abstract capabilities (`file_read`, `code_search`, `file_edit`, `shell_exec`) to target-native tool lists.
   - Parse agent definitions across all five frontends.
   - Lower agents to `.claude/agents/<id>.md`, `.cursor/agents/<id>.md`, `.agents/agents/<id>.md`, `.codex/agents/<id>.toml`, and `.opencode/agents/<id>.md`.
4. **P3: Polyglot plugin and marketplace bundler (`rulette-4dq.3`, `rulette-d8u`)**
   - Emit strict sanitized root `plugin.json` for Antigravity (`additionalProperties: false`).
   - Emit vendor-specific manifests (`.claude-plugin/`, `.cursor-plugin/`, `.codex-plugin/`).
   - Emit marketplace catalogs across supported harnesses.
