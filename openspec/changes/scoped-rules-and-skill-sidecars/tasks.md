## Execution blueprint and dependency map

This task breakdown structures downstream implementation into sequential layers:

- Layer 1 (Task 1) implements pure intermediate representation types for skill metadata in `src/ir/graph.rs`.
- Layer 2 (Task 2) updates frontend parsers across all five target harnesses to ingest skill metadata and sidecars.
- Layer 3 (Task 3) implements the Longest Common Directory Prefix (`LCDP`) calculation algorithm.
- Layer 4 (Task 4) implements target lowerings for scoped rules, skill sidecars, and subdirectory projections.
- Layer 5 (Task 5) extends publication path mappings and staged publication validation in `src/publication/mapping.rs`.
- Layer 6 (Task 6) integrates `@AGENTS.md` bridge emission and CLI inspection diagnostics.
- Layer 7 (Task 7) validates end-to-end integration through automated unit and integration tests.

## 1. Layer 1: Pure intermediate representation types for skill metadata

- [ ] 1.1 Extend `src/ir/graph.rs` with `SkillMetadata`:
  - Implement fields: `display_name`, `allowed_tools`, `paths`, `disable_model_invocation`, `user_invocable`, `context`, `agent`, `icon`, `color`, and `default_prompt`.
  - Add `SkillMetadata::is_empty(&self) -> bool` helper.
  - Implement `SkillMetadata::validate(&self) -> Result<()>` checking glob safety and tool string grammar.
  - Derive `Serialize`, `Deserialize`, `JsonSchema`, `Clone`, `Debug`, `Eq`, and `PartialEq`.
- [ ] 1.2 Update `SemanticItem::Skill` in `src/ir/graph.rs`:
  - Add `metadata: SkillMetadata` field with `#[serde(default, skip_serializing_if = "SkillMetadata::is_empty")]`.
  - Update `SemanticItem::validate()` to validate `SkillMetadata`.
  - Incorporate serialized `SkillMetadata` digest into `PackageId` hash calculation in `Package::package_id()`.
- [ ] 1.3 Write unit tests in `src/ir/graph.rs`:
  - Verify JSON and TOML serialization round-trip with populated `SkillMetadata`.
  - Verify `PackageId` changes when `SkillMetadata` fields change.
  - Verify rejection of unsafe path traversal in skill `paths`.

## 2. Layer 2: Frontend parsers update for skill metadata and sidecars

- [ ] 2.1 Update OpenAI Codex parser in `src/parsers/codex.rs`:
  - Ingest `agents/openai.yaml` when present in `.codex/skills/<id>/` or `.agents/skills/<id>/`.
  - Deserialize `interface`, `policy`, and `dependencies.tools` blocks into `SkillMetadata`.
  - Mark `agents/openai.yaml` observation disposition as `NativeObservationDisposition::PackageContent`.
- [ ] 2.2 Update Claude Code parser in `src/parsers/claude.rs`:
  - Ingest scoped rules from `.claude/rules/*.md` into `PackageKind::Rule` with `ActivationMode::Glob`.
  - Parse `allowed-tools`, `paths`, `disable-model-invocation`, `user-invocable`, `context`, and `agent` from `SKILL.md` frontmatter into `SkillMetadata`.
- [ ] 2.3 Update Cursor parser in `src/parsers/cursor.rs`:
  - Parse `paths`, `globs`, `disable-model-invocation`, `icon`, and `color` from `SKILL.md` frontmatter into `SkillMetadata`.
  - Ingest scoped rules from `.cursor/rules/*.mdc` with `alwaysApply: false`.
- [ ] 2.4 Update Google Antigravity parser in `src/parsers/antigravity.rs`:
  - Parse `name` and `description` from `SKILL.md` frontmatter and reject unrepresentable fields under strict mode.
  - Ingest scoped rules from `.agents/rules/*.md` with `trigger: glob`.
- [ ] 2.5 Write frontend parser unit tests in `src/parsers/`:
  - Verify round-trip parsing of Codex skills with `agents/openai.yaml`.
  - Verify parsing of scoped rules across Claude, Cursor, and Antigravity frontends.

## 3. Layer 3: Longest common directory prefix calculation algorithm

