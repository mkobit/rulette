## Context

`--config <path>` exists today (`src/cli/commands/transform.rs:59-82,280-301`) but only reads TOML into a flat `TransformConfig { filter, exclude, rename, set, to, out }` — one optional string per pipeline step, no `inputs`, no per-output granularity, no reverse (scaffold) direction. `docs/2026-08-18-cli-ux-design.md` §1 specifies the target shape: multi-syntax parsing, an ordered `pipeline` array, a structured `outputs` array with per-target `entities`/`drop`/`strict`, an `inputs` field with a defined CLI-composition rule, and `--to transform-config` to scaffold a manifest from an existing repo.

Relevant existing code, read in full this session:

- `TransformArgs::execute` (`src/cli/commands/transform.rs:227-637`): reads inputs, applies overrides, loads `--config`, runs pipeline steps in a fixed order (filter → exclude → rename → set), resolves `run_targets` via `parse_targets`, then loops over `run_targets` building one output per target from the *same* `doc`.
- `parse_targets` (`transform.rs:97-172`): already handles a bare (no `:`) `-o <path>` argument by pairing it with `--to <format>` — this is reused as-is for the scaffold direction (see Decision 6).
- `InputFormat`/`OutputFormat` (`src/cli/formats.rs:9-42`).
- `parse()` (`src/parsers/frontend.rs:11-117`): `InputFormat::Auto` cannot distinguish Claude/Codex/Copilot/Windsurf/CursorLegacy from each other by *content* — all four plain-markdown tools fall through the same "has frontmatter? no → `parse_claude`" branch. Tool identity for those formats is recoverable only from the **input path's naming convention** (directory/filename), not from parsing the body. This directly shapes Decision 6.
- `openspec/changes/coverage-reporting/{proposal,design,tasks}.md`: structural and decision-recording template for this document.

## Goals / Non-Goals

**Goals:**

- Replace `TransformConfig` with a schema matching `docs/2026-08-18-cli-ux-design.md` §1: `inputs`, `pipeline` (ordered steps), `outputs` (structured per-target entries).
- Accept `.toml`, `.json`, `.jsonc`, `.json5` by extension against one internal schema.
- Define exactly how CLI flags compose with a loaded config (the source doc says both "compose with" and "override" without reconciling the two for list-shaped fields — resolved below).
- Implement `--to transform-config --out <path>` scaffolding from on-disk inputs.
- Keep `--filter`/`--exclude`/`--rename`/`--set`/`--check` CLI-flag behavior for invocations with no `--config` byte-for-byte unchanged.

**Non-Goals:**

