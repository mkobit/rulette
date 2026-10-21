# Rule bundling and marketplace transforms design

## Context

`rulette` compiles AI-agent guidance packages (`rule` and `skill` packages) into a deterministic `CompilationGraph` in [`src/ir/graph.rs`](../../../src/ir/graph.rs) and stages target artifacts through `rulette transform`.
In the current implementation, `lower_rule` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) maps every `PackageKind::Rule` package to `AGENTS.md` for `codex` and to `CLAUDE.md` for `claude`.
When a `CompilationGraph` contains two or more `PackageKind::Rule` packages, `validate_no_collisions` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) rejects the build with an `artifact-collision` error.
This behavior blocks migrations from multi-rule directories (such as `.cursor/rules/*.mdc`, `.opencode/rules/*.md`, `.agents/rules/*.md`, and `.claude/rules/*.md`) to single-file instruction targets (`AGENTS.md` and `CLAUDE.md`).
Teams also vendor remote plugin repositories into local directories for offline security review and reproducible builds.
When teams consume vendored plugins from local relative paths, skill invocations must retain their canonical `/<plugin>:<skill>` names.
Local marketplace catalogs at `.claude-plugin/marketplace.json` map canonical `<plugin>` names to local relative plugin directories.
Decision bead `rulette-w6d` approved passive `agent-plugin` (`AgentPlugin`) packaging and explicit declarative `--config <PATH>` manifests in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs).
All logic must reside in the static Rust library with zero runtime dependencies, zero network access, and no ambient configuration discovery.

## Decisions

### 1. Deterministic single-file rule bundling

- `lower` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) groups all selected `PackageKind::Rule` packages when targeting `NativeTarget::Codex` (`AGENTS.md`) or `NativeTarget::Claude` (`CLAUDE.md`).
- Instead of emitting separate colliding `NativeArtifactClass::Instruction` artifacts, `lower` bundles the rule packages into one `NativeArtifactClass::Instruction` artifact.
- Contributing `PackageKind::Rule` packages are sorted deterministically by `(SemanticIdentity, PackageId)` from `CompilationGraph` in [`src/ir/graph.rs`](../../../src/ir/graph.rs).
- Multi-file rule targets (`NativeTarget::Cursor` at `.cursor/rules/<name>.mdc`, `NativeTarget::OpenCode` at `.opencode/rules/<name>.md`, and `NativeTarget::Antigravity` at `.agents/rules/<name>.md`) keep one-file-per-rule emission.
- When `CompilationGraph` contains only one `PackageKind::Rule` package and default bundling options are active, `lower` emits the single rule body directly without inter-rule delimiters.

### 2. Frontmatter conversion and clean delimiter format

- Single-file instruction targets (`AGENTS.md` and `CLAUDE.md`) do not parse per-section YAML `---` frontmatter blocks.
- During multi-rule bundling, the renderer strips raw YAML frontmatter from each rule and converts rule identity and frontmatter metadata into a deterministic Markdown header or HTML comment block (`<!-- rulette:rule <logical-name> -->`).
- Consecutive bundled rule sections are separated by a normalized `\n\n---\n\n` delimiter and end with a single `\n` newline.
- `rule_field_findings` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) evaluates each contributing `PackageKind::Rule` package and records `rule-description-unrepresentable` and `rule-activation-unrepresentable` findings with the originating `PackageId` and `SourceProvenance`.
- Strict mode (`allow_lossy = false`) rejects lowering if any bundled rule drops unrepresentable metadata unless the caller passes `--allow-lossy` or sets `allow_lossy = true` for that target in `--config <PATH>`.

### 3. Passive local marketplace catalog transforms and canonical plugin names

