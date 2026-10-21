# Rule bundling and marketplace transforms tasks

## 1. Phase 1: Pure IR and domain modeling

- [ ] 1.1 Add canonical `<plugin>` identifier validation and namespace helpers in [`src/ir/graph.rs`](../../../src/ir/graph.rs) for `FrontendPayload` namespaces `"agent-plugin.marketplace"` and `"agent-plugin.manifest"`.
  - Acceptance criteria: `CompilationGraph::validate` in [`src/ir/graph.rs`](../../../src/ir/graph.rs) preserves `/<plugin>:<skill>` namespace metadata across `to_canonical_json` and `to_canonical_toml` round-trips and rejects `<plugin>` names that are empty or contain `:`, `..`, `/`, `\`, or control characters.
- [ ] 1.2 Add a deterministic `(SemanticIdentity, PackageId)` ordering key helper for `PackageKind::Rule` packages in [`src/ir/graph.rs`](../../../src/ir/graph.rs).
  - Acceptance criteria: `CompilationGraph` in [`src/ir/graph.rs`](../../../src/ir/graph.rs) returns `PackageKind::Rule` packages ordered by `(SemanticIdentity, PackageId)` for any input observation order.
- [ ] 1.3 Add `contributing_packages: Vec<PackageId>` to `NativeArtifact` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) and `PlanEntry` in [`src/publication/model.rs`](../../../src/publication/model.rs) and keep `source_package: PackageId` as the primary provenance identifier.
  - Acceptance criteria: `NativeArtifact` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) and `PlanEntry` in [`src/publication/model.rs`](../../../src/publication/model.rs) record all sorted contributing `PackageId` values and serialize deterministically in [`src/publication/plan.rs`](../../../src/publication/plan.rs).
- [ ] 1.4 Define `MarketplaceCatalogWireV1` and `MarketplacePluginEntryWireV1` in [`src/parsers/agent_plugin/wire.rs`](../../../src/parsers/agent_plugin/wire.rs) and `MarketplaceCatalog` and `MarketplacePluginEntry` in [`src/parsers/agent_plugin/model.rs`](../../../src/parsers/agent_plugin/model.rs).
  - Acceptance criteria: `MarketplaceCatalog::try_from` in [`src/parsers/agent_plugin/model.rs`](../../../src/parsers/agent_plugin/model.rs) validates each plugin path with `ResourcePath::parse` and rejects duplicate `<plugin>` names, `..`, `.`, leading `/`, backslashes, and drive prefixes.

## 2. Phase 2: Rule bundling implementation in lowering

- [ ] 2.1 Define `RuleBundlingOptions` and `RuleHeaderStyle` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) and add them to `LoweringOptions` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs).
  - Acceptance criteria: `LoweringOptions` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) defaults to unbundled output for one `PackageKind::Rule` package and deterministic bundling for two or more `PackageKind::Rule` packages.
- [ ] 2.2 Implement `lower_bundled_rules` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) to bundle selected `PackageKind::Rule` packages into `AGENTS.md` for `NativeTarget::Codex` and `CLAUDE.md` for `NativeTarget::Claude`.
  - Acceptance criteria: `lower` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) emits one `NativeArtifactClass::Instruction` artifact for `codex` and `claude` without `artifact-collision` errors and keeps one-file-per-rule `NativeArtifactClass::Rule` emission for `cursor`, `opencode`, and `antigravity`.
- [ ] 2.3 Implement YAML `---` frontmatter conversion, `<!-- rulette:rule <logical-name> -->` section headers, and `\n\n---\n\n` delimiters with a single trailing `\n` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs).
  - Acceptance criteria: `lower_bundled_rules` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) strips raw YAML frontmatter blocks, rejects `-->` sequences and control characters in rule headers, joins sections with `\n\n---\n\n`, and emits a single rule without inter-rule delimiters by default.
- [ ] 2.4 Record per-package `CapabilityFinding` entries (`rule-description-unrepresentable` and `rule-activation-unrepresentable`) in `lower_bundled_rules` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) and update `matching_artifact_descriptor` and `collect_losses` in [`src/publication/stage.rs`](../../../src/publication/stage.rs).
  - Acceptance criteria: Strict lowering (`allow_lossy = false`) aborts when any bundled rule drops `description` or `activation` metadata, and lossy staging (`allow_lossy = true`) links each per-package `PlanLossFinding` to the shared `PlanEntry` in `rulette.plan.json`.

## 3. Phase 3: Marketplace catalog parser and emitter

- [ ] 3.1 Extend `native_frontend_candidates` in [`src/parsers/frontend.rs`](../../../src/parsers/frontend.rs) to match `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` for `--from` and `--from auto`.
  - Acceptance criteria: `compile_graph` in [`src/parsers/frontend.rs`](../../../src/parsers/frontend.rs) classifies `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` as recognized native observations without `unrecognized-native-file` warnings.
- [ ] 3.2 Update `compile_native` in [`src/parsers/agent_plugin/mod.rs`](../../../src/parsers/agent_plugin/mod.rs) and [`src/parsers/claude.rs`](../../../src/parsers/claude.rs) to parse `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` offline and bind discovered plugin skills to canonical `<plugin>` namespaces.
  - Acceptance criteria: `compile_native` in [`src/parsers/agent_plugin/mod.rs`](../../../src/parsers/agent_plugin/mod.rs) and [`src/parsers/claude.rs`](../../../src/parsers/claude.rs) preserves `/<plugin>:<skill>` identities and returns a parse error when `.claude-plugin/marketplace.json` contains `..`, an absolute path, or a duplicate `<plugin>` name.
- [ ] 3.3 Update `lower_agent_plugin` and `NativeTarget::Claude` lowering in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) to emit `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` artifacts.
  - Acceptance criteria: `lower` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) formats `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` with deterministic JSON key ordering, a trailing `\n`, and intact canonical `<plugin>` names.
- [ ] 3.4 Add `ArtifactPathRule::Exact` rules for `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` to `CLAUDE_PROJECT_RULES` and `AGENT_PLUGIN_PROJECT_RULES` in [`src/publication/mapping.rs`](../../../src/publication/mapping.rs).
  - Acceptance criteria: `mapping_for` in [`src/publication/mapping.rs`](../../../src/publication/mapping.rs) permits `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` under `claude@project` and `agent-plugin@project` and rejects them under user scope.

## 4. Phase 4: Declarative transform configuration

- [ ] 4.1 Add `RuleBundlingConfig` (`header_style`, `delimiter`, and `include_metadata_comments`) and `MarketplacePluginMapping` with `#[serde(deny_unknown_fields)]` to `TransformConfigFile` and `OutputEntry` in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs).
  - Acceptance criteria: `TransformConfigFile::load` in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs) loads `rule_bundling` and `marketplace_plugins` from explicit `--config <PATH>` manifests in `.toml`, `.json`, `.jsonc`, and `.json5` formats and rejects unknown fields or live root fields (`project_root`, `user_root`, `allow_project_root`, `allow_user_root`, `expect_plan_sha256`, and `replace`).
