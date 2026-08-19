## ADDED Requirements

### Requirement: Multi-syntax transform-config parsing

The `--config <path>` flag SHALL accept a file with extension `.toml`, `.json`, `.jsonc`, or `.json5`, parsed by its extension against one internal schema.
`.jsonc` SHALL be treated as the canonical documented syntax; `.toml`, `.json`, and `.json5` SHALL remain fully accepted with identical schema semantics.
If the file extension does not match any of the four known extensions, Rulette SHALL attempt to parse it as `.json5` and report that parser's error on failure.

#### Scenario: Loading a canonical .jsonc config

- **WHEN** `--config rulette.transform.jsonc` is given, and the file contains `//` comments and a trailing comma
- **THEN** Rulette SHALL parse it successfully using the same internal schema as a `.toml` config.

#### Scenario: Loading a .toml config

- **WHEN** `--config rulette.transform.toml` is given
- **THEN** Rulette SHALL parse it as TOML against the same internal schema as the `.jsonc` form.

#### Scenario: Loading a strict .json config

- **WHEN** `--config rulette.transform.json` is given, and the file contains a comment or a trailing comma
- **THEN** Rulette SHALL fail with a parse error, since `.json` is parsed strictly.

#### Scenario: Unrecognized extension falls back to the canonical parser

- **WHEN** `--config rulette.transform.cfg` is given (an extension that is not `.toml`, `.json`, `.jsonc`, or `.json5`)
- **THEN** Rulette SHALL attempt to parse the file's contents as `.json5`
- **AND** if that parse fails, SHALL report that parser's error, not a generic "unrecognized extension" error.

### Requirement: Transform-config schema

A transform-config file SHALL declare three top-level keys: `inputs` (a list of input paths), `pipeline` (an ordered list of transformation steps), and `outputs` (a list of output destinations). All three keys SHALL be optional and SHALL default to an empty list when omitted.
Each `pipeline` entry SHALL be a single-key object naming one of `filter`, `exclude`, `rename`, or `set`, with the same expression syntax as the corresponding CLI flag.
Each `outputs` entry SHALL be an object with a required `target` (an output format token) and `path`, and optional `scope`, `entities`, `drop`, and `strict` fields.
An `outputs` entry's `path` value of `"-"` SHALL mean standard output, identical to the CLI's `-o <format>:-` convention, rather than a literal path.
Rulette SHALL reject a transform-config file containing any top-level key other than `inputs`, `pipeline`, or `outputs` with a parse error naming the unrecognized key.

#### Scenario: Ordered pipeline steps apply in file order

- **WHEN** a config's `pipeline` is `[{"filter": "status == \"stable\""}, {"rename": "org:name=internal:name"}]`
- **THEN** Rulette SHALL apply the filter step before the rename step, in that order.

#### Scenario: Unrecognized top-level key is rejected

- **WHEN** a config file contains a top-level key that is not `inputs`, `pipeline`, or `outputs` (for example, a leftover `filter` key from the pre-existing flat config shape)
- **THEN** Rulette SHALL exit with a non-zero exit code and a parse error naming that key.

#### Scenario: A dash path means standard output, not a literal path

- **WHEN** an output entry has `path: "-"`
- **THEN** Rulette SHALL write that output to standard output
- **AND** SHALL NOT create a file or directory literally named `-`.

### Requirement: Per-output entity and strictness scoping

Each `outputs` entry's `entities` field, if present, SHALL act as an allow-list of IR entity kinds (using the same kebab-case kind tokens as `coverage-reporting`'s capability matrix): only entities of a listed kind SHALL be considered for that output.
Each `outputs` entry's `drop` field, if present, SHALL act as a deny-list applied after `entities`: entities of a listed kind SHALL be excluded from that output even if `entities` would otherwise include them.
If both `entities` and `drop` are omitted for an output entry, that output SHALL receive every entity kind its target emitter can represent.
Each `outputs` entry's `strict` field, if present, SHALL override the invocation's global `--strict` flag for that output only; if absent, that output SHALL use the invocation's global `--strict` value.
These fields SHALL be evaluated independently per output entry: two output entries in the same invocation with different `entities`/`drop`/`strict` values SHALL each receive their own filtered view of the input entities.

#### Scenario: Coarse allow-list restricts an output to specific entity kinds

- **WHEN** an output entry has `entities: ["rule", "skill"]` and the parsed input also contains an `mcp-server` entity
- **THEN** that output SHALL NOT receive the `mcp-server` entity
- **AND** other output entries in the same invocation without an `entities` restriction SHALL still receive it.

#### Scenario: Deny-list excludes specific entity kinds

- **WHEN** an output entry has `drop: ["hook"]` and no `entities` restriction
- **THEN** that output SHALL receive every entity kind present in the input except `hook`.

#### Scenario: Per-output strict overrides the global flag

- **WHEN** the invocation is run without a global `--strict` flag
- **AND** one output entry has `strict: true`
- **THEN** that output's emission SHALL fail on a lossy conversion that would otherwise only warn
- **AND** other outputs in the same invocation without `strict: true` SHALL still only warn on the same kind of lossy conversion.

#### Scenario: Per-output strict can relax below the global flag

- **WHEN** the invocation is run with a global `--strict` flag
- **AND** one output entry has `strict: false`
- **THEN** that output's emission SHALL only warn (not fail) on a lossy conversion
- **AND** other outputs in the same invocation without `strict: false` SHALL still fail on the same kind of lossy conversion.

### Requirement: Entity-kind token validation

Each token listed in an `outputs` entry's `entities` or `drop` field SHALL be validated at parse time against the closed set of IR entity-kind kebab-case tokens (`rule`, `skill`, `mcp-server`, `hook`, `agent`, `permissions`); an unrecognized token SHALL be rejected as a parse error naming the invalid token, not silently accepted as an allow-list or deny-list entry that matches nothing.

