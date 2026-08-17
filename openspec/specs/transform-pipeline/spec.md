# transform-pipeline Specification

## Purpose

Define the execution model for the `transform` verb, including pipeline filtering, metadata mutation, streaming standard I/O, and atomic multi-target output generation.

## Requirements

### Requirement: Unified transform command execution

The CLI SHALL provide a single `transform` command as the entry point for reading, filtering, mutating, and emitting rule configurations.
The command SHALL accept inputs from stdin, single files, directories, and tar archives.

#### Scenario: Pipe streaming input

- **WHEN** input is passed via standard input (`-`)
- **THEN** Rulette SHALL parse the input stream into a `RuletteDocument`
- **AND** apply requested transformations before emitting output.

### Requirement: Declarative filtering and metadata transformations

The `transform` pipeline SHALL support `--filter`, `--exclude`, `--rename`, and `--set` expression flags.
Transformations SHALL apply in order to the IR entities prior to emission.

#### Scenario: Filtering entities by status

- **WHEN** `--filter 'status == "stable"'` is specified
- **THEN** only entities matching the filter predicate SHALL be retained for emission.

### Requirement: All-or-nothing multi-target emission

When emitting to multiple targets via `-o/--out`, Rulette SHALL compute all output target transformations before writing files.
If any single target emission fails, the entire command SHALL exit with a non-zero exit code without leaving partial writes on disk.

#### Scenario: Atomic multi-output write

- **WHEN** multiple `-o target:path` targets are requested
- **THEN** Rulette SHALL validate all targets before writing any files
- **AND** if validation passes, it SHALL write all outputs atomically.
