# frontends-and-backends Specification

## Purpose

Define the format detection, parsing frontends, target emitters, and capability inspection mechanisms in Rulette.

## Requirements

### Requirement: Format auto-detection and explicit format overrides

Rulette SHALL automatically infer the source format based on file paths, file extensions, and content structure.
Users SHALL be able to override auto-detection using explicit `--from` and `--to` flags.

#### Scenario: Auto-detecting Cursor MDC files

- **WHEN** a `.cursor/rules/*.mdc` file is passed as input without `--from`
- **THEN** Rulette SHALL detect the format as Cursor MDC
- **AND** parse frontmatter metadata and body rules accordingly.

### Requirement: Lossy conversion analysis via inspect verb

The `inspect` command SHALL print human-readable IR details and analyze field loss when targeting specific output formats.
When running `inspect --to <format>`, Rulette SHALL report any fields or entity kinds that cannot be represented in the destination format.

#### Scenario: Inspecting capability loss

- **WHEN** `rulette inspect <input> --to <target>` is executed
- **THEN** Rulette SHALL list surviving fields and surface warnings for unmapped or dropped metadata.
