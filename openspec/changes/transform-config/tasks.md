## 1. Config schema types

- [x] 1.1 Replace `TransformConfig` in `src/cli/commands/transform.rs` with `TransformConfigFile { inputs: Vec<String>, pipeline: Vec<PipelineStep>, outputs: Vec<OutputEntry> }`, all three `#[serde(default)]`, struct annotated `#[serde(deny_unknown_fields)]`.
- [x] 1.2 Define `PipelineStep` as an externally-tagged enum (`Filter(String)`, `Exclude(String)`, `Rename(String)`, `Set(String)`, `#[serde(rename_all = "lowercase")]`) so its default serde shape is `{"filter": "..."}` etc.
- [x] 1.3 Define `OutputEntry { target: OutputFormat, scope: String (default "project"), path: String, entities: Option<Vec<String>>, drop: Option<Vec<String>>, strict: Option<bool> }`, also annotated `#[serde(deny_unknown_fields)]`.
- [x] 1.4 Add scope-token validation (`{project, user, enterprise, local}`) run at config-load time, rejecting unknown tokens with an error naming the invalid value.
- [x] 1.5 Add entity-kind-token validation for every token in `entities` and `drop` (`{rule, skill, mcp-server, hook, agent, permissions}`, same closed set `coverage-reporting` uses), rejecting unknown tokens with an error naming the invalid value — same fail-fast treatment as 1.4, not a lesser one (design.md Decision 1).
- [x] 1.6 Extend `OutputTarget` (`transform.rs:84-87`) with `entities: Option<Vec<String>>`, `drop: Option<Vec<String>>`, `strict: Option<bool>`; update every existing CLI-only construction site (`transform.rs:105,109,138,151`) to set all three to `None`.
- [x] 1.7 (found during implementation) `OutputEntry` -> `OutputTarget` conversion must treat `path: "-"` as stdout (`None`), matching `parse_targets`'s existing `-o format:-` convention — the naive `Some(entry.path.clone())` conversion instead created a literal file/directory named `-` on disk, caught by a stray `-/` directory appearing during test runs.

## 2. Multi-syntax parsing (config *input* only — see task 6.3 for why scaffold *output* never uses this)

- [x] 2.1 Add the `json5` crate to `Cargo.toml`.
- [x] 2.2 Implement extension-dispatch loading: `.toml` → `toml::from_str`, `.json` → `serde_json::from_str`, `.jsonc`/`.json5` → `json5::from_str`, unrecognized extension → `json5::from_str`, surfacing its error on failure (single attempt, not a fallback chain across all three parsers — design.md Decision 2).
- [x] 2.3 Verify `#[serde(deny_unknown_fields)]` produces a clear "unknown field" error when a config still uses the old flat shape (`filter`/`exclude`/`rename`/`set`/`to`/`out` at the top level) under each of the four syntaxes.

## 3. Inputs precedence and config-load ordering

- [x] 3.1 Move the `--config` loading block (currently `transform.rs:280-301`) to the top of `execute()`, ahead of `read_inputs` (currently called at line 231) and everything else currently ahead of it. This is a real control-flow reorder, not a one-line substitution (design.md Decision 3 / Risks) — confirm no other logic (metadata overrides, identity-collision detection) implicitly depended on the old ordering.
- [x] 3.2 Remove `#[arg(default_value = "-")]` from `TransformArgs::input`, leaving it a plain (possibly empty) `Vec<String>`.
- [x] 3.3 In `execute()`, compute `resolved_inputs`: usage error if both `config.inputs` and `self.input` are non-empty; else CLI positionals if non-empty; else config inputs if non-empty; else `vec!["-"]`.
- [x] 3.4 Update the `read_inputs` call site to use `resolved_inputs` instead of `&self.input` directly.

## 4. Pipeline and output composition