- [ ] 4.2 Validate `marketplace_plugins` mappings in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs) and [`src/lib.rs`](../../../src/lib.rs) with `ResourcePath::parse` and canonical `<plugin>` identifier rules before source compilation.
  - Acceptance criteria: `rulette transform --config <PATH>` fails with a non-zero exit code before compiling sources or creating a stage directory when `marketplace_plugins` contains `..`, leading `/`, backslashes, duplicate `<plugin>` keys, or invalid `<plugin>` names.
- [ ] 4.3 Pass validated `rule_bundling` and `marketplace_plugins` options from `TransformArgs::execute` in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs) to library compilation and `resolve_lowering_options`.
  - Acceptance criteria: `rulette transform --config <PATH> --stage <DIRECTORY> --project-root <DIRECTORY>` stages bundled `AGENTS.md` and `CLAUDE.md` artifacts and mapped marketplace plugins through library functions in [`src/lib.rs`](../../../src/lib.rs).
- [ ] 4.4 Document single-file rule bundling, `.claude-plugin/marketplace.json` transforms, and `--config <PATH>` options in [`docs/2026-04-11-man-page.md`](../../../docs/2026-04-11-man-page.md) and [`docs/2026-04-11-prd.md`](../../../docs/2026-04-11-prd.md).
  - Acceptance criteria: [`docs/2026-04-11-man-page.md`](../../../docs/2026-04-11-man-page.md) and [`docs/2026-04-11-prd.md`](../../../docs/2026-04-11-prd.md) describe the new bundling and marketplace capabilities and pass `mise run lint:md`.

## 5. Phase 5: Verification and test suite

- [ ] 5.1 Write TDD unit tests in [`src/ir/graph.rs`](../../../src/ir/graph.rs) and [`src/parsers/agent_plugin/model.rs`](../../../src/parsers/agent_plugin/model.rs) that test `(SemanticIdentity, PackageId)` rule ordering, canonical `<plugin>` namespace validation, and `MarketplaceCatalog` path containment rejection.
  - Acceptance criteria: `cargo test ir::graph` and `cargo test parsers::agent_plugin` pass with zero warnings.
- [ ] 5.2 Write TDD unit tests in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) and [`src/publication/stage.rs`](../../../src/publication/stage.rs) that test multi-rule bundling into `AGENTS.md` and `CLAUDE.md`, single-rule preservation, frontmatter conversion, `-->` injection rejection, `\n\n---\n\n` delimiters, and per-package `CapabilityFinding` and `PlanLossFinding` attribution.
  - Acceptance criteria: `cargo test emitters::lowering` and `cargo test publication::stage` pass and verify byte-identical bundled outputs across input permutations.
- [ ] 5.3 Write CLI integration tests in [`tests/cli_tests/transform_tests.rs`](../../../tests/cli_tests/transform_tests.rs), [`tests/cli_tests/transform_config_tests.rs`](../../../tests/cli_tests/transform_config_tests.rs), and [`tests/cli_tests/staged_publication_tests.rs`](../../../tests/cli_tests/staged_publication_tests.rs) that test multi-rule directory migrations, `.claude-plugin/marketplace.json` round-trips, symlink rejection, and `--config <PATH>` manifests across `.toml`, `.json`, `.jsonc`, and `.json5`.
  - Acceptance criteria: `cargo test --test '*'` passes all integration tests for staged publication and `--apply` workflows.
- [ ] 5.4 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` across the workspace.
  - Acceptance criteria: `mise run fmt`, `mise run lint`, and `mise run test` exit with code `0`.
- [ ] 5.5 Run `mise run spec-validate` and `mise run lint:md` to validate all OpenSpec specifications and Markdown files.
  - Acceptance criteria: `mise run spec-validate` and `mise run lint:md` exit with code `0` and report zero violations.
