## ADDED Requirements

### Requirement: Canonical plugin namespace metadata in the compilation graph

`CompilationGraph` in `src/ir/graph.rs` SHALL preserve canonical `<plugin>` namespace metadata on `Package` units decoded from `.claude-plugin/marketplace.json`, `.claude-plugin/plugin.json`, and `plugin.json`.
Each `PackageKind::Skill` package from a vendored plugin SHALL retain its canonical `<plugin>` name so `/<plugin>:<skill>` invocations remain valid.
`CompilationGraph` validation SHALL reject `<plugin>` names that are empty or contain `:`, `..`, `/`, `\`, or control characters.

#### Scenario: Preserving canonical plugin name for vendored skill packages

- **WHEN** `rulette` compiles a vendored plugin directory with canonical name `<plugin>` and skill `<skill>` into a `CompilationGraph`
- **THEN** `rulette` SHALL store the canonical `<plugin>` namespace in `FrontendPayload` and `SemanticIdentity` metadata on the skill `Package`
- **AND** SHALL preserve the `/<plugin>:<skill>` invocation identifier across graph serialization and target lowering.

#### Scenario: Rejecting invalid canonical plugin namespace identifiers

- **WHEN** a plugin manifest or `.claude-plugin/marketplace.json` catalog entry declares an empty `<plugin>` name or a `<plugin>` name containing `/`, `\`, `..`, `:`, or control characters
- **THEN** `rulette` SHALL fail `CompilationGraph` validation before package selection or target lowering
- **AND** SHALL report the invalid `<plugin>` identifier in the error output.

### Requirement: Multi-package provenance and deterministic ordering for bundled artifacts

`CompilationGraph` in `src/ir/graph.rs` and `NativeArtifact` in `src/emitters/lowering.rs` SHALL preserve multi-package provenance links when multiple `PackageKind::Rule` packages bundle into one `NativeArtifactClass::Instruction` artifact.
`rulette` SHALL sort bundled `PackageKind::Rule` packages by `(SemanticIdentity, PackageId)`.
Every `CapabilityFinding` and `PlanLossFinding` for a bundled artifact SHALL retain the originating `PackageId` and `SourceProvenance`.

#### Scenario: Deterministic package ordering for bundled rules

- **WHEN** a `CompilationGraph` contains multiple `PackageKind::Rule` packages discovered from different files or archives
- **THEN** `rulette` SHALL sort the contributing rule packages by `(SemanticIdentity, PackageId)` before rendering the bundled instruction artifact
- **AND** SHALL produce byte-identical output regardless of input discovery order.

#### Scenario: Retaining per-package provenance on bundled capability findings

- **WHEN** multiple `PackageKind::Rule` packages are bundled into one `AGENTS.md` or `CLAUDE.md` artifact and one package omits `description` or `activation` metadata
- **THEN** `rulette` SHALL attach the exact `PackageId` and `SourceProvenance` of that rule package to its `CapabilityFinding`
- **AND** SHALL link the `PlanLossFinding` to the shared bundled `PlanEntry` in `rulette.plan.json`.
