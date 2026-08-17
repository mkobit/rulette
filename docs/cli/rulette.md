# Command-Line Help for `rulette`

This document contains the help content for the `rulette` command-line program.

**Command Overview:**

* [`rulette`↴](#rulette)
* [`rulette inspect`↴](#rulette-inspect)
* [`rulette schema`↴](#rulette-schema)
* [`rulette transform`↴](#rulette-transform)

## `rulette`

Stateless CLI tool for transforming AI rules and skills across systems

**Usage:** `rulette [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `inspect` — Pretty-print the IR for debugging
* `schema` — Output JSON Schema for the IR or a specific target format
* `transform` — Parse, transform, and emit rules across formats

###### **Options:**

* `-q`, `--quiet` — Suppress non-error output
* `--strict` — Fail on warnings (including lossy conversion warnings)
* `--no-color` — Disable colored output
* `--log-level <LOG_LEVEL>` — Log verbosity (error, warn, info, debug, trace)



## `rulette inspect`

Pretty-print the IR for debugging

**Usage:** `rulette inspect [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `-t`, `--to <TO>` — Target format to dry-run emission and show lossy conversion warnings

  Possible values: `claude`, `cursor-mdc`, `cursor-mcp`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`, `json-schema`




## `rulette schema`

Output JSON Schema for the IR or a specific target format

**Usage:** `rulette schema [OPTIONS]`

###### **Options:**

* `-t`, `--to <TO>` — Format to output schema for (ir, claude, cursor-mdc, etc.)

  Default value: `ir`
* `--extension <EXTENSION>` — Extension key to output schema for (e.g., rulette:activation)



## `rulette transform`

Parse, transform, and emit rules across formats

**Usage:** `rulette transform [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `--from <FROM>` — Source format (auto-detected if omitted)

  Default value: `auto`

  Possible values: `auto`, `skill-md`, `agent-skills`, `claude`, `claude-settings`, `cursor-mdc`, `cursor-legacy`, `cursor-mcp`, `codex`, `windsurf`, `copilot`, `gemini`, `ir-json`, `ir-toml`

* `--to <TO>` — Target output format

  Possible values: `claude`, `cursor-mdc`, `cursor-mcp`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`, `json-schema`

* `-o`, `--out <OUT>` — Output path (file or directory) or multiple targets via format:path
* `--name <NAME>` — Override name metadata for parsed entities
* `--description <DESCRIPTION>` — Override description metadata for parsed entities
* `--filter <FILTER>` — Keep only rules matching expression (e.g., 'license == "MIT"')
* `--exclude <EXCLUDE>` — Remove rules matching expression
* `--rename <RENAME>` — Rename a metadata field value (from=to)
* `--set <SET>` — Set a metadata field (field=value)
* `--config <CONFIG>` — Load transform pipeline from TOML file



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