- [x] 4.1 Build the effective pipeline-step list as `config.pipeline` followed by any CLI `--filter`/`--exclude`/`--rename`/`--set` flags appended in that fixed order (each CLI flag becomes zero or one `PipelineStep` appended after the config's steps).
- [x] 4.2 Execute the effective pipeline-step list in order, replacing the current fixed filter → exclude → rename → set block (`transform.rs:305-335`) with a loop over the ordered steps.
- [x] 4.3 Resolve `run_targets`: if any CLI `-o`/`--to` is present, use CLI-derived `OutputTarget`s only (`entities`/`drop`/`strict` all `None`, ignore `config.outputs` entirely); else convert `config.outputs` into `OutputTarget`s (populating the task 1.6 fields from each `OutputEntry`); else fall back to today's default (`ir-json` to stdout).

## 5. Per-output entity and strictness scoping

- [x] 5.1 For each target in `run_targets` with `entities` or `drop` set (`Some`), build a filtered `RuletteDocument` clone before calling `emit()`: retain entities whose kebab-case kind is in `entities` when `Some`, then remove entities whose kind is in `drop` when `Some`. A target with both `None` (every CLI-only target) uses the shared unfiltered `doc`, unchanged from today.
- [x] 5.2 Thread each target's effective `strict` (`target.strict.unwrap_or(self.strict)`) into that target's `Emitter::emit` call, replacing the single global `strict` argument used uniformly today. Confirm this can both escalate above and relax below the global `--strict` (design.md Decision 1).
- [x] 5.3 Confirm CLI-only invocations (no `--config`) are unaffected: every output continues to share one unfiltered document and the single global `--strict` value, exactly as today.

## 6. Scaffold: `--to transform-config`

- [x] 6.1 Add `OutputFormat::TransformConfig` (`transform-config` kebab-case token) to `src/cli/formats.rs`.
- [x] 6.2 Define the path-convention table (design.md Decision 6): ordered, most-specific-first list of `(matcher: fn(&Path) -> bool, OutputFormat, default_path: &str)` — `AGENTS.md` filename → codex; `CLAUDE.md` filename → claude; `.claude` path component → claude; `.cursor/mcp.json` (filename `mcp.json` under a `.cursor` component, checked *before* the generic `.cursor` matcher below it) → cursor-mcp; `.cursor` path component → cursor-mdc; `.opencode` path component → opencode (once `rulette-5bk.9` lands). Deliberately excludes `windsurf`/`copilot`/`gemini`/`agent-skills` — out of scope per the source doc's "Scope of tools" (design.md Decision 6).
- [x] 6.3 Implement the `OutputFormat::TransformConfig` arm in the emission loop (`transform.rs:432-463`): run `resolved_inputs` (task 3.3) — the literal invocation-level input paths, not a recursive directory walk — through the path-convention table, deduplicate matches by target, build a `TransformConfigFile` with `inputs: resolved_inputs` (unmodified) and the inferred `outputs`. Serialize with `toml::to_string` when the target path's extension is `.toml`, else `serde_json::to_string_pretty` (plain JSON, no comments) for `.json`/`.jsonc`/`.json5`/no-extension/stdout — never the `json5` crate for writing (design.md Decision 2a; that crate's serialization support is not assumed or required).
- [x] 6.4 Print a warning for each `resolved_inputs` entry that matches no convention, naming the unmatched path, without failing the invocation.
- [x] 6.5 Add the missing regression coverage for `parse_targets`'s existing bare-path fallback (`transform.rs:150-159`): confirmed by reading `tests/cli_tests/transform_tests.rs` that only the `format:path` colon syntax is exercised today, never bare `-o <path>` + `--to <format>`. This must be added, not treated as conditional (design.md Risks).
- [x] 6.6 Add an `OutputFormat::TransformConfig` match arm to `src/cli/commands/inspect.rs`'s exhaustive `match to { ... }` that rejects `inspect --to transform-config` as a usage error (design.md Decision 7a) — required for the codebase to compile once task 6.1 lands, and confirmed no other exhaustive `match` over `OutputFormat` exists elsewhere in `src/` that would need the same treatment.

## 7. Docs

- [x] 7.1 Update `docs/2026-04-11-prd.md:162`'s `--config <path>` description to reflect multi-syntax support and the new schema shape.
- [x] 7.2 Regenerate CLI reference docs (`src/bin/gen_docs.rs` output) if the `input` arg's removed default value changes rendered help text.

## 8. Tests

- [x] 8.1 Unit test: `.toml`, `.json`, `.jsonc`, `.json5` configs with equivalent content parse to the same `TransformConfigFile`.
- [x] 8.2 Unit test: a `.json` config containing a comment or trailing comma fails to parse (strict JSON only).
- [x] 8.3 Unit test: an unrecognized config extension is parsed as `.json5`, and its parse error (not a generic "unrecognized extension" message) is surfaced on failure.
- [x] 8.4 Unit test: an old-flat-shape config under each syntax fails with an "unknown field" error.
- [x] 8.5 Unit test: config `inputs` + CLI positional inputs both set is a usage error; config `inputs` alone is used when CLI positionals are empty; CLI positionals alone are used when config `inputs` is empty; both empty defaults to stdin.
- [x] 8.6 Unit test: CLI `--filter` appends after a config's `pipeline` steps, not replacing them.
- [x] 8.7 Unit test: a CLI `-o`/`--to` flag replaces a config's `outputs` entirely.
- [x] 8.8 Unit test: an output entry's `entities` allow-list, `drop` deny-list, and combined allow-then-deny behavior, each verified against a multi-entity-kind input document.
- [x] 8.9 Unit test: an unrecognized entity-kind token in `entities` or `drop` is rejected at parse time.
- [x] 8.10 Unit test: an output entry's `strict: true` fails on a lossy conversion while a sibling output without `strict` only warns, in the same invocation; and, separately, `strict: false` relaxes below an invocation-wide `--strict` while a sibling output without it still fails.
- [x] 8.11 Unit test: an unrecognized `scope` token is rejected at parse time; an omitted `scope` defaults to `"project"`.
- [x] 8.12 Integration test: `--to transform-config --out <path>` against a fixture repo with `.claude/`, `.cursor/rules/*.mdc`, `.cursor/mcp.json`, and `AGENTS.md` produces `cursor-mdc` and `cursor-mcp` as distinct entries (not one misclassified as the other), plus `claude` and `codex`.
- [x] 8.13 Integration test: a fixture repo with a bare top-level `CLAUDE.md` (no `.claude/` directory among the inputs) scaffolds a `claude` output entry.
- [x] 8.14 Integration test: a fixture repo with both a top-level `AGENTS.md` and a nested `src/backend/AGENTS.md` scaffolds exactly one `codex` entry (not two), and `--config <generated> --check` on the unmodified repo still reports every target unchanged.
- [x] 8.15 Integration test: an unmatched input path produces a warning but is still listed in the generated `inputs`; a single repo-root input (e.g. `.`) is not recursively expanded and produces zero `outputs` entries.
- [x] 8.16 Integration test (round-trip acceptance criterion from proposal/design): scaffolding a config from an unmodified fixture repo, then running `rulette transform --config <generated> --check`, exits 0 with every target reported unchanged.
- [x] 8.17 Integration test: `rulette inspect <input> --to transform-config` exits non-zero with a usage error.
