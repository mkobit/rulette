## 1. Shared capability infrastructure

- [x] 1.1 Define `CoverageStatus` enum (`Supported`, `Lossy`, `Dropped`) and a `CapabilityEntry { entity_kind: String, status: CoverageStatus, reason: Option<String> }` type in `src/emitters/mod.rs`, with kebab-case entity-kind values matching the IR's `#[serde(tag = "kind")]` values.
- [x] 1.2 Add `fn capabilities(&self, doc: &RuletteDocument) -> Vec<CapabilityEntry>` to the `Emitter` trait.
- [x] 1.3 Define the worst-status aggregation helper (`Dropped` > `Lossy` > `Supported`) for rolling up multiple entities of the same kind into one `CapabilityEntry` per `(target, entity_kind)`.

## 2. Extract shared loss-detection logic per emitter

- [x] 2.1 For each of the 8 emitters (claude, cursor, cursor_mcp, codex, copilot, gemini, windsurf, agent_skills), extract the per-entity-kind loss-detection logic currently inlined before each `eprintln!` call into a helper function returning `(CoverageStatus, Option<reason>)`, without changing the existing `eprintln!` wording or `strict` behavior in `emit()`.
- [x] 2.2 Implement `capabilities()` for each of the 8 emitters using the same extracted helpers from 2.1, applying the aggregation helper from 1.3 across all entities of each kind present in the input document.
- [x] 2.3 Run the full existing emitter test suite after each emitter's extraction to confirm `emit()`'s behavior and output (including `eprintln!` warning text and `--strict` error behavior) is unchanged.

## 3. Parity test (required, not deferred)

- [x] 3.1 Add a test that, for every emitter, builds a `RuletteDocument` containing one entity of every kind the IR defines and asserts `capabilities()`'s per-kind status agrees with `emit(doc, strict=true)`'s actual success/error outcome for that same document (Dropped/Lossy in `capabilities()` implies `emit(strict=true)` errors or warns; Supported implies it doesn't).

## 4. `inspect --coverage` CLI surface

- [x] 4.1 Add `--coverage` flag to `InspectArgs`, with `conflicts_with("to")` so `--coverage` and `--to <target>` are mutually exclusive (clap-enforced usage error, not a runtime check).
- [x] 4.2 Add `--json` flag to `InspectArgs` (modifier, only meaningful with `--coverage`).
- [x] 4.3 Implement coverage-mode execution: parse input to IR once, call `.capabilities(&doc)` on every registered emitter (the same list `--to` already switches over), aggregate into the matrix.
- [x] 4.4 Implement human-readable table rendering: entity kinds (present in input only, per spec) as rows, targets as columns, `Supported`/`Lossy`/`Dropped` cells.
- [x] 4.5 Implement `--json` rendering: flat array of `{target, entity_kind, status, reason}` records, kebab-case `target`/`entity_kind` values.
- [x] 4.6 Wire `--strict` to coverage mode: exit non-zero if any matrix cell is `Lossy` or `Dropped`; exit 0 otherwise (with or without `--strict`, absent a failing cell).

## 5. Tests

- [x] 5.1 Unit test: multiple entities of the same kind with mixed statuses roll up to the worst status (spec scenario "Multiple entities of the same kind roll up to the worst status").
- [x] 5.2 Unit test: coverage matrix only includes entity kinds present in input (spec scenario "Coverage matrix reflects actual input").
- [x] 5.3 Unit test: `--coverage --to <target>` is rejected as a usage error.
- [x] 5.4 Unit test: `--coverage --strict` exits non-zero when a Dropped or Lossy cell exists; exits 0 when the matrix is all-Supported.
- [x] 5.5 Unit test: `--coverage --json` output shape matches spec (kebab-case fields, non-null `reason` on lossy/dropped entries, null on supported).
- [x] 5.6 Regression test: existing `inspect --to <format> --strict` output (both text and warnings) is byte-identical to pre-change behavior for a representative fixture document.

## 6. Docs

- [x] 6.1 Update `docs/2026-04-11-prd.md`'s v0.3 milestone entry to reflect the shipped `--coverage` flag shape.
- [x] 6.2 Regenerate CLI reference docs (`src/bin/gen_docs.rs` output) to include the new `--coverage`/`--json` flags.
