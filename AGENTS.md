# Rulette context

## Core identity
Rulette is a stateless Rust CLI compiler for AI skills.
It transforms Markdown and TOML input into structured JSON sinks or other derived formats.
The tool operates without local state or an initialization phase.

## Mental model
The architecture follows a standard compiler pattern consisting of a frontend, IR, and backend.
The frontend handles parsing of Markdown and TOML content.
The IR represents the intermediate data structure of the skill.
The backend emits the IR into specific platform formats.

## Tech stack
Rust stable is the primary language.
Clap provides the CLI framework.
Serde handles data serialization and deserialization.
Pulldown-cmark parses Markdown content.

## Hard constraints
Input is restricted to stdin or explicit file paths.
Output is directed to stdout or specified file destinations.
Centralized configuration files are not supported.

## Architectural style
The source code uses a modular directory structure.
Core logic remains decoupled from CLI boilerplate and I/O.
The project mirrors patterns found in the Mise and Jules repositories.