- [ ] 3.1 Implement `calculate_lcdp` in `src/ir/paths.rs` or `src/emitters/lowering.rs`:
  - Normalize path separators from Windows `\` to POSIX `/`.
  - Extract literal prefix before wildcard characters (`*`, `?`, `[`).
  - Extract parent directory of literal prefix.
  - Compute intersection of directory prefixes for multi-glob rules.
  - Return empty string for root and cross-cutting extension globs.
- [ ] 3.2 Add path normalization and security checks:
  - Reject path traversal (`..`), leading slashes, and control characters.
  - Strip redundant leading `./`.
- [ ] 3.3 Write unit tests for `calculate_lcdp`:
  - Test subtree globs: `src/api/**/*.ts` -> `src/api`.
  - Test nested files: `crates/core/src/parser.rs` -> `crates/core/src`.
  - Test extension globs: `**/*.test.ts` -> `""`.
  - Test disjoint globs: `["src/api/**", "lib/api/**"]` -> `""`.
  - Test single-directory wildcards: `src/*.ts` -> `src`.

## 4. Layer 4: Target lowering backends

- [ ] 4.1 Implement Codex skill sidecar emission in `src/emitters/lowering.rs`:
  - Lower `SkillMetadata` to `skills/<name>/agents/openai.yaml`.
  - Map `display_name`, `icon`, `color`, `default_prompt` to `interface`.
  - Map `disable_model_invocation` to `policy.allow_implicit_invocation`.
  - Map `allowed_tools` to `dependencies.tools`.
  - Omit sidecar when `SkillMetadata` is empty.
- [ ] 4.2 Implement scoped rule lowering to `.claude/rules/<id>.md`:
  - Lower `ActivationMode::Glob` rules to `rules/<id>.md`.
  - Render YAML frontmatter with `paths: [globs]`.
- [ ] 4.3 Implement scoped rule lowering to `.cursor/rules/<id>.mdc`:
  - Render YAML frontmatter with `description`, `globs: [globs]`, and `alwaysApply: false`.
- [ ] 4.4 Implement scoped rule lowering to `.agents/rules/<id>.md`:
  - Render YAML frontmatter with `description`, `trigger: glob`, and `globs: [globs]`.
- [ ] 4.5 Implement Codex `LCDP` rule projection:
  - If `LCDP` is non-empty (`<D>`), emit `<D>/AGENTS.md` as `NativeArtifactClass::Instruction` (`Tier 1`).
  - Bundle multiple rules projecting to the same `<D>/AGENTS.md` deterministically.
  - If `LCDP` is empty, inline into root `AGENTS.md` under scoped markdown section header (`Tier 2`).
  - Record `CapabilityReasonCode::RuleActivationApproximated` finding for cross-cutting globs.
- [ ] 4.6 Write lowering unit tests:
  - Verify Codex emission of `agents/openai.yaml`.
  - Verify Claude emission of `.claude/rules/<id>.md`.
  - Verify Codex `LCDP` subdirectory projection to `<subdir>/AGENTS.md`.

## 5. Layer 5: Publication mapping and staged publication validation

- [ ] 5.1 Update `src/publication/mapping.rs`:
  - Add `.claude/rules/*.md` rule prefix to `CLAUDE_PROJECT_RULES`.
  - Add subdirectory `*/AGENTS.md` instruction rule to `CODEX_PROJECT_RULES`.
  - Add `skills/*/agents/openai.yaml` sidecar prefix to `CODEX_PROJECT_RULES`.
- [ ] 5.2 Update staged publication plan generation in `src/publication/stage.rs`:
  - Record `PlanLossFinding` for unrepresentable skill tools under `--allow-lossy`.
  - Enforce fail-closed rejection when `allow_lossy: false`.
- [ ] 5.3 Write publication mapping unit tests:
  - Verify authorized paths for `.claude/rules/<id>.md`.
  - Verify authorized paths for `src/api/AGENTS.md`.
  - Verify authorized paths for `skills/<id>/agents/openai.yaml`.

## 6. Layer 6: Import bridge emission and CLI inspection

- [ ] 6.1 Implement `@AGENTS.md` import bridge in `src/emitters/lowering.rs`:
  - Prepend `@AGENTS.md\n\n` to `CLAUDE.md` when root instructions target Claude Code and `AGENTS.md` is emitted or present.
  - Guarantee unidirectional import invariant (never emit `@CLAUDE.md` in `AGENTS.md`).
- [ ] 6.2 Update `src/cli/commands/inspect.rs`:
  - Display skill metadata fields (`allowed_tools`, `paths`, invocation policies).
  - Display rule projection destinations and `LCDP` paths in `inspect --to`.
- [ ] 6.3 Update schema generator in `src/cli/commands/schema.rs`:
  - Register `SkillMetadata` in generated schemas.

## 7. Layer 7: Verification and quality gates

- [ ] 7.1 Validate OpenSpec specifications strictly using `mise run spec-validate`.
- [ ] 7.2 Run cargo test suite using `cargo test`.
- [ ] 7.3 Run linter checks using `cargo clippy --all-targets --all-features`.
- [ ] 7.4 Run code formatting checks using `cargo fmt -- --check`.
