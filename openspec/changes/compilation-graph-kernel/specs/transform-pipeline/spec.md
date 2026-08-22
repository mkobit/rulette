## MODIFIED Requirements

### Requirement: Unified transform command execution

The CLI SHALL provide one `transform` command as the entry point for reading native inputs, constructing a compilation graph, selecting packages, analyzing capability loss, lowering to target artifacts, and passing native artifacts to the publication layer.
The command SHALL accept inputs from standard input, single files, directories, and tar archives.
Graph output to standard output SHALL be lossless graph serialization and SHALL not publish a native target layout.

#### Scenario: Transforming a tar input through the graph

- **WHEN** a tar archive is passed to `transform`
- **THEN** Rulette SHALL discover safe observations, construct a compilation graph, and lower only after graph validation succeeds.

#### Scenario: Pipe streaming input

- **WHEN** standard input (`-`) is passed to `transform`
- **THEN** Rulette SHALL construct a compilation graph from the input stream before selection and lowering.

### Requirement: Declarative filtering and metadata transformations

The v0.1 transform command SHALL expose exact package selection as its only public graph transformation.
`--select <package-id>` SHALL be repeatable and SHALL select the deterministic union of exact package identifiers.
No `--select` argument SHALL select every package in stable package-identifier order.
An unknown package identifier SHALL be a hard error.
Selection SHALL not mutate instruction text, metadata, activation, resources, package roots, target payloads, or provenance.
The `--filter`, `--exclude`, `--rename`, and `--set` mutations SHALL not be accepted by the v0.1 transform command.

#### Scenario: Selecting two exact packages

- **WHEN** an invocation supplies two known `--select` package identifiers
- **THEN** Rulette SHALL lower the stable union of those two packages
- **AND** SHALL retain each selected package's provenance and attached resources.

#### Scenario: Rejecting a removed metadata mutation flag

- **WHEN** an invocation supplies `--rename` or `--set`
- **THEN** Rulette SHALL exit with a usage error before reading input.

#### Scenario: Filtering entities by status

- **WHEN** an invocation supplies the prior `--filter` expression flag
- **THEN** Rulette SHALL reject the invocation with a usage error because expression filtering is not a v0.1 public transform.

### Requirement: All-or-nothing multi-target emission

Rulette SHALL compute graph validation, selection, capability analysis, and every target lowering plan before the publication layer is allowed to write a native destination or staging artifact.
Any graph, selection, capability, or lowering failure SHALL prevent all destination and stage writes for that invocation.
Native destination transaction semantics and staging layout are defined by the staged-publication capability.

#### Scenario: One target's lowering failure blocks every target

- **WHEN** an invocation selects multiple targets and one target has a hard lowering failure
- **THEN** Rulette SHALL not write a staging artifact or native destination for any target.

#### Scenario: Atomic multi-output write

- **WHEN** multiple native targets are requested
- **THEN** Rulette SHALL validate graph selection, capability, and lowering for every target before the publication layer writes any target artifact.

## ADDED Requirements

### Requirement: Strict loss policy for transform

Transform compilation SHALL fail before staging or publication when any selected graph package is classified as `lossy` or `dropped` for any selected target.
`--allow-lossy` SHALL be the only v0.1 escape hatch for such representational loss.
When `--allow-lossy` is present, Rulette SHALL retain structured loss diagnostics in the compilation result and staged publication plan.
`--allow-lossy` SHALL not relax unsafe paths, malformed metadata, identity collisions, unsupported input syntax, or publication authorization failures.
The transform command SHALL not accept `--strict` because strict loss handling is its default behavior.

#### Scenario: Loss blocks strict transformation

- **WHEN** a selected package would be dropped or degraded by one requested target
- **AND** `--allow-lossy` is absent
- **THEN** Rulette SHALL fail before stage or destination output is written.

#### Scenario: Explicit lossy transformation retains diagnostics

- **WHEN** a selected package would be degraded by one requested target
- **AND** `--allow-lossy` is present
- **THEN** Rulette SHALL continue to lowering
- **AND** SHALL retain the structured loss findings for review.

### Requirement: Selection-only transform configuration

An explicit transform configuration MAY provide source inputs, target requests, logical scope requests, and a sorted `select` array of exact package identifiers.
Transform configuration SHALL reject prior mutation pipeline fields and global or per-output `strict` fields.
Transform configuration SHALL never grant loss permission, publication authority, stage-root selection, or destination-path authority.

#### Scenario: Configured exact selection

- **WHEN** a transform configuration contains a sorted array of known package identifiers in `select`
- **THEN** Rulette SHALL apply the same exact selection semantics as repeatable CLI `--select` arguments.

#### Scenario: Rejecting a config mutation pipeline

- **WHEN** a transform configuration contains a `pipeline` mutation step or `strict` field
- **THEN** Rulette SHALL reject the configuration before compilation.

### Requirement: Graph schema output

The `ir-json`, `ir-toml`, and `schema` surfaces SHALL expose the versioned compilation graph contract rather than the removed `RuletteDocument` entity envelope.
Graph JSON serialization SHALL use stable struct-field order, sorted maps, UTF-8 text, canonical base64 byte payloads, and a terminal newline.
Graph TOML serialization SHALL use the same graph ordering and canonical byte encoding.
The prior `RuletteDocument` JSON, TOML, and direct entity interchange APIs SHALL not be supported by v0.1.

#### Scenario: Repeated graph serialization is byte-identical

- **WHEN** Rulette serializes the same validated compilation graph twice with the same compiler version
- **THEN** both JSON outputs SHALL be byte-identical.
