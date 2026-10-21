## Why

`rulette` is a static, offline compiler for AI-agent guidance packages (`rule` and `skill` packages) in [`src/ir/graph.rs`](../../../src/ir/graph.rs).
Currently, `lower_rule` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) maps every `PackageKind::Rule` package to `AGENTS.md` for `codex` and to `CLAUDE.md` for `claude`.
When a `CompilationGraph` contains more than one `PackageKind::Rule` package, `validate_no_collisions` in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) fails with an `artifact-collision` error.
This error prevents teams from compiling multi-rule inputs (such as `.cursor/rules/*.mdc`, `.opencode/rules/*.md`, `.agents/rules/*.md`, or `.claude/rules/*.md`) to single-file targets (`AGENTS.md` and `CLAUDE.md`).
Teams also vendor remote plugin repositories into local directories for security review and offline audits.
Teams must consume these vendored plugins locally under their original canonical plugin names so that skill invocations (`/<plugin>:<skill>`) do not break.
Local marketplace catalogs (`.claude-plugin/marketplace.json`) map canonical plugin names to local relative paths.
Decision bead `rulette-w6d` approved passive `agent-plugin` (`AgentPlugin`) packaging and explicit declarative `--config <PATH>` manifests in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs).
`rulette` must bundle multiple rules for single-file targets and transform local marketplace catalogs without network access or runtime dependencies.

## What changes

- Combine multiple `PackageKind::Rule` packages deterministically into one `NativeArtifactClass::Instruction` artifact (`AGENTS.md` for `codex` and `CLAUDE.md` for `claude`) in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) instead of failing with `artifact-collision`.
- Order bundled rule sections by deterministic `PackageId` and `SemanticIdentity` keys from `CompilationGraph` in [`src/ir/graph.rs`](../../../src/ir/graph.rs).
- Record per-package `CapabilityFinding` entries (including `rule-description-unrepresentable` and `rule-activation-unrepresentable`) for every rule package in a bundled instruction file in [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs) and [`src/publication/stage.rs`](../../../src/publication/stage.rs).
- Add passive parsing and emission for local marketplace catalogs (`.claude-plugin/marketplace.json`) and Claude plugin manifests (`.claude-plugin/plugin.json` and `plugin.json`) in [`src/parsers/agent_plugin/mod.rs`](../../../src/parsers/agent_plugin/mod.rs) and [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs).
- Map canonical plugin names in `.claude-plugin/marketplace.json` to local relative paths so that vendored plugin skills keep their `/<plugin>:<skill>` invocation names.
- Extend the declarative `--config <PATH>` manifest in [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs) to declare rule bundling options and local plugin path mappings alongside `inputs`, `targets`, sorted `select` selectors, and per-target `allow_lossy` overrides.
- Update target mapping rules in [`src/publication/mapping.rs`](../../../src/publication/mapping.rs) to permit `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` artifacts under authorized project roots.

## Scope

- Bundle multiple `PackageKind::Rule` packages into one deterministic `AGENTS.md` or `CLAUDE.md` instruction artifact for `codex@project`, `codex@user`, `claude@project`, and `claude@user`.
- Keep one-file-per-rule emission for multi-file rule targets (`cursor@project` at `.cursor/rules/<name>.mdc`, `opencode@project` at `.opencode/rules/<name>.md`, and `antigravity@project` at `.agents/rules/<name>.md`).
- Read and validate `.claude-plugin/marketplace.json` catalogs that map canonical `<plugin>` names to safe relative paths inside explicit local snapshot inputs.
- Preserve canonical `<plugin>` namespaces across vendored plugin packages so that `/<plugin>:<skill>` skill invocations remain valid.
- Load explicit declarative build manifests through `rulette transform --config <PATH>` in `.toml`, `.json`, `.jsonc`, or `.json5` format as approved in `rulette-w6d`.
- Reject capability loss by default (`allow_lossy = false`) and require `--allow-lossy` or per-target `allow_lossy = true` in `--config <PATH>` when lowering drops rule metadata or synthesizes plugin manifests.
- Implement all compilation, bundling, and marketplace transformation logic in the library while [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs) remains a thin CLI wrapper.

## Non-goals

