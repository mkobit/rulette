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
* `schema` — Output the JSON Schema for the compilation graph
* `transform` — Compile graphs and stage or apply native publication plans

###### **Options:**

* `-q`, `--quiet` — Suppress non-error output
* `--no-color` — Disable colored output
* `--log-level <LOG_LEVEL>` — Log verbosity (error, warn, info, debug, trace)



## `rulette inspect`

Pretty-print the IR for debugging

**Usage:** `rulette inspect [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Native input files or directories, or `-` for standard input

  Default value: `-`

###### **Options:**

* `--from <FROM>` — Source frontend, auto-detected when omitted

  Default value: `auto`

  Possible values: `auto`, `claude`, `cursor-mdc`, `codex`, `antigravity`, `opencode`, `graph-json`, `graph-toml`

* `-t`, `--to <TO>` — Analyze one core target without publishing native artifacts
* `--coverage` — Compute the core-target capability matrix for observed package kinds
* `--json` — Render coverage as JSON
* `--strict` — Fail coverage when any observed package kind is lossy or dropped



## `rulette schema`

Output the JSON Schema for the compilation graph

**Usage:** `rulette schema [OPTIONS]`

###### **Options:**

* `-t`, `--to <TO>` — Schema contract to output (only `graph` is supported)

  Default value: `graph`



## `rulette transform`

Compile graphs and stage or apply native publication plans

**Usage:** `rulette transform [OPTIONS] [INPUT]...`

###### **Arguments:**

* `<INPUT>` — Native input files or directories, or `-` for standard input.

   Stdin is used when neither these inputs nor config inputs are supplied.

###### **Options:**

* `--from <FROM>` — Source frontend, auto-detected when omitted

  Default value: `auto`

  Possible values: `auto`, `claude`, `cursor-mdc`, `codex`, `antigravity`, `opencode`, `graph-json`, `graph-toml`

* `--select <SELECT>` — Select one package by its exact graph package ID
* `--target <TARGET>` — Stage a native target as `format@scope`
* `--allow-lossy` — Accept reported representational loss for requested native targets
* `--stage <STAGE>` — Write a self-contained publication plan to this new directory
* `--project-root <PROJECT_ROOT>` — Explicitly authorize the live project root for all project targets
* `--user-root <USER_ROOT>` — Explicitly authorize one user target root as `target=path`
* `--check` — Check destinations without creating a stage or applying a plan
* `--apply <STAGE_DIR/rulette.plan.json>` — Apply the plan at `stage-dir/rulette.plan.json`
* `--expect-plan-sha256 <EXPECT_PLAN_SHA256>` — Require this SHA-256 digest before checking or applying a staged plan
* `--allow-project-root <ALLOW_PROJECT_ROOT>` — Explicitly authorize the live project root for plan operations
* `--allow-user-root <ALLOW_USER_ROOT>` — Explicitly authorize one plan user target root as `target=path`
* `--replace` — Allow an apply operation to replace conflicting destinations
* `--config <CONFIG>` — Load one explicit selection-and-target-only transform configuration file



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