- `src/parsers/agent_plugin/wire.rs` and `src/parsers/agent_plugin/model.rs` define strongly typed wire and domain structs (`MarketplaceCatalogWireV1`, `MarketplacePluginEntryWireV1`, `MarketplaceCatalog`, and `MarketplacePluginEntry`) for `.claude-plugin/marketplace.json`.
- `src/parsers/agent_plugin/mod.rs` and `src/parsers/claude.rs` parse `.claude-plugin/marketplace.json`, `.claude-plugin/plugin.json`, and `plugin.json` passively from explicit local inputs.
- Every plugin source path in `.claude-plugin/marketplace.json` must validate as a normalized relative `ResourcePath` beneath the input root without `..`, leading `/`, backslashes, drive prefixes, or symbolic links.
- Each vendored plugin retains its canonical `<plugin>` name in `CompilationGraph` metadata so that skill invocations using `/<plugin>:<skill>` remain valid across transformations.
- `src/publication/mapping.rs` permits `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` under `CLAUDE_PROJECT_RULES` and `AGENT_PLUGIN_PROJECT_RULES`.

### 4. Declarative `--config <PATH>` options for bundling and marketplace mappings

- `TransformConfigFile` and `OutputEntry` in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs) accept optional `rule_bundling` settings (`header_style`, `delimiter`, and `include_metadata_comments`) and `marketplace_plugins` mappings (`<plugin>` to relative path).
- `rulette` reads a configuration manifest only when the caller explicitly passes `rulette transform --config <PATH>` in `.toml`, `.json`, `.jsonc`, or `.json5` format.
- Configuration manifests never supply live root authorization (`--project-root`, `--user-root`, `--allow-project-root`, `--allow-user-root`, or `--replace`).

## Technical architecture

### Intermediate representation (`src/ir/graph.rs`)

- Preserve canonical `<plugin>` namespace metadata on `Package` units in `FrontendPayload` (`namespace: "agent-plugin.marketplace"` and `"agent-plugin.manifest"`) and validate `<plugin>` names against the skill and plugin identifier grammar.
- Maintain deterministic `(SemanticIdentity, PackageId)` iteration order for `PackageKind::Rule` packages in `CompilationGraph`.
- Retain `contributing_packages: Vec<PackageId>` on bundled `NativeArtifact` and `PlanEntry` records while preserving `source_package: PackageId` for primary deterministic provenance binding.

### Frontend parsers (`src/parsers/agent_plugin/mod.rs`, `src/parsers/claude.rs`, and `src/parsers/frontend.rs`)

- Extend `native_frontend_candidates` in [`src/parsers/frontend.rs`](../../../src/parsers/frontend.rs) to recognize `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json`.
- Parse `.claude-plugin/marketplace.json` entries in `src/parsers/agent_plugin/mod.rs` and `src/parsers/claude.rs`, verifying that each plugin entry maps a valid `<plugin>` identifier to a safe relative `ResourcePath`.
- Bind skills discovered inside a mapped plugin directory to the canonical `<plugin>` namespace so `/<plugin>:<skill>` references resolve deterministically.

### Lowering engine (`src/emitters/lowering.rs`)

- Partition `PackageKind::Rule` packages in `lower` by target rule emission mode:
  - Single-file instruction targets (`NativeTarget::Codex` and `NativeTarget::Claude`) call `lower_bundled_rules` to produce one `NativeArtifactClass::Instruction` artifact at `AGENTS.md` or `CLAUDE.md`.
  - Multi-file rule targets (`NativeTarget::Cursor`, `NativeTarget::OpenCode`, and `NativeTarget::Antigravity`) call `lower_rule` per package.
- In `lower_bundled_rules`, sort packages by `(SemanticIdentity, PackageId)`, render each rule section with configured header comments and `\n\n---\n\n` delimiters, and emit per-package `CapabilityFinding` entries referencing the shared artifact path (`AGENTS.md` or `CLAUDE.md`).
- Emit `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` artifacts with canonical JSON formatting and deterministic key ordering.

### Publication mappings and staging (`src/publication/mapping.rs`, `src/publication/model.rs`, and `src/publication/stage.rs`)