#### Scenario: Unknown entity-kind token in entities is rejected

- **WHEN** an output entry has `entities: ["rules"]` (misspelled, not a valid entity-kind token)
- **THEN** Rulette SHALL exit with a non-zero exit code and a parse error naming the invalid token, rather than silently producing an output with no matching entities.

#### Scenario: Unknown entity-kind token in drop is rejected

- **WHEN** an output entry has `drop: ["hooks"]` (misspelled, not a valid entity-kind token)
- **THEN** Rulette SHALL exit with a non-zero exit code and a parse error naming the invalid token.

### Requirement: Output scope token validation

Each `outputs` entry's `scope` field SHALL default to `"project"` when omitted, and SHALL be validated against the closed set `{project, user, enterprise, local}` at parse time; an unrecognized token SHALL be rejected as a parse error.
Behavioral differences between scope tiers other than `project` are not defined by this requirement.

#### Scenario: Unknown scope token is rejected

- **WHEN** an output entry has `scope: "team"`
- **THEN** Rulette SHALL exit with a non-zero exit code and a parse error naming the invalid scope token.

#### Scenario: Omitted scope defaults to project

- **WHEN** an output entry omits `scope`
- **THEN** Rulette SHALL treat that entry as `scope: "project"`.

### Requirement: Scaffold a transform-config from on-disk inputs

The `transform` command SHALL support `--to transform-config` as an output format, which, combined with `--out <path>`, SHALL write a transform-config file (in the syntax implied by `<path>`'s extension, defaulting to plain JSON content when writing to standard output or to an unrecognized extension; scaffold output SHALL NOT contain generated comments regardless of extension) whose `inputs` reproduce the invocation's resolved input paths exactly, unmodified.
For each of the invocation's resolved input paths — the literal paths given as positional arguments or via a config's `inputs`, not a recursive expansion of directory contents — Rulette SHALL infer a target output format from the path's own naming convention, checked against an ordered, most-specific-first table of known conventions for the targets this capability currently recognizes (`codex`, `claude`, `cursor-mdc`, `cursor-mcp`); a path matching no recognized target is out of scope for inference (see the corresponding scenario below).
A matched input SHALL contribute one `outputs` entry using that target's default path, deduplicated so that multiple inputs matching the same target contribute only one `outputs` entry, regardless of how many distinct input paths matched it.
An input path matching no known convention SHALL still appear in the generated `inputs` list but SHALL NOT contribute an `outputs` entry, and Rulette SHALL print a warning naming the unmatched path.
This capability's target recognition is a strict subset of `OutputFormat`: it does not attempt to infer `windsurf`, `copilot`, `gemini`, or `agent-skills` targets from path conventions, consistent with those targets being deferred by the source design (`docs/2026-08-18-cli-ux-design.md` "Scope of tools"); a path belonging to one of them is treated the same as any other unmatched path.

#### Scenario: Scaffolding from a known multi-tool repo layout

- **WHEN** `rulette transform .claude/ .cursor/ AGENTS.md --to transform-config --out rulette.transform.jsonc` is run
- **THEN** Rulette SHALL write `rulette.transform.jsonc` with `inputs: [".claude/", ".cursor/", "AGENTS.md"]`
- **AND** `outputs` SHALL contain one entry each for `claude`, `cursor-mdc`, and `codex`, at their respective default paths.

#### Scenario: Cursor rules and Cursor MCP config scaffold to distinct targets

- **WHEN** the invocation's inputs include both `.cursor/rules/typescript.mdc` and `.cursor/mcp.json`
- **THEN** the generated `outputs` SHALL contain a `cursor-mdc` entry and a separate `cursor-mcp` entry
- **AND** neither input SHALL be misclassified as the other's target.

#### Scenario: A bare CLAUDE.md file scaffolds to the claude target

- **WHEN** the invocation's inputs include a top-level `CLAUDE.md` file with no accompanying `.claude/` directory among the inputs
- **THEN** the generated `outputs` SHALL contain a `claude` entry.

#### Scenario: Re-running a freshly scaffolded config is an identity round-trip

- **WHEN** `rulette transform --config <path>` is run using a config generated by `--to transform-config` from an unmodified repo
- **AND** `--check` is passed
- **THEN** Rulette SHALL exit with a zero exit code, reporting every target unchanged.

#### Scenario: Deduplicated Codex output preserves nested AGENTS.md fidelity on round-trip

- **WHEN** the invocation's inputs include both a top-level `AGENTS.md` and a nested `src/backend/AGENTS.md`
- **THEN** the generated `outputs` SHALL contain exactly one `codex` entry (not two)
- **AND** re-running `rulette transform --config <generated-path> --check` SHALL still report every target unchanged, because the nested file's placement is carried as IR entity metadata (`rulette:directory-scope`) reproduced from `inputs`, not from a separate `outputs` entry per file.

#### Scenario: Unrecognized input path is preserved but not scaffolded as an output

- **WHEN** scaffolding includes an input path that matches no known tool path convention
- **THEN** the generated config's `inputs` SHALL still include that path
- **AND** no `outputs` entry SHALL be generated for it
- **AND** Rulette SHALL print a warning naming the unmatched path.

#### Scenario: A single repo-root input is not recursively expanded

- **WHEN** `rulette transform . --to transform-config --out rulette.transform.jsonc` is run against a directory that itself contains `.claude/`, `.cursor/`, and `AGENTS.md`
- **THEN** Rulette SHALL treat `.` itself as the one resolved input path, matching no known convention
- **AND** SHALL generate `inputs: ["."]` with no `outputs` entries and one unmatched-path warning, rather than discovering the nested tool directories.
