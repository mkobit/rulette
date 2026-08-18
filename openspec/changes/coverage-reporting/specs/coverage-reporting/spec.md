## ADDED Requirements

### Requirement: All-targets capability matrix reporting

Rulette SHALL provide a mechanism to compute a capability matrix across every registered output target in a single invocation. For each entity kind present in the input IR, and for each registered target, the matrix SHALL report one of three statuses:

- **Dropped**: at least one entity of that kind contributes zero bytes to any file the target emitter produces.
- **Lossy**: no entity of that kind is Dropped, but at least one entity of that kind has a field or metadata item the target emitter does not carry over.
- **Supported**: every entity of that kind is fully represented in the target's output with no detected field loss.

When an input document contains multiple entities of the same kind, the reported status for that `(target, entity kind)` pair SHALL be the worst status across all instances of that kind (Dropped outranks Lossy outranks Supported).

#### Scenario: Generating a full coverage matrix

- **WHEN** `rulette inspect <input> --coverage` is executed
- **THEN** Rulette SHALL run the capability check against every registered emitter target without requiring a separate invocation per target
- **AND** print a human-readable matrix with entity kinds as rows and targets as columns, each cell showing Supported, Lossy, or Dropped.

#### Scenario: Coverage matrix reflects actual input

- **WHEN** the input IR contains only a subset of entity kinds (for example, only `rule` and `skill`)
- **THEN** the coverage matrix SHALL only include rows for entity kinds present in the input, not every entity kind the IR schema defines.

#### Scenario: Multiple entities of the same kind roll up to the worst status

- **WHEN** the input IR contains two `mcp-server` entities, one fully representable by a target and one that target only partially represents
- **THEN** the coverage matrix cell for that `(target, mcp-server)` pair SHALL report Lossy, not Supported.

#### Scenario: `--coverage` and `--to` are mutually exclusive

- **WHEN** `rulette inspect <input> --coverage --to <target>` is executed
- **THEN** Rulette SHALL reject the invocation with a usage error rather than silently choosing one mode.

### Requirement: Machine-readable coverage output

Rulette SHALL support emitting the coverage matrix as structured JSON, in addition to the human-readable table, so the result can be consumed by scripts and CI checks.

#### Scenario: JSON coverage output

- **WHEN** `rulette inspect <input> --coverage --json` is executed
- **THEN** Rulette SHALL print the coverage matrix as JSON with one entry per `(target, entity kind)` pair
- **AND** each entry SHALL carry `target` and `entity_kind` using the same kebab-case identifiers used elsewhere in the CLI (the IR's `kind` tag values and `OutputFormat`'s kebab-case target names), plus `status` (`supported`, `lossy`, or `dropped`)
- **AND** each entry with status `lossy` or `dropped` SHALL carry a non-null `reason` string describing what was lost, so a consumer does not need a separate `inspect --to <target>` call to learn why.

### Requirement: `--strict` gates coverage matrix exit status

Rulette SHALL reuse the existing global `--strict` flag to make `--coverage` usable as a CI check, consistent with how `--strict` already escalates lossy-conversion warnings to errors elsewhere in the CLI.

#### Scenario: Coverage check fails a CI gate

- **WHEN** `rulette inspect <input> --coverage --strict` is executed and any matrix cell is Lossy or Dropped
- **THEN** Rulette SHALL exit with a non-zero status code.

#### Scenario: Coverage check is informational without `--strict`

- **WHEN** `rulette inspect <input> --coverage` is executed without `--strict`, regardless of matrix contents
- **THEN** Rulette SHALL exit 0.
