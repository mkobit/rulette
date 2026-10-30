## ADDED Requirements

### Requirement: Native skill sidecar and scoped rule lowering

Frontends SHALL ingest skill frontmatter, Codex `agents/openai.yaml` sidecars, and native scoped rules.
Target backends SHALL lower `SkillMetadata` into target-specific skill formats and sidecars.
Target backends SHALL lower scoped rules with `ActivationMode::Glob` into native scoped rule files (`.claude/rules/<id>.md`, `.cursor/rules/<id>.mdc`, `.agents/rules/<id>.md`).

#### Scenario: Ingesting and lowering Codex agents/openai.yaml sidecar

- **WHEN** a skill package targeting `codex` contains `display_name`, `disable_model_invocation`, or `allowed_tools`
- **THEN** the Codex backend SHALL emit `skills/<name>/SKILL.md`
- **AND** SHALL emit `skills/<name>/agents/openai.yaml` containing the corresponding `interface`, `policy.allow_implicit_invocation`, and `dependencies.tools` blocks.

#### Scenario: Lowering scoped rules to Claude Code

- **WHEN** a rule package contains `ActivationMode::Glob` and targets `claude`
- **THEN** the Claude backend SHALL emit `.claude/rules/<id>.md` as `NativeArtifactClass::Rule`
- **AND** SHALL render YAML frontmatter containing `paths: [globs]`.

#### Scenario: Lowering scoped rules to Cursor

- **WHEN** a rule package contains `ActivationMode::Glob` and targets `cursor`
- **THEN** the Cursor backend SHALL emit `.cursor/rules/<id>.mdc` as `NativeArtifactClass::Rule`
- **AND** SHALL render YAML frontmatter containing `globs: [globs]` and `alwaysApply: false`.

#### Scenario: Lowering scoped rules to Antigravity

- **WHEN** a rule package contains `ActivationMode::Glob` and targets `antigravity`
- **THEN** the Antigravity backend SHALL emit `.agents/rules/<id>.md` as `NativeArtifactClass::Rule`
- **AND** SHALL render YAML frontmatter containing `trigger: glob` and `globs: [globs]`.

### Requirement: Longest common directory prefix rule projection for Codex

When lowering scoped rules to `codex`, the backend SHALL compute the Longest Common Directory Prefix (`LCDP`) across the rule's glob patterns.
If the prefix is non-empty (`<D>`), the backend SHALL project the rule to `<D>/AGENTS.md`.
If the prefix is empty, the backend SHALL inline the rule into root `AGENTS.md` under a scoped markdown section and report `rule-activation-approximated`.

#### Scenario: Subtree glob projects to subdirectory AGENTS.md

- **WHEN** a rule with glob `src/api/**/*.ts` is lowered to `codex`
- **THEN** the backend SHALL emit `src/api/AGENTS.md` as `NativeArtifactClass::Instruction`
- **AND** SHALL report capability status `Supported`.

#### Scenario: Cross-cutting glob inlines into root AGENTS.md

- **WHEN** a rule with glob `**/*.test.ts` is lowered to `codex`
- **THEN** the backend SHALL inline the rule into root `AGENTS.md` with a scoped markdown section header
- **AND** SHALL report capability status `Lossy` with reason code `rule-activation-approximated`.

### Requirement: One-way import bridge in CLAUDE.md

When emitting root instructions for Claude Code, the backend SHALL prepend `@AGENTS.md` to `CLAUDE.md` if `AGENTS.md` is emitted or present in the workspace.
The backend SHALL NOT emit `@` import statements referencing `CLAUDE.md` inside `AGENTS.md`.

#### Scenario: Emitting CLAUDE.md with import bridge

- **WHEN** root instructions are emitted for `claude` and an `AGENTS.md` artifact is generated
- **THEN** the emitted `CLAUDE.md` SHALL start with `@AGENTS.md\n\n`.
