## ADDED Requirements

### Requirement: Declarative transform configuration for rule bundling and marketplace mappings

`TransformConfigFile` in `src/cli/commands/transform.rs` SHALL accept optional declarative `rule_bundling` options (`header_style`, `delimiter`, and `include_metadata_comments`) and local `marketplace_plugins` path mappings when the caller passes `rulette transform --config <PATH>`.
`rulette` SHALL load `.toml`, `.json`, `.jsonc`, and `.json5` manifests only through explicit `--config <PATH>` and SHALL NOT auto-discover configuration files or grant live filesystem write authority.

#### Scenario: Applying declarative rule bundling options from `--config <PATH>`

- **WHEN** a caller executes `rulette transform --config <PATH> --stage <DIRECTORY> --project-root <DIRECTORY>` with a manifest that specifies `rule_bundling` options for `codex@project` or `claude@project`
- **THEN** `rulette` SHALL render the bundled `AGENTS.md` or `CLAUDE.md` artifact using the configured header style and delimiter format
- **AND** SHALL stage the bundled artifact and `rulette.plan.json` deterministically.

#### Scenario: Mapping vendored plugins to canonical names in `--config <PATH>`

- **WHEN** a `--config <PATH>` manifest declares `marketplace_plugins` entries mapping canonical `<plugin>` names to relative local paths
- **THEN** `rulette` SHALL validate that each path is a safe relative `ResourcePath` without `..` or symbolic links and bind each vendored plugin to its canonical `<plugin>` namespace
- **AND** SHALL preserve `/<plugin>:<skill>` invocation names in the emitted target artifacts.

#### Scenario: Rejecting authority or unknown fields in `--config <PATH>`

- **WHEN** a `--config <PATH>` manifest contains unknown fields, `..` path traversal in `marketplace_plugins`, or live root fields such as `project_root`, `user_root`, or `replace`
- **THEN** `rulette` SHALL exit with a non-zero validation error before compiling sources or creating a stage directory.
