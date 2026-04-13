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
* [`rulette validate`↴](#rulette-validate)
* [`rulette fetch`↴](#rulette-fetch)
* [`rulette lock`↴](#rulette-lock)
* [`rulette verify`↴](#rulette-verify)
* [`rulette archive`↴](#rulette-archive)
* [`rulette unarchive`↴](#rulette-unarchive)

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
* `validate` — Validate rules against the IR schema and optional policy constraints (v0.1.1)
* `fetch` — Fetch rules from a remote source (v0.2)
* `lock` — Generate or update a lockfile from a manifest (v0.2)
* `verify` — Verify that fetched content matches the lockfile (v0.2)
* `archive` — Bundle rules into a content-addressed tar archive (v0.2)
* `unarchive` — Extract and verify a content-addressed archive (v0.2)

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

  Possible values: `auto`, `skill-md`, `agent-skills`, `claude`, `cursor-mdc`, `cursor-legacy`, `codex`, `windsurf`, `copilot`, `gemini`, `ir-json`, `ir-toml`

* `-o`, `--out <OUT>` — Write output to file instead of stdout
* `--strict` — Fail on parse warnings
* `--name <NAME>` — Override name metadata for parsed entities
* `--description <DESCRIPTION>` — Override description metadata for parsed entities



## `rulette emit`

Emit IR (from stdin or files) to a target format

**Usage:** `rulette emit [OPTIONS] --to <TO> [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `-t`, `--to <TO>` — Target output format

  Possible values: `claude`, `cursor-mdc`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`

* `-o`, `--out <OUT>` — Output path (file or directory)
* `--scope <SCOPE>` — Output scope: project (default) or user

  Default value: `project`
* `--merge` — Merge multiple rules into a single output file
* `--split` — Split into one file per rule (default for directory output)



## `rulette convert`

Parse input and emit to a target format in one step

**Usage:** `rulette convert [OPTIONS] --to <TO> [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `--from <FROM>` — Source format (auto-detected if omitted)

  Default value: `auto`

  Possible values: `auto`, `skill-md`, `agent-skills`, `claude`, `cursor-mdc`, `cursor-legacy`, `codex`, `windsurf`, `copilot`, `gemini`, `ir-json`, `ir-toml`

* `--to <TO>` — Target output format

  Possible values: `claude`, `cursor-mdc`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`

* `-o`, `--out <OUT>` — Output path (file or directory)
* `--scope <SCOPE>` — Output scope: project (default) or user

  Default value: `project`
* `--merge` — Merge multiple rules into a single output file
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

  Possible values: `claude`, `cursor-mdc`, `codex`, `windsurf`, `copilot`, `gemini`, `agent-skills`, `ir-json`, `ir-toml`




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

* `--filter <FILTER>` — Keep only rules matching expression
* `--exclude <EXCLUDE>` — Remove rules matching expression
* `--rename <RENAME>` — Rename a metadata field value (from=to)
* `--set <SET>` — Set a metadata field (field=value)
* `--config <CONFIG>` — Load transform pipeline from TOML file
* `--shell <SHELL>` — Pipe each rule body through a shell command



## `rulette validate`

Validate rules against the IR schema and optional policy constraints (v0.1.1)

**Usage:** `rulette validate [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories (or "-" for stdin)

  Default value: `-`

###### **Options:**

* `--policy <POLICY>` — Policy file (TOML) defining additional constraints
* `--strict` — Treat warnings as errors



## `rulette fetch`

Fetch rules from a remote source (v0.2)

**Usage:** `rulette fetch [OPTIONS] <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — Source to fetch rules from

###### **Options:**

* `--lockfile <LOCKFILE>` — Lockfile to verify against (default: rules.lock)

  Default value: `rules.lock`
* `--allow-mutable` — Allow fetching without pinned version
* `--no-verify` — Skip integrity verification (requires --allow-mutable)
* `-o`, `--out <OUT>` — Output path



## `rulette lock`

Generate or update a lockfile from a manifest (v0.2)

**Usage:** `rulette lock [OPTIONS] [MANIFEST]`

###### **Arguments:**

* `<MANIFEST>` — Manifest file (rulette.toml)

###### **Options:**

* `-o`, `--out <OUT>` — Lockfile output path (default: rules.lock)

  Default value: `rules.lock`
* `--update <UPDATE>` — Update only the named package



## `rulette verify`

Verify that fetched content matches the lockfile (v0.2)

**Usage:** `rulette verify [OPTIONS] [LOCKFILE]`

###### **Arguments:**

* `<LOCKFILE>` — Lockfile to verify

  Default value: `rules.lock`

###### **Options:**

* `--vendor <VENDOR>` — Vendor directory to verify

  Default value: `vendor/rules/`



## `rulette archive`

Bundle rules into a content-addressed tar archive (v0.2)

**Usage:** `rulette archive [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Input files or directories to archive

###### **Options:**

* `-o`, `--out <OUT>` — Output archive path
* `--compress <COMPRESS>` — Compression (none, gzip, zstd; default: gzip)

  Default value: `gzip`



## `rulette unarchive`

Extract and verify a content-addressed archive (v0.2)

**Usage:** `rulette unarchive [OPTIONS] <ARCHIVE>`

###### **Arguments:**

* `<ARCHIVE>` — Archive file to extract

###### **Options:**

* `-o`, `--out <OUT>` — Extraction directory



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