- `scope` tier *behavior* (user/enterprise/local semantics, per-tool support matrix) — `rulette-5bk.7`. This change parses and round-trips the `scope` field but only `"project"` has any effect on emission.
- `rulette:activation` per-target override blocks — `rulette-5bk.8`, unrelated file.
- Adding OpenCode/Antigravity as targets — `rulette-5bk.9`/`.10`. The config schema must not hardcode the five-tool list; `target` accepts whatever `OutputFormat` already supports.
- A `rulette transform --to json-schema` validator for the config file itself. `schemars` is already a dependency and this would be cheap, but it's not asked for anywhere in the source doc — speculative, left for later if wanted.
- Migrating existing on-disk configs written against the old flat shape. `--config` is never auto-discovered (stated invariant, both in this repo's docs and the source design doc), so no config file is checked-in state Rulette itself must migrate; only `docs/2026-04-11-prd.md:162`'s own line needs updating (Impact section).

## Decisions

**1. Config schema types.**

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TransformConfigFile {
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    pipeline: Vec<PipelineStep>,
    #[serde(default)]
    outputs: Vec<OutputEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PipelineStep {
    Filter(String),
    Exclude(String),
    Rename(String),
    Set(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputEntry {
    target: OutputFormat,
    #[serde(default = "default_scope")]
    scope: String,           // validated against {project,user,enterprise,local}; only "project" has behavior yet
    path: String,
    #[serde(default)]
    entities: Option<Vec<String>>,  // allow-list, IR kebab-case kind tags; each token validated
    #[serde(default)]
    drop: Option<Vec<String>>,      // deny-list, applied after entities; each token validated
    #[serde(default)]
    strict: Option<bool>,           // per-output override of the global --strict, can escalate or relax
}
```

`PipelineStep`'s default (externally-tagged) serde representation of a Rust enum *is* `{"filter": "..."}` / `{"exclude": "..."}` / etc. — no custom `Deserialize` impl needed; this is exactly the doc's `pipeline` shape for free.

`#[serde(deny_unknown_fields)]` on both `TransformConfigFile` and `OutputEntry` is deliberate: loading an old-shape config (`filter`/`exclude`/`rename`/`set`/`to`/`out` at the top level) now fails with serde's "unknown field `filter`, expected one of `inputs`, `pipeline`, `outputs`" — a clear signal of *what* changed, in lieu of a migration path (Non-Goals) — and a typo inside an output entry (e.g. `"paths"` instead of `"path"`) fails the same way instead of silently doing nothing.

`entities`/`drop` reuse the entity-kind kebab-case tokens already established by `coverage-reporting` (`rule`, `skill`, `mcp-server`, `hook`, `agent`, `permissions`) rather than inventing a second vocabulary, and — like `scope` — each token is validated against that closed set at config-load time. Without this, a typo like `"entities": ["rules"]` would silently produce an output with zero matching entities: no error, no warning, and no config-load-time signal that anything is wrong. That is exactly the "silent drop" failure mode Decision 6's unmatched-path warning is designed to avoid elsewhere in this same file format, so `entities`/`drop` get the identical fail-fast treatment as `scope`, not a lesser one.

`scope` stays a validated `String`, not a typed enum, so `rulette-5bk.7` can grow real tier behavior without a breaking type change here — only the validation set (currently the four known tokens) is a design surface, not the wire type.

`strict: Option<bool>` is a bidirectional override, not only an escalation: an output can set `strict: false` to relax below an invocation-wide `--strict`, not just set `strict: true` to escalate above an invocation with no `--strict`. The source doc's own wording ("escalates … mirroring the global `--strict`") only illustrates the escalate direction, but nothing in it forbids the relax direction, and a plain `Option<bool>` override is simpler than a tri-state "escalate-only" type — the relax direction is exercised explicitly in tasks.md/spec.md rather than left as an untested side effect of the type choice.

**2. Multi-syntax parsing: extension dispatch, three parsers.**

| Extension | Parser | Rationale |
| --- | --- | --- |
| `.toml` | `toml` crate (already a dependency) | unchanged from today |
| `.json` | `serde_json` (already a dependency) | strict JSON, no comments — matches user expectation for a plain `.json` file |
| `.jsonc`, `.json5` | new dependency: `json5` crate | JSON5's grammar is a strict superset of "JSON plus comments and trailing commas" (it also permits unquoted keys and single-quoted strings), so it parses valid `.jsonc` content without a second, JSONC-only crate. One parser covers both extensions. |

Alternative considered: a dedicated JSONC-only crate (e.g. `jsonc-parser`) for `.jsonc`, keeping `.json5`'s extra liberties (unquoted keys, trailing commas in more positions) exclusive to `.json5`. Rejected — it adds a second dependency to enforce a stricter grammar than the doc requires; nothing in the source doc says `.jsonc` must *reject* JSON5-only syntax, and the canonical examples (§1) are valid under either parser.

**Decision, stated once, no ambiguity**: an unrecognized extension is parsed with the `json5` crate (the canonical-form parser), and on failure that parser's error is surfaced directly. This is a single attempt, not a fallback chain across all three parsers — trying `.toml` then `.json` then `.json5` in sequence on an ambiguous file would risk a wrong-but-successful parse (e.g. a `.json5`-syntax file that also happens to be valid, differently-meaning TOML is not a realistic concern given the schema's shape, but "try everything until something doesn't error" is a worse failure mode than one clear parser and one clear error). Single-attempt-with-`.json5` also matches the canonical-form framing already established for `.jsonc`/`.json5`.

**2a. Scaffold serialization never uses `json5` — only `toml`/`serde_json`, and never emits comments.**

The `json5` crate is evaluated above purely for *parsing* `--config` input. The scaffold direction (Decision 6/7) needs to *write* a generated manifest, and the source doc's own scaffold example (§1, "The scaffold is a starting point") shows hand-authored-style inline comments (a leading `// rulette.transform.jsonc (generated by …)` header, a `// TODO:` hint). Serde's data model has no comment concept, and not every `json5`-family crate on crates.io even exposes a serializer (several are parse-only, since JSON5 is meant to be hand-authored) — this design does not assume one exists.

**Decision**: scaffold output is generated with `toml::to_string` when the target path's extension is `.toml`, and `serde_json::to_string_pretty` (plain, comment-free JSON — a valid subset of both `.jsonc` and `.json5`) for `.json`, `.jsonc`, `.json5`, or no/unrecognized extension (stdout case). No comments are generated. This is stated as a deliberate scope boundary, not a gap discovered mid-implementation: the doc's inline comments are documentation flavor for a hand-written illustrative example, not a requirement that `--to transform-config` itself emit prose commentary. A generated `.jsonc` file is therefore valid, parseable, comment-free JSON that merely uses the `.jsonc` extension — round-tripping it back through `--config` (Requirement "Loading a canonical .jsonc config") still works, since the `.jsonc` parser accepts plain JSON as a subset.

**3. `inputs` precedence, and how the CLI can tell "explicit" from "defaulted."**

Today `TransformArgs::input: Vec<String>` has `#[arg(default_value = "-")]`, so it is *never* empty — there is no way to distinguish "user passed no positional inputs" from "user explicitly typed `-`". The doc's rule ("usage error only when *both* the config and the CLI set `inputs`") needs that distinction.

**Change**: drop `default_value = "-"` from `TransformArgs::input`, leaving it a plain `Vec<String>` (empty when nothing is passed — clap's default for a `Vec`-typed positional with no default). Resolution in `execute()`:

```text
if config.inputs is non-empty AND self.input is non-empty → usage error
resolved_inputs =
    self.input if non-empty
    else config.inputs if non-empty
    else vec!["-"]   // preserves today's "no input ⇒ read stdin" default
```

Alternative considered: keep the clap default and detect "explicit" by comparing `self.input == ["-"]`. Rejected — indistinguishable from a user who explicitly runs `rulette transform - --config …` intending to read stdin, which is a real, documented use of `-`.

**Ordering constraint this creates, stated explicitly**: `resolved_inputs` needs `config.inputs`, which means config-loading must happen *before* `read_inputs(&self.input)` is called. Today's `execute()` (`transform.rs:227-637`) calls `read_inputs` at line 231, and only loads `--config` afterward, at lines 280-301 — after input reading, after parsing, after metadata overrides. This decision requires **moving the config-load block to the top of `execute()`**, ahead of all of that, not a one-line substitution at the `read_inputs` call site. This is called out here as a real restructuring, not an implementation nicety, precisely because it's easy to under-scope as "just change what variable `read_inputs` takes."

**4. Pipeline composition: config runs first, CLI flags append.**

The source doc's own wording ("CLI flags compose with **and** override the config") is ambiguous once `pipeline` is a list rather than one scalar per operation — its only worked example (`--check`) has no config-file analog at all, so it doesn't disambiguate. Resolved as two different rules for two different fields:

- **`pipeline`**: config's steps run first, in file order; any CLI `--filter`/`--exclude`/`--rename`/`--set` flags run *after*, in that fixed order (matching today's code order at `transform.rs:305-335`) — i.e., CLI flags **compose** (append), they do not replace the config's pipeline. Rationale: replacing a checked-in pipeline outright because a CI job added one extra `--filter` would silently drop the rest of the file's steps — surprising for exactly the CI-composition case the doc's example is trying to illustrate.
- **`outputs`**: if any `-o`/`--to` is given on the CLI, it **replaces** the config's `outputs` entirely (today's existing scalar "CLI wins if set" behavior, extended unchanged to the list). Rationale: a CI job redirecting output to a scratch directory for a dry run wants exactly its own destinations, not the checked-in ones plus its own.

This is a genuine behavior change for the `pipeline` half, not a continuation of today's code: today's actual logic (`transform.rs:283-294`) is pure "CLI wins if set, else config" (full replacement) for `filter`/`exclude`/`rename`/`set` — there is no existing append/compose precedent anywhere in this codebase. The compose-vs-replace split is justified on its own terms above (by what each of the doc's two illustrated use cases needs), not by continuity with existing behavior. `run_targets`'s CLI-wins-if-present resolution *is* an unchanged continuation of today's code, extended from a scalar to a list.

**5. Per-output `entities`/`drop`/`strict` are applied per target, not globally, carried on an extended `OutputTarget`.**

Today one `doc: RuletteDocument` is built once and reused for every target in the `for target in run_targets` loop (`transform.rs:431-465`), and `OutputTarget` (`transform.rs:84-87`) is just `{ format, path }` — it has nowhere to carry a config entry's `entities`/`drop`/`strict` into that loop. `OutputTarget` gains three optional fields:

```rust
pub struct OutputTarget {
    pub format: OutputFormat,
    pub path: Option<String>,
    pub entities: Option<Vec<String>>,
    pub drop: Option<Vec<String>>,
    pub strict: Option<bool>,
}
```

`parse_targets`'s existing CLI-only construction sites (`transform.rs:105,109,138,151`) set all three new fields to `None`; only the new config-`outputs`-to-`OutputTarget` conversion (Decision 4) populates them from an `OutputEntry`.

Per-output `entities`/`drop` require a *filtered view* of `doc.entities` specific to that one output, since different outputs in the same invocation can request different subsets (the whole point of §1's per-target granularity). Implementation: for each target, build a filtered `RuletteDocument` (clone `doc`, retain entities whose kebab-case kind is in `entities` if `Some`, then remove any whose kind is in `drop` if `Some`) before calling that target's `emit()`; `strict` for that call is `target.strict.unwrap_or(self.strict)` (falls back to the global `--strict` flag; can escalate above or relax below it, per Decision 1). A target with `entities: None, drop: None` (every CLI-only target, and any config output that didn't set either) is unfiltered, identical to today's behavior.

Alternative considered: filter `combined_entities` once, globally, before the target loop. Rejected outright — it cannot express "target A gets rules+skills, target B gets everything except hooks" in the same invocation, which is the documented use case (§1 example).

**6. `--to transform-config` infers each output's target format from the *input path's* naming convention, not from parsed content.**

Per Context, `InputFormat::Auto` cannot distinguish Claude/Codex/Copilot/Windsurf by content — they're textually near-identical plain markdown. Tool identity is recoverable only from path shape. A new static table drives both scaffold inference and default-path generation. **Order matters: it is evaluated top to bottom, first match wins**, so more specific conventions are listed before more general ones that would otherwise also match:

```rust
const TOOL_PATH_CONVENTIONS: &[(fn(&Path) -> bool, OutputFormat, &str)] = &[
    (|p| p.file_name() == Some("AGENTS.md".as_ref()), OutputFormat::Codex, "AGENTS.md"),
    (|p| p.file_name() == Some("CLAUDE.md".as_ref()), OutputFormat::Claude, ".claude/"),
    (|p| p.components().any(|c| c.as_os_str() == ".claude"), OutputFormat::Claude, ".claude/"),
    // More specific first: an mcp.json path under .cursor must not fall through to the
    // generic .cursor directory matcher below it, or every Cursor MCP config would be
    // misclassified as a cursor-mdc rules directory.
    (|p| p.file_name() == Some("mcp.json".as_ref())
        && p.components().any(|c| c.as_os_str() == ".cursor"), OutputFormat::CursorMcp, ".cursor/mcp.json"),
    (|p| p.components().any(|c| c.as_os_str() == ".cursor"), OutputFormat::CursorMdc, ".cursor/rules/"),
    (|p| p.components().any(|c| c.as_os_str() == ".opencode"), OutputFormat::OpenCode, ".opencode/"), // once rulette-5bk.9 lands
];
```

Table coverage is deliberately limited to the targets this repo actually implements or has committed to (`codex`, `claude`, `cursor-mdc`, `cursor-mcp`, and `opencode`/`antigravity` once `rulette-5bk.9`/`.10` land) — **not** `windsurf`/`copilot`/`gemini`/`agent-skills`. This follows the source doc's own "Scope of tools" section (§0), which explicitly defers those four to later; an input path belonging to one of them simply produces the Decision 6 "unmatched path" warning today, same as any other unrecognized convention, until a future change both adds them to `OutputFormat`'s core-five scope and to this table together. (This is a statement of *current* deliberate scope, not a claim that those four targets can never appear here.)

For each of the invocation's `resolved_inputs` (Decision 3) — the literal, explicitly-given CLI/config input paths, **not** the recursively-walked file list `read_inputs` expands them into — run it through the table; the first match contributes one `OutputEntry { target, scope: "project", path: <default path>, entities: None, drop: None, strict: None }` to the generated manifest, deduplicated by target: multiple matched inputs mapping to the *same* target contribute exactly one `outputs` entry, not one per input.

**Why matching only the literal top-level input paths (not the recursive walk) is the right scope, not a gap**: every worked example in the source doc (§1, §5 Journey 3) passes each tool's directory or file explicitly as its own positional argument (`.claude/`, `.cursor/`, `AGENTS.md`) — never a single repo-root path like `.`. Recursively discovering tool conventions from an arbitrary root is a materially bigger feature (walking an unbounded tree, guessing at nested conventions with no input boundary) that the source doc neither shows nor asks for. **Non-Goal, stated explicitly**: `rulette transform . --to transform-config --out …` is not a supported "auto-discover everything under this root" invocation in this change; it resolves to one `resolved_inputs` entry (`"."`) matching no convention in the table, producing zero `outputs` and one unmatched-path warning — a correct, if unhelpful, outcome for an unsupported usage, not silently-wrong behavior.

**Why deduplicating by target does not lose Codex's nested-`AGENTS.md` fidelity**: `rulette-5bk`'s already-shipped Codex support infers `rulette:directory-scope` (`src/parsers/frontend.rs:451-487`) as **entity-level IR metadata**, not as an outputs-list concern — a document containing entities from both a root `AGENTS.md` and a nested `src/backend/AGENTS.md` already carries that nesting on each `Rule`'s `extra["rulette:directory-scope"]`, and the existing Codex emitter reconstructs the nested files from that metadata at emission time, from a *single* `{codex, "AGENTS.md"}` output target — the same way `.claude/` today produces many files from one output target. Scaffolding one deduplicated `codex` entry for a repo with several nested `AGENTS.md` inputs therefore does not drop anything: re-running `--config <generated>` re-parses all of `resolved_inputs` (which the scaffold's `inputs` field already reproduces in full, nested files included), re-derives the same directory-scope metadata, and the one `codex` output reconstructs the same nested layout. This is stated explicitly here because it is the one place in this design where "fewer output entries than input files" could look like a fidelity loss at a glance; it isn't, because the outputs list and the IR's own metadata operate at different levels.

Alternative considered: require explicit `-o target:path` pairs on the scaffold invocation instead of inferring anything (simpler, reuses the emitter loop as-is with real targets, `outputs` would be built directly from `run_targets`). Rejected — it contradicts the source doc's flagship example, which passes **zero** `-o` flags and relies entirely on inference to reproduce "the tools found on disk"; requiring explicit flags would just be `transform-config` re-encoding a command the user already had to type in full, defeating the onboarding purpose of §1's "scaffold from an existing repo" (Journey 3).

Alternative considered: content-based `InputFormat::Auto` detection reused for inference. Rejected per Context — it cannot distinguish the four plain-markdown tools from each other; only path convention can.

**7. `OutputFormat::TransformConfig` is a new emitter-loop match arm, not a new `Emitter` impl.**

`--to transform-config --out <path>` composes with the *existing* `-o`/`--to` machinery: `parse_targets`'s existing bare-path fallback (`transform.rs:150-159`, "if it's not `format:path` and no `--to`... " — actually the *has*-`--to` branch) already produces `OutputTarget { format: TransformConfig, path: Some(<out path>) }` from `--to transform-config --out rulette.transform.jsonc` with **no changes to `parse_targets`**. The new work is confined to one match arm in the emission loop (`transform.rs:432-463`) that, instead of calling an `Emitter::emit`, runs the Decision 6 inference over `resolved_inputs` (Decision 3) and serializes a `TransformConfigFile`-shaped value using the syntax implied by the target's own path extension (defaulting to `.jsonc` for stdout, matching the canonical form).

This is deliberately *not* wired through the `Emitter` trait (`src/emitters/mod.rs`) — `Emitter::emit` takes a parsed `&RuletteDocument` and has no way to see the original *input paths* it needs for Decision 6's inference, and `coverage-reporting`'s `capabilities()` addition already established the precedent that not every output mode needs to be an `Emitter`.

**7a. Adding `OutputFormat::TransformConfig` also touches `inspect.rs`, which must reject it, not silently mishandle it.**

`OutputFormat` is shared between `transform --to`/`-o` and `inspect --to` (`src/cli/commands/inspect.rs`, which has its own exhaustive `match to { ... }` over every `OutputFormat` variant, no wildcard arm — confirmed by reading the file). Adding a 12th variant is a compile error there until a matching arm exists. `inspect --to transform-config` has no sensible behavior under Decision 6/7: `inspect` operates on an already-parsed `&RuletteDocument`, with no equivalent of `resolved_inputs` (the original, invocation-level CLI input path list) available to it — the same reason Decision 7 keeps this out of the `Emitter` trait applies equally to `inspect`. **Decision**: `inspect --to transform-config` is a deliberate, explicit usage error ("`transform-config` is only a valid target for the `transform` command, not `inspect`"), added as its own match arm in `inspect.rs` — a stated scope boundary, not an oversight discovered by a compiler error during implementation.

**8. `pipeline`/`outputs` empty-vs-absent: an omitted key and an empty array are equivalent (`#[serde(default)]`).** No behavioral distinction is given any meaning — avoids a footgun where `"pipeline": []` written explicitly would behave differently from omitting the key entirely.

## Risks / Trade-offs

- **[Risk]** `#[serde(deny_unknown_fields)]` means any config field this design didn't anticipate becomes a hard parse error instead of being silently ignored. → Mitigation: this is the intended behavior for the old-shape migration story (Decision 1) and is consistent with the project's "the IR never drops what it does not understand" ethos applying to *known* extension points (`extra`), not to top-level schema typos — a hard error here is a feature, not a gap.
- **[Risk]** The path-convention table (Decision 6) is a second place (besides parsers/emitters) that encodes "what does tool X's config tree look like," and can drift from the real per-tool paths as `rulette-5bk.9`/`.10` add OpenCode/Antigravity. → Mitigation: the table only needs one entry per target added by those changes; call out in their design docs that this table needs a matching entry, same as `openspec/specs/frontends-and-backends/spec.md` needs a new emitter section.
- **[Trade-off]** Decision 4's split rule (pipeline composes, outputs override) is an asymmetry a reader has to learn — not a single uniform "CLI always wins" or "CLI always appends" story. → Accepted: the source doc's own example set only supports the split behavior once `pipeline` becomes list-shaped (see Decision 4's rationale); a uniform rule would violate one of the two illustrated use cases (CI adding a filter without dropping the rest of the file, vs. CI redirecting output entirely).
- **[Trade-off]** Adding the `json5` crate is a new runtime-uninvolved *build* dependency. Does not affect the "fully static binary, no runtime dependencies" hard constraint (AGENTS.md) — it is pure-Rust code compiled into the static binary, not a dynamic library or network call at runtime, same category as `toml`/`serde_yaml` already in the tree. It is used for parsing only (Decision 2a); no assumption is made about its serialization support.
- **[Risk]** Decision 3's ordering constraint (config-load must move ahead of `read_inputs` and everything currently before it in `execute()`) is a real control-flow restructuring of `TransformArgs::execute`, not a localized change — a naive implementation could reorder incorrectly and silently change behavior for invocations that don't even use `--config` (e.g. if `--name`/`--description` overrides or the identity-collision check accidentally moved relative to parsing). → Mitigation: tasks.md sequences this as its own task ahead of the rest of section 3, and task 8 includes a regression check that no-`--config` invocations are behaviorally identical before and after the reorder (existing CLI integration tests as the guard, per `coverage-reporting`'s same pattern for its own emitter refactor).
- **[Risk]** `transform.rs` has no `#[test]`-attributed unit tests today; `tests/cli_tests/transform_tests.rs` exercises only the `format:path` colon syntax for `-o`, never the bare `--to <format> --out <path>` fallback Decision 7 depends on. That fallback path is therefore unverified by the existing suite, not merely "already covered" — tasks.md treats adding that coverage as required, not conditional.

## Migration Plan

No runtime migration — `--config` is never auto-discovered, so no committed state depends on the old schema surviving. Steps are purely in-repo:

1. Land the new `TransformConfigFile`/`PipelineStep`/`OutputEntry` types alongside the existing flat `TransformConfig`, gated behind extension dispatch (new types used for `.jsonc`/`.json5`/`.json`, old type still used for `.toml`) — **rejected**, adds a second schema and a permanent forked code path for no real benefit (`.toml` is a first-class citizen of the new schema too, per the doc). Instead: replace `TransformConfig` outright in one change; `mise run check`'s existing test suite (transform CLI integration tests) is the regression guard, updated in the same change (tasks.md).
2. Update `docs/2026-04-11-prd.md:162` (`--config <path>` description) in the same change that ships the new parser, so docs and behavior never diverge even transiently.
3. Rollback is a plain revert — no data migration, no on-disk state to reconcile.

## Open Questions

Resolved during drafting (see Decisions above, each was a genuine gap in the source doc, not merely an implementation nicety): pipeline-vs-outputs CLI composition rule (4), how `entities`/`drop`/`strict` scope per-output rather than globally (5), how the scaffold direction infers targets without content-based detection (6), and how `--config`+positional-inputs "usage error" is actually detectable given clap's current default (3).

Resolved during adversarial review (a second pass, after the first draft of this design): the unrecognized-extension parsing strategy was a genuine single-parser-vs-fallback-chain contradiction between an earlier draft of this section and the spec/tasks artifacts, now stated once (2); scaffold serialization was previously silent on whether the `json5` crate supports writing at all, now decided to never depend on it for output (2a); `entities`/`drop` tokens had no validation while `scope` did, now aligned (1); `OutputTarget` had no field to carry per-output `entities`/`drop`/`strict` into the emission loop, now specified (5); the `.cursor` path matcher conflated `cursor-mdc` and `cursor-mcp`, and had no bare-`CLAUDE.md` matcher, now split and added (6); `inspect.rs`'s exhaustive `OutputFormat` match was an undiscovered compile break, now an explicit rejection (7a); and the config-load-before-`read_inputs` ordering dependency was implicit, now stated as its own constraint (3).

- Exact wording/format of the "unmatched path, no output entry" warning from Decision 6 — left to implementation (tasks.md), consistent with existing lossy-conversion warning phrasing elsewhere in the codebase (`eprintln!`-based, `--strict`-escalatable).
- Whether `TOOL_PATH_CONVENTIONS` (Decision 6) should also drive default-path resolution for the *normal* (non-scaffold) `-o <format>` bare-path case — out of scope here; today's `-o <format>` with no path already requires the emitter's own path-join logic, unrelated to this change's inference table. Worth revisiting once `rulette-5bk.7`'s scope tiers add more per-format default paths, but not blocking here.
