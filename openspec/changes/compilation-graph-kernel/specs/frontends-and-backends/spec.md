## MODIFIED Requirements

### Requirement: Format auto-detection and explicit format overrides

Rulette v0.1 SHALL provide native frontends and backends for Codex, OpenCode, Claude, Cursor, and Antigravity.
Each frontend SHALL auto-detect only native layouts that its documented harness loads and SHALL allow source-format override through `--from`.
Each frontend SHALL own native discovery, package grouping, primary-instruction identification, native metadata decoding, and construction of target-independent graph values.
Each backend SHALL own native layout, native metadata encoding, resource eligibility, and capability classification.
Target selection and publication syntax are defined by the transform-pipeline and staged-publication capabilities.

#### Scenario: Auto-detecting a core native layout

- **WHEN** an input path has a documented native layout for one core v0.1 harness
- **AND** no `--from` override is supplied
- **THEN** Rulette SHALL select that harness frontend and construct graph packages according to its documented native layout.

#### Scenario: Explicit source override

- **WHEN** a caller supplies `--from <format>` for a supported core frontend
- **THEN** Rulette SHALL parse the supplied input through that frontend rather than relying on auto-detection.

#### Scenario: Auto-detecting Cursor MDC files

- **WHEN** a `.cursor/rules/*.mdc` input is passed without `--from`
- **THEN** Rulette SHALL select the Cursor frontend
- **AND** SHALL construct a graph rule package from its native frontmatter and instruction resource.

### Requirement: Lossy conversion analysis via inspect verb

The `inspect` command SHALL analyze a compilation graph against one or more registered target backends using the same structured per-package and per-resource capability data used by transform lowering.
When inspect targets a format, it SHALL report every `supported`, `lossy`, or `dropped` result with stable reason information and source package provenance.

#### Scenario: Inspecting graph-package capability loss

- **WHEN** `rulette inspect <input> --to <target>` analyzes a package with a resource the backend cannot represent
- **THEN** Rulette SHALL report the resource as lossy or dropped using the same capability result that transform uses.

#### Scenario: Inspecting capability loss

- **WHEN** `rulette inspect <input> --to <target>` analyzes a graph package
- **THEN** Rulette SHALL list representable package content
- **AND** SHALL report unmapped or dropped content with its structured capability reason.

## ADDED Requirements

### Requirement: Native frontend conformance

Each core frontend SHALL convert native files into validated graph packages without embedding native destination paths in the graph.
Each frontend SHALL preserve recognized primary instructions, portable rule or skill semantics, source provenance, retained opaque resource bytes, normalized resource paths, and executable-bit metadata.
Each frontend SHALL produce an unsupported-semantic diagnostic instead of a portable entity for agents, hooks, MCP servers, permissions, and other unsupported native semantics.
Each frontend SHALL preserve an unknown native frontmatter or configuration field only in a namespaced frontend payload attached to its owning package or semantic item.
That payload SHALL not change the portable rule or skill meaning.

#### Scenario: Retaining an opaque same-domain package resource

- **WHEN** a core frontend recognizes a safe non-primary file inside a rule or skill package
- **THEN** it SHALL retain the file as an opaque resource with its normalized path and exact bytes
- **AND** SHALL not infer portable semantics for the file.

### Requirement: Deterministic backend conformance

Each core backend SHALL lower selected graph packages deterministically to typed native artifacts without writing files.
Each backend SHALL return a capability result for every selected package and resource before publication.
Each backend SHALL report a loss instead of inventing a target layout for an unrepresentable package resource or semantic item.
Backends SHALL not use network access, initialization, local state, automatic configuration discovery, or caller filesystem paths during lowering.

#### Scenario: Repeated core backend lowering

- **WHEN** the same selected graph, target, and compiler version are lowered twice
- **THEN** the backend SHALL return artifact descriptors, bytes, and capability findings in byte-identical deterministic order.

### Requirement: Core-target capability inspection

`inspect --coverage` SHALL provide a capability matrix for every registered core target based on graph packages and resources present in the input.
Each matrix cell SHALL report the worst `supported`, `lossy`, or `dropped` classification for its target and observed package kind.
`inspect --coverage --strict` SHALL exit non-zero when any cell is `lossy` or `dropped`.
Without `--strict`, coverage reporting SHALL be informational.

#### Scenario: Coverage reports a dropped opaque resource

- **WHEN** a selected input package contains an opaque resource that a target cannot represent
- **THEN** `inspect --coverage` SHALL report the affected package kind and target as dropped or lossy
- **AND** SHALL expose a stable reason with source provenance.
