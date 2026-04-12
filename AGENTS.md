# Rulette context

## Hard constraints

The binary must be fully static with no runtime dependencies.
The CLI is a thin wrapper; all logic lives in the library.
No initialization phase, no local state, no configuration files.

## Documentation Reference

For product requirements, goals, and milestones, see:

- `docs/2026-04-11-prd.md`

For CLI command documentation and options, see:

- `docs/2026-04-11-man-page.md`

For context on why Rulette exists and how it compares to alternatives, see:

- `docs/2026-04-11-announcement.md`
- `docs/2026-04-11-landscape.md`

## Inputs

Single files (path or stdin) and tar archives containing multiple files are both valid inputs.

## Pipeline

Between parsing and emission, the IR passes through an ordered transformation pipeline (filter, map, rename, merge) before reaching any backend.
New backends are additive and do not affect existing ones.
