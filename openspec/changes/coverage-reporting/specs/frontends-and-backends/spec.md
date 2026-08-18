## MODIFIED Requirements

### Requirement: Lossy conversion analysis via inspect verb

The `inspect` command SHALL print human-readable IR details and analyze field loss when targeting specific output formats.
When running `inspect --to <format>`, Rulette SHALL report any fields or entity kinds that cannot be represented in the destination format, sourced from each emitter's structured capability data rather than ad hoc warning strings.
Each `Emitter` implementation SHALL expose its lossy-conversion determinations as structured, per-entity-kind data usable both by single-target `inspect --to` output and by the all-targets coverage matrix (see the `coverage-reporting` capability).

#### Scenario: Inspecting capability loss

- **WHEN** `rulette inspect <input> --to <target>` is executed
- **THEN** Rulette SHALL list surviving fields and surface warnings for unmapped or dropped metadata
- **AND** the reported warnings SHALL be derived from the same structured capability data used by `--coverage` matrix reporting, not a separate ad hoc code path.
