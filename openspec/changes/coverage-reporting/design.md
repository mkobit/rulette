## Context

Every `Emitter` impl (`src/emitters/{claude,cursor,cursor_mcp,codex,copilot,gemini,windsurf,agent_skills}.rs`) already computes, inline, whether a given entity/field survives conversion to that target. Today that computation immediately collapses into an `eprintln!("Warning: Lossy conversion: ...")` call gated on the `strict: bool` parameter of `Emitter::emit`. The determination and the reporting are fused: there is no intermediate structured value, so nothing else (a coverage matrix, a JSON report, a future `--strict` policy engine) can reuse the same logic without re-deriving it.

`rulette inspect --to <format> --strict` (`src/cli/commands/inspect.rs`) is the only current consumer, and it only sees one target per invocation — checking coverage across all 8 targets today means 8 separate CLI invocations and manual diffing of stderr output.

## Goals / Non-Goals

**Goals:**

- Make lossy-conversion determinations structured data, computed once per emitter, consumed by both the existing single-target warning path and a new all-targets coverage matrix.
- Preserve today's single-target `inspect --to <format> --strict` output and exit-code behavior exactly (no behavior change visible to existing scripts/CI).
- Support both a human-readable table and machine-readable JSON for the new coverage matrix.

**Non-Goals:**

- Per-field granularity beyond what emitters already distinguish today (most emit per-entity-kind warnings, e.g. "Hook to Copilot drops metadata" rather than per-field detail). Extending to field-level detail is a possible future iteration, not required here.
- Changing IR structure, transform behavior, or any on-disk output format.
- A new standalone `coverage` top-level command — this is a mode of `inspect`, consistent with `inspect` already being "the debugger" per the PRD's CLI structure.

## Decisions

**1. Extend the `Emitter` trait with a `capabilities()` method rather than changing `emit()`'s return type.**

```rust
pub enum CoverageStatus { Supported, Lossy, Dropped }

pub trait Emitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>>;
    fn capabilities(&self, doc: &RuletteDocument) -> Vec<(EntityKindLabel, CoverageStatus)>;
}
```

Alternative considered: change `emit()` to return `(HashMap<PathBuf, String>, Vec<Warning>)` and have `inspect --to` print from the `Vec<Warning>` instead of `eprintln!`. Rejected because it conflates two different call patterns — `emit()` is also called from `transform` (the hot path, real writes) where the existing `eprintln!`-on-`strict` behavior is fine and cheap; the coverage matrix needs to call the capability check across all 8 emitters without doing 8 real emissions. A separate `capabilities()` method lets the coverage matrix probe every emitter without invoking `emit()`'s full serialization work, and keeps `emit()`'s signature (and every one of its 8 call sites, including `transform`) untouched.

Each emitter's `capabilities()` implementation and its `emit()` implementation both call into the same shared per-entity-kind loss-detection helper functions (extracted from the current inline logic), so the `eprintln!` strings in `emit()` and the structured `CoverageStatus` values in `capabilities()` are computed by the same code path, not merely similar code. This is enforced by a required parity test (see Risks below and tasks.md), not left as an aspiration.

`Emitter` is `pub use`'d from `src/lib.rs`, so adding `capabilities()` as a required trait method is a real public API break for the `rulette` library crate (not "internal only" as an earlier draft of this proposal claimed) — acceptable at `0.1.0` pre-1.0, but worth flagging in release notes.

**1a. Classification rule: Dropped vs. Lossy vs. Supported.**

To keep this consistent across 8 emitters × up to 6 entity kinds, `capabilities()` classifies each entity instance using one rule, applied by the shared helper functions from Decision 1:

- **Dropped**: the entity contributes zero bytes to any file the target emitter produces (e.g., Gemini's `McpServer` handling today: the entity is silently absent from `output`).
- **Lossy**: the entity contributes output, but the shared helper detects at least one field/metadata item it did not carry over (today's `eprintln!` trigger condition, unchanged).
- **Supported**: the entity contributes output and the shared helper detects no dropped field.

This is exactly today's existing binary warn/don't-warn logic, split into three buckets instead of two — no new detection logic, just finer-grained labeling of what's already computed.

**1b. Aggregation rule when a document has multiple entities of the same kind.**

A document commonly has more than one entity of a given kind (multi-file input, tar archives). `capabilities()` reports one status per `(target, entity_kind)` pair, so multiple instances must roll up to one value. Rule: **worst-case wins** — `Dropped` if any instance is Dropped, else `Lossy` if any instance is Lossy, else `Supported`. Rationale: the coverage matrix's primary consumer is a CI gate (see Decision 5 below); a cell that hides a single Dropped instance behind a Supported summary would produce a false-clean signal, which is worse than an over-cautious one.

**2. `capabilities()` takes the parsed `&RuletteDocument`, not a static per-target declaration.**

Coverage depends on what's actually in the document (e.g., an `mcp-server` entity with only `command`/`args` set is Supported by a target that drops `env`, but the same target is Lossy if `env` is populated). A static "Cursor supports: rule, skill; drops: hook, agent, permissions" table would be simpler but loses this data-dependent nuance that today's inline `eprintln!` logic already captures (see e.g. `cursor_mcp.rs`'s per-field extra-metadata check). Reusing the existing per-document logic (goal 1) means this nuance carries over for free.

**3. `inspect --coverage` iterates the same emitter registry `inspect --to` already switches on.**

No new registry/plugin mechanism. `src/cli/commands/inspect.rs` already has an exhaustive match over `OutputFormat` → `Emitter` impl; `--coverage` reuses that same list, calling `.capabilities(&doc)` on each instead of `.emit(&doc, strict)`.

**4. JSON output shape: flat array of `{target, entity_kind, status}` records, not a nested matrix object.**

A flat array pipes cleanly into `jq` (`jq '.[] | select(.status == "dropped")'`) without needing to know the target list up front, consistent with the project's existing Unix-pipeline philosophy (PRD "Pipeline-First Transformations"). A nested `{target: {entity_kind: status}}` object was considered and rejected as harder to filter with `jq` for the common case (find everything that's lossy/dropped) without first enumerating keys.

Each record also carries a `reason: Option<String>` field for `lossy`/`dropped` statuses (`null` for `supported`), populated from the same message text the shared helper already builds for the `eprintln!` call. Without it, a `--json` consumer can see *that* something is lossy but not *what* was lost, contradicting the coverage-reporting spec's claim that JSON output needs no additional prose-parsing — this closes that gap at near-zero cost, since the string already exists.

`entity_kind` and `target` values use the same kebab-case identifiers already used elsewhere in the CLI/IR, not the emitters' internal PascalCase Rust type names: `entity_kind` matches the IR's `#[serde(tag = "kind")]` values (`rule`, `skill`, `mcp-server`, `hook`, `agent`, `permissions`, as seen in `ir.json` output), and `target` matches `OutputFormat`'s existing `#[serde(rename_all = "kebab-case")]` values (`claude`, `cursor-mdc`, `cursor-mcp`, `codex`, `copilot`, `gemini`, `windsurf`, `agent-skills`). A consumer who already parsed `inspect`'s IR JSON output moments earlier should not have to learn a second casing convention for the coverage JSON.

**5. `--coverage` reuses the existing global `--strict` flag for CI gating; no new flag.**

The project already has a flag whose documented meaning is "fail on warnings, including lossy conversion warnings" (PRD global flags). `rulette inspect <input> --coverage --strict` exits non-zero if any cell in the matrix is `Lossy` or `Dropped`; without `--strict`, `--coverage` always exits 0 and is purely informational (matching `inspect`'s existing behavior for `--to`). This was an explicit gap in the first draft of this design — the JSON output's stated purpose is CI/script consumption, but nothing defined pass/fail semantics. Reusing `--strict` avoids inventing a parallel `--fail-on=<status>` flag with its own semantics to design and document, and keeps `--coverage`'s CI story consistent with every other lossy-conversion warning path in the codebase.

## Risks / Trade-offs

- **[Risk]** Extracting shared loss-detection helpers out of 8 emitters is a real refactor touching every emitter file, not just additive code. → Mitigation: do it emitter-by-emitter with existing emitter unit tests as the regression guard; no emitter's `eprintln!` wording needs to change, only where the underlying boolean/enum determination is computed.
- **[Risk]** `capabilities()` and `emit()` could still drift if an emitter's author updates one without the other, and if they drift, the coverage matrix reports a status that doesn't match what `emit()` actually does — the exact failure mode a CI gate built on `--coverage --strict` (Decision 5) depends on not happening. → Mitigation: shared helper functions (Decision 1) make the two paths call the same code; additionally, a required task (tasks.md) adds a test that, for every emitter, exercises a document containing every entity kind and asserts `capabilities()`'s per-kind status agrees with whether `emit(strict=true)` actually errors/warns for that same document. Not deferred to a future change.
- **[Trade-off]** Entity-kind-level granularity (not field-level) means the coverage matrix can say "McpServer is Lossy on Cursor MCP" but not "specifically the `env` field is dropped" from the JSON `status` field alone. Mitigated by the `reason` string (Decision 4) for a human-readable hint without a second CLI call; still not structured per-field data. Acceptable per Non-Goals; field-level detail is a natural follow-up, not blocking.

## Open Questions

Resolved during adversarial review (see Decisions 1a/1b/4/5 above): Dropped/Lossy/Supported classification rule, multi-instance aggregation, JSON casing convention, `reason` field, and `--strict` exit-code gating were all previously open and are now decided.

- `--coverage` and `--to <format>` are mutually exclusive on the same `inspect` invocation (simpler mental model: single-target detail view vs. all-targets summary view are two distinct modes, not composable). `clap` should enforce this via `conflicts_with` rather than a runtime check.
- Exact flag shape: `--coverage` (table output by default) plus a `--json` modifier, e.g. `rulette inspect <input> --coverage --json`, reusing the same `--json`-as-modifier pattern rather than a combined `--coverage-format=table|json` enum flag, for consistency with how `--strict` already composes with `--coverage` as an independent modifier rather than a mode enum.
