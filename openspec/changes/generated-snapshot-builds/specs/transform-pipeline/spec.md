## MODIFIED Requirements

### Requirement: Unified transform command execution

The `transform` command SHALL parse source-decoder and target syntax before source I/O and SHALL compile every caller-supplied source input into one validated `CompilationGraph` before backend resolution, capability analysis, lowering, or output handling.

For a generated-snapshot build, the command SHALL select one homogeneous statically compiled native frontend through explicit `--from` selection or an unambiguous `--from auto` resolution, or one explicit graph JSON or graph TOML reader for all source inputs.

For a generated-snapshot build, the command SHALL normalize and deduplicate target spellings under existing behavior and accept one or more unique resolved targets from the compiled-in backend registry after complete graph aggregation and collision validation.

The command SHALL never execute, discover, or synchronize with a source generator, and it SHALL never treat a staged or published target artifact as an implicit source.

#### Scenario: Pipe streaming input

- **WHEN** one source is passed through standard input as an explicitly selected tar archive, gzip-compressed tar archive, graph JSON document, or graph TOML document
- **THEN** Rulette SHALL parse the stream into the aggregated `CompilationGraph`
- **AND** SHALL apply requested target lowering only after complete aggregation and collision validation.

#### Scenario: Transforming multiple explicit snapshots to multiple targets

- **WHEN** a caller supplies multiple valid homogeneous generated snapshots and two supported targets
- **THEN** the command SHALL aggregate the snapshots into one graph before backend resolution
- **AND** SHALL produce two independent target artifact sets through the existing atomic publication flow.

#### Scenario: A destination is not an implicit transform input

- **WHEN** a caller runs a generated-snapshot build after manually editing a prior target artifact
- **THEN** the command SHALL use only the explicit sources supplied for the new invocation
- **AND** SHALL NOT import or merge the edited target artifact.
