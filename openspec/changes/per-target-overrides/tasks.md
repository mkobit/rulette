## 1. Data model and IR integration

- [x] 1.1 Define parametric `TargetOverrides<T>` enum (`Wrapped { default: T, overrides: BTreeMap<String, T> }` and `Bare(T)`) with `#[serde(untagged)]` in `src/ir/mod.rs`.
- [x] 1.2 Implement `resolve(&self, target: &str) -> &T` with lookup precedence: exact match > tool alias match > default.
- [x] 1.3 Update `RuleMetadata::activation` in `src/ir/mod.rs` to `Option<TargetOverrides<Activation>>`.
- [x] 1.4 Add unit tests for `TargetOverrides` deserialization (bare vs wrapped), serialization, and override resolution fallback.

## 2. Parser integration

- [ ] 2.1 Verify `parse_rule_markdown` in `src/parsers/frontend.rs` correctly parses both bare and wrapped `rulette:activation` YAML frontmatter blocks.
- [ ] 2.2 Add unit tests for rule parsing with multi-target activation overrides.
- [ ] 2.3 Verify round-trip parsing of Cursor MDC files into `TargetOverrides<Activation>`.

## 3. Emitter resolution and capability updates

- [ ] 3.1 Update `src/emitters/cursor.rs` to resolve activation settings for `cursor-mdc` / `cursor`.
- [ ] 3.2 Update remaining emitters (`claude.rs`, `codex.rs`, `copilot.rs`, `gemini.rs`, `windsurf.rs`, `cursor_mcp.rs`, `agent_skills.rs`) to consume target-resolved activation.
- [ ] 3.3 Ensure emitter capability checks and lossy warnings inspect the resolved activation settings for their target format.

## 4. Schema generation

- [ ] 4.1 Update `src/cli/commands/schema.rs` so `rulette schema rulette:activation` generates the schema for `TargetOverrides<Activation>`.
- [ ] 4.2 Add a test verifying `rulette schema rulette:activation` outputs valid JSON Schema covering bare and wrapped variants.

## 5. End-to-end validation

- [ ] 5.1 Add integration tests transforming multi-target override rules into various target outputs.
- [ ] 5.2 Verify backwards compatibility across existing test fixtures and commands.
- [ ] 5.3 Run `mise run spec-validate` and full test suite (`cargo test`).
