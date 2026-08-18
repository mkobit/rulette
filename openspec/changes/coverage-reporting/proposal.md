## Why

Rulette already computes per-entity lossy-conversion warnings inside every emitter (`src/emitters/*.rs`), but each one hand-writes an `eprintln!` string, gated on `strict`, visible only for the single target passed to `rulette inspect --to <format>` in one invocation. There is no structured, machine-readable, or standing view of which entity kinds and fields survive across all targets at once. The PRD's v0.3 stretch goal "Coverage reporting (which entity kinds survive per target)" (`docs/2026-04-11-prd.md:336`) requires exactly this: a matrix view generalizing the existing per-invocation warnings into a queryable report. Tracked as `rulette-ldv`.

## What Changes

- Emitters report capability/loss information as structured data instead of only printing to stderr. The `Emitter` trait gains a way to describe, per entity kind (and where practical, per field), whether it is fully supported, partially supported (lossy), or unsupported (dropped) for that target — computed from the same logic that currently drives the `eprintln!` warnings, not duplicated.
- `inspect` gains a coverage mode that runs the capability check against **all** registered targets in one invocation and renders a matrix (entity kind × target → supported/lossy/dropped), instead of requiring one `--to <format>` run per target.
- The existing per-target `inspect --to <format> --strict` warning output continues to work, now sourced from the same structured data (no duplicate logic, no behavior change to today's single-target output).
- **BREAKING**: the `Emitter` trait gains a new required method (`capabilities()`); `emit()`'s own signature is unchanged (see design.md Decision 1). `Emitter` is `pub use`'d from the crate root (`src/lib.rs`), so this is a genuine public API break for anyone implementing `Emitter` against the `rulette` library crate, not merely an internal detail — acceptable pre-1.0 (crate is `0.1.0`) but should be called out in the release notes for whatever version ships this.

## Capabilities

### New Capabilities

- `coverage-reporting`: All-targets capability matrix — computing and rendering which entity kinds/fields survive, are lossy, or are dropped across every registered output target in a single report.

### Modified Capabilities

- `frontends-and-backends`: The "Lossy conversion analysis via inspect verb" requirement changes from "warnings printed to stderr per single-target run" to "structured capability data available per emitter, surfaced as warnings for single-target `inspect --to` and as a matrix for `coverage-reporting`."

## Impact

- `src/emitters/mod.rs`: `Emitter` trait signature.
- `src/emitters/*.rs` (8 emitters: claude, cursor, cursor_mcp, codex, copilot, gemini, windsurf, agent_skills): replace `eprintln!` lossy-warning calls with structured capability reporting.
- `src/cli/commands/inspect.rs`: new coverage mode/flag, all-targets iteration.
- No change to on-disk output formats, transform behavior, or the IR.
- Tracking: beads `rulette-ldv`.
