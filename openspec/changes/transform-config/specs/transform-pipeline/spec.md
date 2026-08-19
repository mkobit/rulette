## ADDED Requirements

### Requirement: Config-file input composition with CLI positional inputs

The `transform` command SHALL treat a transform-config file's `inputs` and the CLI's positional input arguments as mutually exclusive when both are non-empty: if `--config <path>` is given, the loaded config declares a non-empty `inputs`, and the CLI invocation also passes at least one positional input, Rulette SHALL exit with a non-zero exit code and a usage error rather than silently preferring one source.
If the loaded config's `inputs` is empty, the CLI's positional inputs (if any) SHALL be used; if both are empty, Rulette SHALL default to reading standard input, matching the command's behavior with no `--config` at all.

#### Scenario: Config inputs and CLI positional inputs both set is a usage error

- **WHEN** `--config rulette.transform.jsonc` is given, that file declares `inputs: ["./rules/"]`
- **AND** the CLI invocation also passes a positional input path
- **THEN** Rulette SHALL exit with a non-zero exit code and a usage error, without reading any input.

#### Scenario: Config without inputs lets CLI positionals fill the gap

- **WHEN** `--config rulette.transform.jsonc` is given, that file omits `inputs` (or declares it as an empty list)
- **AND** the CLI invocation passes one or more positional input paths
- **THEN** Rulette SHALL read from the CLI-provided positional inputs.

#### Scenario: Neither source sets inputs defaults to stdin

- **WHEN** `--config rulette.transform.jsonc` is given, that file omits `inputs`
- **AND** the CLI invocation passes no positional input paths
- **THEN** Rulette SHALL read from standard input, matching the no-`--config` default.

### Requirement: Config-file pipeline and output composition with CLI flags

When `--config <path>` is given alongside CLI pipeline flags (`--filter`, `--exclude`, `--rename`, `--set`), Rulette SHALL run the config's `pipeline` steps first, in the file's order, followed by any CLI pipeline flags, in the fixed order filter, exclude, rename, set. CLI pipeline flags SHALL compose with (append to), not replace, the config's `pipeline`.
When `--config <path>` is given alongside CLI output flags (`-o`/`--out`, `--to`), and at least one CLI output flag is present, the CLI-specified outputs SHALL entirely replace the config's `outputs`; the config's `outputs` SHALL only be used when no CLI output flag is given.

#### Scenario: CLI filter flag appends to a config's pipeline

- **WHEN** a config's `pipeline` is `[{"rename": "org:name=internal:name"}]`
- **AND** the invocation also passes `--filter 'status == "stable"'`
- **THEN** Rulette SHALL apply the config's rename step first, then the CLI's filter step.

#### Scenario: CLI output flag replaces a config's outputs entirely

- **WHEN** a config's `outputs` declares five targets
- **AND** the invocation also passes `-o ir-json:-`
- **THEN** Rulette SHALL emit only the CLI-specified `ir-json` target, not any of the config's five.
