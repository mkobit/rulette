## Why

Rulette compiles AI rules, skills, subagents, and permissions across five target harnesses: Claude Code, Cursor, Google Antigravity, OpenAI Codex, and OpenCode.
Multi-entity governance requires precise skill configurations and path-scoped rule boundaries across these tools.
Currently, `SemanticItem::Skill` in [`src/ir/graph.rs`](../../../src/ir/graph.rs) stores only `primary_instruction`, `description`, and untyped `frontend_payload`.
When frontends ingest skill metadata such as `allowed-tools`, `paths`, and execution policy flags, backends drop these fields with `SkillFrontmatterUnrepresentable` diagnostics.
Target harnesses also implement different skill sidecar and configuration models.
OpenAI Codex requires an `agents/openai.yaml` sidecar file within the skill directory to declare user interface metadata, Model Context Protocol (`MCP`) dependencies, and invocation policies.
Antigravity permits only `name` and `description` in `SKILL.md` frontmatter.
Target harnesses also implement different rule scoping architectures.
Claude Code evaluates scoped rules in `.claude/rules/<id>.md` using `paths: [...]` frontmatter.
Cursor evaluates scoped rules in `.cursor/rules/<id>.mdc` using `globs: [...]` and `alwaysApply: false`.
Google Antigravity evaluates scoped rules in `.agents/rules/<id>.md` using `trigger: glob` and `globs: [...]`.
OpenAI Codex does not support glob-scoped rule files and only evaluates `AGENTS.md` files located in the current working directory and parent directories.
Currently, Rulette maps all rules to root `AGENTS.md` or `CLAUDE.md`, which inflates token counts and causes `artifact-collision` failures during multi-rule compilation.
Claude Code does not read `AGENTS.md` by default unless bridged through `@AGENTS.md` import syntax in `CLAUDE.md`.
This change (`rulette-g0h` / `rulette-o7t.7`) defines typed skill frontmatter fields, sidecar ingestion and lowering for Codex, deterministic scoped rule emission across all targets, Longest Common Directory Prefix (`LCDP`) rule projection for Codex, and the `@AGENTS.md` import bridge for Claude Code.

## What changes

- Add typed skill metadata fields to `SemanticItem::Skill` in [`src/ir/graph.rs`](../../../src/ir/graph.rs): `allowed_tools`, `paths`, `disable_model_invocation`, `user_invocable`, `context`, `agent`, `display_name`, `icon`, `color`, and `default_prompt`.
- Ingest and emit Codex `agents/openai.yaml` sidecar files in [`src/parsers/codex.rs`](../../../src/parsers/codex.rs) and [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs).
- Emit scoped rules to `.claude/rules/<id>.md` with `paths: [...]` frontmatter.
- Emit scoped rules to `.cursor/rules/<id>.mdc` with `globs: [...]` and `alwaysApply: false`.
- Emit scoped rules to `.agents/rules/<id>.md` with `trigger: glob` and `globs: [...]`.
- Implement Longest Common Directory Prefix (`LCDP`) calculation for rules with glob triggers to project subtree rules into `<subdir>/AGENTS.md` for Codex.
- Inline cross-cutting extension globs (where `LCDP` is empty) into root `AGENTS.md` with scoped section annotations and Tier 2 capability diagnostics.
- Emit `@AGENTS.md` import bridge in `CLAUDE.md` when root instructions target Claude Code.
- Update target mapping rules in [`src/publication/mapping.rs`](../../../src/publication/mapping.rs) to permit `.claude/rules/*.md` and subdirectory `*/AGENTS.md` instruction files.
- Report structured capability findings using stable reason codes for lossy or dropped frontmatter under `--allow-lossy`.

## Scope

- Intermediate representation extensions for skill metadata in [`src/ir/graph.rs`](../../../src/ir/graph.rs).
- Native frontmatter and sidecar ingestion across all five frontends.
- Target lowering logic for skills, sidecars, and scoped rules in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs).
- Longest Common Directory Prefix (`LCDP`) path projection algorithm and collision bundling.
- Target mapping rule updates in [`src/publication/mapping.rs`](../../../src/publication/mapping.rs).
- Structured capability analysis and reporting in `inspect --to` and `inspect --coverage`.
- Staged publication plan validation for scoped rules and skill sidecars.

## Non-goals

- Dynamic execution or runtime supervision of Model Context Protocol (`MCP`) servers.
- Synthesis of external script shims such as `rulette-guard.py`.
- Runtime file system monitoring or daemon processes.
- Modification of host environment settings or network configuration.
- Parsing or execution of programmatic TypeScript plugins for OpenCode.

## Capabilities

### Modified capabilities

- `ir-core`: Add typed metadata fields to `SemanticItem::Skill` and hash canonical skill metadata in `PackageId`.
- `frontends-and-backends`: Ingest and emit Codex `agents/openai.yaml` sidecars, emit `.claude/rules/<id>.md`, `.cursor/rules/<id>.mdc`, `.agents/rules/<id>.md`, and Codex `LCDP` `<subdir>/AGENTS.md` projections.
- `transform-pipeline`: Validate publication paths for `.claude/rules/*.md` and subdirectory `*/AGENTS.md`, and record `LCDP` projection findings in staged publication plans.

## Impact

- [`src/ir/graph.rs`](../../../src/ir/graph.rs): Add typed skill frontmatter and metadata fields to `SemanticItem::Skill`.
- [`src/parsers/codex.rs`](../../../src/parsers/codex.rs): Ingest `agents/openai.yaml` sidecar files into canonical skill metadata.
- [`src/parsers/claude.rs`](../../../src/parsers/claude.rs): Ingest `.claude/rules/*.md` files and skill frontmatter fields.
- [`src/parsers/cursor.rs`](../../../src/parsers/cursor.rs): Ingest `.cursor/rules/*.mdc` files and skill frontmatter fields.
- [`src/parsers/antigravity.rs`](../../../src/parsers/antigravity.rs): Ingest `.agents/rules/*.md` files and validate skill frontmatter.
- [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs): Implement target lowering for skill sidecars, scoped rule formats, and Codex `LCDP` projections.
- [`src/publication/mapping.rs`](../../../src/publication/mapping.rs): Register authorized relative paths for `.claude/rules/*.md` and subdirectory `*/AGENTS.md`.
- [`src/cli/commands/inspect.rs`](../../../src/cli/commands/inspect.rs): Report skill metadata support and `LCDP` projection diagnostics.
- Tracking bead: `rulette-g0h` (P0 epic) and `rulette-o7t.7`.
