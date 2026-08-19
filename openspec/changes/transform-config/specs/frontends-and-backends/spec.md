## ADDED Requirements

### Requirement: Inspect rejects transform-only output targets

The `inspect --to <format>` target format SHALL exclude `transform-config`: attempting `inspect --to transform-config` SHALL fail with a usage error explaining that `transform-config` is only a valid target for the `transform` command, rather than a compile-time omission or a silently wrong report.

#### Scenario: inspect rejects the transform-config target

- **WHEN** `rulette inspect <input> --to transform-config` is run
- **THEN** Rulette SHALL exit with a non-zero exit code and a usage error naming `transform-config` as unsupported for `inspect`
- **AND** SHALL NOT attempt to produce any coverage or field-loss report.
