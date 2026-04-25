# Rulette context

## Hard constraints

The binary must be fully static with no runtime dependencies.
The CLI is a thin wrapper; all logic lives in the library.
No initialization phase, no local state, no configuration files.

## Documentation Reference

For product requirements, goals, and milestones, see:

- [docs/2026-04-11-prd.md](docs/2026-04-11-prd.md)

For CLI command documentation and options, see:

- [docs/2026-04-11-man-page.md](docs/2026-04-11-man-page.md)

For context on why Rulette exists and how it compares to alternatives, see:

- [docs/2026-04-11-announcement.md](docs/2026-04-11-announcement.md)
- [docs/2026-04-11-landscape.md](docs/2026-04-11-landscape.md)

## Inputs

Single files (path or stdin) and tar archives containing multiple files are both valid inputs.

## Pipeline

Rulette uses a single `transform` command to handle the full lifecycle of AI rules.
It reads from any source, applies a transformation pipeline (filter, rename, set, dedup), and emits to any target.
This command replaces the separate parse, convert, and emit verbs with a unified, source-to-sink engine.