- `rulette` does not access the network, query remote marketplaces, or fetch remote repositories.
- `rulette` does not load dynamic plugins, shared libraries (`.so`, `.dylib`, `.dll`), or external executables.
- `rulette` does not run a background daemon, require an initialization phase, or store local state.
- `rulette` does not auto-discover ambient configuration files and reads a manifest only when the caller passes `--config <PATH>`.
- `rulette` does not execute plugin hooks, start Model Context Protocol (`MCP`) servers, or run `/<plugin>:<skill>` invocations.
- `rulette` does not grant filesystem write authority from `--config <PATH>` manifests or `.claude-plugin/marketplace.json` catalogs.
- Live destination publication requires `rulette transform --stage <DIRECTORY>` followed by `rulette transform --apply <STAGE_DIR/rulette.plan.json>` with `--expect-plan-sha256 <DIGEST>` and `--allow-project-root <DIRECTORY>` or `--allow-user-root <target=PATH>`.

## Capabilities

### New capabilities

- `rule-bundling`: Deterministic bundling of multiple `PackageKind::Rule` packages into single-file instruction artifacts (`AGENTS.md` and `CLAUDE.md`) with per-package provenance and capability loss reporting.
- `marketplace-transforms`: Offline parsing, canonical `<plugin>` name-to-path mapping, and passive emission of `.claude-plugin/marketplace.json` catalogs and vendored `agent-plugin` packages.

### Modified capabilities

- `ir-core`: Preserve canonical `<plugin>` namespace metadata and multi-package provenance links in `CompilationGraph` ([`openspec/specs/ir-core/spec.md`](../../../openspec/specs/ir-core/spec.md) and [`src/ir/graph.rs`](../../../src/ir/graph.rs)).
- `transform-pipeline`: Accept declarative rule-bundling and local marketplace mapping options through `rulette transform --config <PATH>` while preserving staged publication checks ([`openspec/specs/transform-pipeline/spec.md`](../../../openspec/specs/transform-pipeline/spec.md) and [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs)).
- `frontends-and-backends`: Replace single-file rule `artifact-collision` failures with deterministic rule bundling for `codex` and `claude`, and support `.claude-plugin/marketplace.json` in `agent-plugin` and `claude` adapters ([`openspec/specs/frontends-and-backends/spec.md`](../../../openspec/specs/frontends-and-backends/spec.md), [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs), and [`src/parsers/agent_plugin/mod.rs`](../../../src/parsers/agent_plugin/mod.rs)).

## Impact

- [`src/emitters/lowering.rs`](../../../src/emitters/lowering.rs): Update `lower`, `lower_rule`, `validate_no_collisions`, and `lower_agent_plugin` to bundle multiple `PackageKind::Rule` packages for `NativeTarget::Codex` (`AGENTS.md`) and `NativeTarget::Claude` (`CLAUDE.md`) and to emit marketplace catalog artifacts.
- [`src/parsers/agent_plugin/mod.rs`](../../../src/parsers/agent_plugin/mod.rs), [`src/parsers/agent_plugin/wire.rs`](../../../src/parsers/agent_plugin/wire.rs), and [`src/parsers/agent_plugin/model.rs`](../../../src/parsers/agent_plugin/model.rs): Add wire and domain types for `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` with relative path validation and canonical `<plugin>` name mapping.
- [`src/parsers/claude.rs`](../../../src/parsers/claude.rs) and [`src/parsers/frontend.rs`](../../../src/parsers/frontend.rs): Recognize `.claude-plugin/marketplace.json` and vendored plugin directories during explicit `--from` and `--from auto` frontend decoding.
- [`src/cli/commands/transform.rs`](../../../src/cli/commands/transform.rs): Extend `TransformConfigFile` and `OutputEntry` to parse declarative rule-bundling and marketplace catalog settings from `--config <PATH>`.
- [`src/publication/mapping.rs`](../../../src/publication/mapping.rs), [`src/publication/model.rs`](../../../src/publication/model.rs), and [`src/publication/stage.rs`](../../../src/publication/stage.rs): Permit `.claude-plugin/marketplace.json` in project target mappings and associate per-package `PlanLossFinding` entries with bundled `PlanEntry` artifacts.
- [`openspec/specs/ir-core/spec.md`](../../../openspec/specs/ir-core/spec.md), [`openspec/specs/transform-pipeline/spec.md`](../../../openspec/specs/transform-pipeline/spec.md), and [`openspec/specs/frontends-and-backends/spec.md`](../../../openspec/specs/frontends-and-backends/spec.md): Add specification requirements for single-file rule bundling, local marketplace catalog transforms, and `--config <PATH>` manifest options as approved in `rulette-w6d`.
- [`docs/2026-04-11-man-page.md`](../../../docs/2026-04-11-man-page.md) and [`docs/2026-04-11-prd.md`](../../../docs/2026-04-11-prd.md): Document single-file rule bundling and local marketplace catalog transforms.