- Add exact mapping rules in [`src/publication/mapping.rs`](../../../src/publication/mapping.rs) for `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` in project scope.
- Update `matching_artifact_descriptor` and `collect_losses` in [`src/publication/stage.rs`](../../../src/publication/stage.rs) so that per-package `CapabilityFinding` records from any rule in a bundled `AGENTS.md` or `CLAUDE.md` artifact resolve to the shared staged `PlanEntry`.

### CLI configuration (`src/cli/commands/transform.rs`)

- Extend `TransformConfigFile` and `OutputEntry` in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs) with `#[serde(deny_unknown_fields)]` structs `RuleBundlingConfig` and `MarketplacePluginMapping`.
- Forward validated bundling and marketplace options into library compilation and lowering options without adding business logic to the CLI layer.

## Adversarial review analysis and mitigations

### 1. Path traversal and symlink escape in `.claude-plugin/marketplace.json`

- **Risk:** A crafted `.claude-plugin/marketplace.json` file or `--config <PATH>` mapping could specify `../../etc/passwd`, an absolute path `/root`, or a symlinked directory to read or overwrite files outside the repository root.
- **Mitigation:** `ResourcePath::parse` and `PluginPath` reject empty components, `.`, `..`, leading `/`, backslashes, and platform drive prefixes at parse time.
- **Mitigation:** `observe_sources` in [`src/inputs/mod.rs`](../../../src/inputs/mod.rs) and `open_root` in [`src/publication/fs.rs`](../../../src/publication/fs.rs) reject symbolic links and non-regular files before any frontend or apply operation runs.

### 2. Delimiter and frontmatter injection in bundled `AGENTS.md` and `CLAUDE.md`

- **Risk:** A rule body containing raw `---` lines or unclosed HTML comments could corrupt subsequent bundled rule sections or spoof provenance headers in `AGENTS.md` and `CLAUDE.md`.
- **Mitigation:** The bundler strips parsed YAML frontmatter before rendering, normalizes trailing whitespace on each rule body, sanitizes logical rule names in `<!-- rulette:rule <logical-name> -->` headers to reject `-->` sequences and control characters, and inserts a canonical `\n\n---\n\n` delimiter between sections.

### 3. Canonical `<plugin>` namespace collisions across vendored plugins

- **Risk:** Two marketplace catalog entries or `--config <PATH>` mappings could claim the same canonical `<plugin>` name or produce duplicate `/<plugin>:<skill>` identities.
- **Mitigation:** `CompilationGraph::validate` in [`src/ir/graph.rs`](../../../src/ir/graph.rs) and the marketplace catalog parser enforce strict uniqueness on canonical `<plugin>` names and `SemanticIdentity` keys, aborting compilation on any duplicate identity.

### 4. Orphaned capability loss findings on bundled artifacts during staging

- **Risk:** In [`src/publication/stage.rs`](../../../src/publication/stage.rs), `matching_artifact_descriptor` currently matches `artifact.source_package == finding.package_id`.
- **Risk:** When multiple `PackageKind::Rule` packages bundle into one `AGENTS.md` or `CLAUDE.md` artifact, findings for the second and subsequent rule packages would fail stage lookup if the artifact only records one `source_package`.
- **Mitigation:** Bundled `NativeArtifact` and `PlanEntry` values record all contributing `PackageId` values, and `matching_artifact_descriptor` matches any finding whose `package_id` is in the artifact's contributing package set and whose `artifact_path` equals the bundled artifact path.

### 5. Implicit authority escalation through `--config <PATH>` or `.claude-plugin/marketplace.json`

- **Risk:** A checked-in `--config <PATH>` manifest or `.claude-plugin/marketplace.json` catalog could attempt to set live destination roots or bypass `--expect-plan-sha256` verification.
- **Mitigation:** `TransformConfigFile` enforces `#[serde(deny_unknown_fields)]` and contains no fields for `--project-root`, `--user-root`, `--allow-project-root`, `--allow-user-root`, `--expect-plan-sha256`, or `--replace`.
- **Mitigation:** Live publication always requires explicit CLI flags on `rulette transform --apply <STAGE_DIR/rulette.plan.json>`.
