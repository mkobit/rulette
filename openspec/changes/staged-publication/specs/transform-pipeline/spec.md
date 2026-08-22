## ADDED Requirements

### Requirement: Stage and apply publication lifecycle

The `transform` command SHALL keep a single top-level source-to-sink lifecycle while separating compilation from live native publication.
`transform <inputs> --target <format>@<scope> --stage <directory>` SHALL compile and stage native artifacts without publishing them to a live harness location.
`transform --apply <plan> --expect-plan-sha256 <digest>` SHALL be the only transform mode that may publish staged native artifacts to a live destination.
Graph output to standard output SHALL remain available for non-native pipeline composition.
Native layouts SHALL NOT be published to standard output as an alternative to stage or apply.

#### Scenario: Native compilation requires a stage

- **WHEN** a caller requests a native target that produces files
- **AND** does not request `--stage` or `--apply`
- **THEN** Rulette SHALL exit with a non-zero usage error
- **AND** SHALL NOT write a live native destination.

#### Scenario: Apply does not recompile sources

- **WHEN** a caller invokes `transform --apply <plan> --expect-plan-sha256 <digest>`
- **THEN** Rulette SHALL publish only the verified artifacts named by the plan
- **AND** SHALL NOT parse source inputs or rerun the transformation pipeline.

## MODIFIED Requirements

### Requirement: Unified transform command execution

The CLI SHALL provide a single `transform` command as the entry point for reading inputs, compiling a graph, selecting portable units, lowering targets, staging native artifacts, checking planned destinations, and applying an authorized publication plan.
The command SHALL accept inputs from stdin, single files, directories, and tar archives in source-compilation modes.
Apply mode SHALL be mutually exclusive with source inputs, `--from`, selection flags, `--target`, `--stage`, and transform configuration loading.

#### Scenario: Pipe streaming input

- **WHEN** input is passed via standard input (`-`) in a source-compilation mode
- **THEN** Rulette SHALL parse the input stream into the compilation graph
- **AND** SHALL apply the requested selection and lowering before emitting graph output or staging native artifacts.

#### Scenario: Apply rejects source-compilation arguments

- **WHEN** an invocation combines `--apply <plan>` with a source input, `--target`, or `--config`
- **THEN** Rulette SHALL exit with a non-zero usage error
- **AND** SHALL NOT read sources, stage artifacts, or publish a destination.

### Requirement: All-or-nothing multi-target emission

When staging multiple native targets, Rulette SHALL complete parsing, graph validation, selection, capability analysis, lowering, and stage-plan construction before it publishes the requested stage directory.
When applying multiple planned destinations, Rulette SHALL complete plan integrity, root authority, mapping, path-safety, artifact, and conflict preflight checks before mutating any destination.
If a stage or apply operation fails, Rulette SHALL exit with a non-zero status without a partial requested stage or a partial in-process publication set, subject to the staged-publication rollback guarantees.

#### Scenario: Invalid target prevents a stage

- **WHEN** a multi-target stage contains one target with an unsupported mapping or an unaccepted semantic loss
- **THEN** Rulette SHALL fail before creating the requested stage directory
- **AND** SHALL not publish any live destination.

#### Scenario: Preflight failure prevents all publication

- **WHEN** one planned destination is unauthorized, unsafe, conflicting without `--replace`, or backed by a changed staged artifact
- **THEN** Rulette SHALL fail apply before mutating any planned destination.

#### Scenario: Atomic multi-output write

- **WHEN** multiple planned destinations are requested by one authorized apply
- **THEN** Rulette SHALL validate every plan entry and destination before the first mutation
- **AND** SHALL publish the changed destinations through the transactional apply contract.

### Requirement: Drift-aware output writes

For authorized apply, the `transform` command SHALL compare every planned destination with the verified staged artifact before writing.
If an existing regular destination has identical bytes and target-representable executable metadata, Rulette SHALL report it as unchanged and leave it untouched.
If a destination is absent, Rulette SHALL report it as created after successful apply.
If a destination differs and `--replace` is explicit, Rulette SHALL report it as replaced after successful apply.
If a destination differs and `--replace` is absent, Rulette SHALL treat it as a hard conflict for the entire apply.
If an existing destination is unreadable or not a regular file, Rulette SHALL treat it as a hard error and SHALL NOT read through or write through it.
Failure after mutation begins SHALL use the staged-publication rollback contract rather than deleting an overwritten destination.

