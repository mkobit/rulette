# Command-Line Help for `rulette`

This document contains the help content for the `rulette` command-line program.

**Command Overview:**

* [`rulette`↴](#rulette)
* [`rulette parse`↴](#rulette-parse)
* [`rulette emit`↴](#rulette-emit)
* [`rulette convert`↴](#rulette-convert)
* [`rulette inspect`↴](#rulette-inspect)
* [`rulette schema`↴](#rulette-schema)
* [`rulette transform`↴](#rulette-transform)

## `rulette`

Stateless CLI tool for transforming AI rules and skills across systems

**Usage:** `rulette [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `parse` — Parse one or more input files (or stdin) into the Rulette IR
* `emit` — Emit IR (from stdin or files) to a target format
* `convert` — Parse input and emit to a target format in one step
* `inspect` — Pretty-print the IR for debugging
* `schema` — Output JSON Schema for the IR or a specific target format
* `transform` — Apply transformations to IR (v0.1.1)

###### **Options:**

* `-q`, `--quiet` — Suppress non-error output
* `--strict` — Fail on warnings (including lossy conversion warnings)
* `--no-color` — Disable colored output
* `--log-level <LOG_LEVEL>` — Log verbosity (error, warn, info, debug, trace)



## `rulette parse`

Parse one or more input files (or stdin) into the Rulette IR

**Usage:** `rulette parse [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `--from <FROM>` — Force input format detection

  Default value: `auto`

  Possible values: `auto`, `skill-md`, `agent-skills`, `claude`, `claude-settings`, `cursor-mdc`, `cursor-legacy`, `cursor-mcp`, `codex`, `windsurf`, `copilot`, `gemini`, `ir-json`, `ir-toml`

* `-o`, `--out <OUT>` — Write output to file instead of stdout
* `--strict` — Fail on parse warnings
* `--name <NAME>` — Override name metadata for parsed entities
* `--description <DESCRIPTION>` — Override description metadata for parsed entities



## `rulette emit`

Emit IR (from stdin or files) to a target format

**Usage:** `rulette emit [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `-t`, `--to <TO>` — Target output format

  Possible values: `claude`, `claude-settings`, `cursor-mdc`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`

* `-o`, `--out <OUT>` — Output path (file or directory) or multiple targets via format:path
* `--scope <SCOPE>` — Output scope: project (default) or user

  Default value: `project`



## `rulette convert`

Parse input and emit to a target format in one step

**Usage:** `rulette convert [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `--from <FROM>` — Source format (auto-detected if omitted)

  Default value: `auto`

  Possible values: `auto`, `skill-md`, `agent-skills`, `claude`, `claude-settings`, `cursor-mdc`, `cursor-legacy`, `cursor-mcp`, `codex`, `windsurf`, `copilot`, `gemini`, `ir-json`, `ir-toml`

* `--to <TO>` — Target output format

  Possible values: `claude`, `claude-settings`, `cursor-mdc`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`

* `-o`, `--out <OUT>` — Output path (file or directory) or multiple targets via format:path
* `--scope <SCOPE>` — Output scope: project (default) or user

  Default value: `project`
* `--name <NAME>` — Override name metadata for parsed entities
* `--description <DESCRIPTION>` — Override description metadata for parsed entities



## `rulette inspect`

Pretty-print the IR for debugging

**Usage:** `rulette inspect [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `-t`, `--target <TARGET>` — Target format to dry-run emission and show lossy conversion warnings

  Possible values: `claude`, `claude-settings`, `cursor-mdc`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`




## `rulette schema`

Output JSON Schema for the IR or a specific target format

**Usage:** `rulette schema [OPTIONS]`

###### **Options:**

* `-f`, `--format <FORMAT>` — Format to output schema for (ir, claude, cursor-mdc, etc.)

  Default value: `ir`



## `rulette transform`

Apply transformations to IR (v0.1.1)

**Usage:** `rulette transform [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `--filter <FILTER>` — Keep only rules matching expression (e.g., 'license == "MIT"')
* `--exclude <EXCLUDE>` — Remove rules matching expression
* `--rename <RENAME>` — Rename a metadata field value (from=to)
* `--set <SET>` — Set a metadata field (field=value)
* `--config <CONFIG>` — Load transform pipeline from TOML file
* `--dedup` — Remove duplicate entities
* `-o`, `--out <OUT>` — Target output format (currently only IrJson is fully supported here)



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