#### Scenario: Unchanged destination is not rewritten

- **WHEN** an apply destination exists as a regular file with identical planned bytes and executable metadata
- **THEN** Rulette SHALL NOT rewrite that file
- **AND** SHALL report the destination as unchanged.

#### Scenario: Unchanged target is not rewritten

- **WHEN** an apply destination already has byte-identical content and matching target-representable executable metadata
- **THEN** Rulette SHALL leave its content and modification time untouched
- **AND** SHALL report the destination as unchanged.

#### Scenario: Changed target is rewritten

- **WHEN** an apply destination differs from the staged artifact
- **AND** the caller passes `--replace`
- **THEN** Rulette SHALL write the verified staged artifact through a same-directory temporary file and atomic replacement
- **AND** SHALL report the destination as replaced.

#### Scenario: New target is created

- **WHEN** an apply destination does not exist
- **THEN** Rulette SHALL create it only after complete apply preflight succeeds
- **AND** SHALL report the destination as created.

#### Scenario: Multiple targets report independent statuses in one invocation

- **WHEN** one apply contains several destinations that are unchanged, absent, and differing with `--replace`
- **THEN** Rulette SHALL report each destination as unchanged, created, or replaced independently
- **AND** SHALL mutate only the created and replaced destinations.

#### Scenario: Rollback restores an overwritten target's original content

- **WHEN** apply replaces a destination and a later destination mutation fails
- **THEN** Rulette SHALL restore the replaced destination's original bytes and executable metadata only when its identity and digest still prove apply ownership
- **AND** SHALL exit with a non-zero status.

#### Scenario: Unreadable existing target aborts before any writes

- **WHEN** apply preflight finds a destination that exists but cannot be read as a regular file
- **THEN** Rulette SHALL exit with a non-zero status
- **AND** SHALL NOT write any destination in that apply.

#### Scenario: Non-regular-file existing target aborts before any writes

- **WHEN** apply preflight finds a destination that is a symlink, directory, junction, reparse point, or another non-regular file
- **THEN** Rulette SHALL exit with a non-zero status
- **AND** SHALL NOT read through or write through that path
- **AND** SHALL NOT write any destination in that apply.

#### Scenario: Replacement is explicit and transactional

- **WHEN** an apply destination differs from the planned artifact
- **AND** the caller passes `--replace`
- **THEN** Rulette SHALL replace the destination only after complete preflight
- **AND** SHALL include it in rollback if a later mutation fails.

### Requirement: Check mode reports drift without writing

The `transform` command SHALL support a check mode in which neither stage nor live destination files are written.
Source-mode check SHALL compute the same lowering and destination status as a corresponding staged compilation using explicit staging roots.
Plan-mode check SHALL verify the plan digest, staged artifacts, root authority, mapping, path safety, and destination status using explicit apply roots.
If every planned destination is unchanged, check SHALL exit with status zero.
If any destination is absent or conflicting, check SHALL exit with a non-zero status.
Check SHALL perform no filesystem mutation, including creating a stage directory, parent directory, temporary file, or destination.

#### Scenario: Check mode with no drift succeeds without writing

- **WHEN** `transform --check` verifies that every planned destination matches its artifact
- **THEN** Rulette SHALL exit with a zero status
- **AND** SHALL not write a stage or destination file.

#### Scenario: Check mode with drift fails without writing

- **WHEN** `transform --check` finds an absent or conflicting planned destination
- **THEN** Rulette SHALL exit with a non-zero status
- **AND** SHALL report the destination classifications without writing files.

#### Scenario: Check mode does not create parent directories

- **WHEN** `transform --check` finds an absent destination whose parent directory does not exist
- **THEN** Rulette SHALL not create the parent directory
- **AND** SHALL report the destination as absent.

#### Scenario: Check mode with only stdout targets fails

- **WHEN** `transform --check` is requested with only graph output to standard output and no native target mapping
- **THEN** Rulette SHALL exit with a non-zero usage error
- **AND** SHALL report that there is no live destination to check.
