## Why

A repo that fans out rules to five tools accumulates a long `transform` invocation with many `-o` targets, filters, and per-target settings; reconstructing that by hand (or from shell history in CI) is the "state to be synced" failure mode Rulette otherwise rejects. `--config <path>` already exists (`src/cli/commands/transform.rs`) but only reads TOML into a flat, single-value `TransformConfig` (one `filter`, one `exclude`, one `rename`, one `set`, plus `to`/`out`) — it cannot express an ordered pipeline, has no `inputs` field, and has no reverse direction to scaffold a config from an existing repo. `docs/2026-08-18-cli-ux-design.md` §1 specifies the full shape this should take; this change closes the gap between that design and the shipped flag. Tracked as `rulette-5bk.6`.

## What Changes

- `--config <path>` accepts `.toml`, `.json`, `.jsonc`, and `.json5` by file extension, all parsed against one internal schema (was TOML-only). `.jsonc` is the canonical/documented form.
- **BREAKING**: the config schema changes shape. `filter`/`exclude`/`rename`/`set` (each a single optional string) are replaced by `pipeline`: an ordered array of single-key step objects (`{"filter": "..."}`, `{"exclude": "..."}`, `{"rename": "..."}`, `{"set": "..."}`), applied in array order. This is a breaking change to the config file format only — no existing TOML config in this repo's tests or docs uses the old flat shape as committed state (`--config` is never auto-discovered), so there is no migration path to design, only a documentation update.
- The config gains an `inputs: Vec<String>` field. Precedence: it is a usage error for both `--config`'s `inputs` and CLI positional inputs to be non-empty in the same invocation; if the config omits `inputs`, CLI positionals fill it.
- `to`/`out` are replaced by `outputs`: an array of `{target, scope?, path, entities?, drop?, strict?}` objects, replacing the flat `to: Option<OutputFormat>` / `out: Vec<String>` pair. `entities` is an allow-list, `drop` is a deny-list applied after `entities`, `strict` escalates that output's lossy conversions to errors. `scope` is accepted and defaults to `"project"`; only token-shape validation happens here — tier semantics (user/enterprise/local) are designed in `rulette-5bk.7` and are out of scope for this change.
- New scaffold direction: `--to transform-config --out <path>` infers each output target from the naming convention of the invocation's own input paths (content-based format detection cannot distinguish Claude/Codex/Copilot/Windsurf, which are textually near-identical plain markdown — see design.md), and writes a `.jsonc` (or matching extension) manifest whose `inputs` reproduce the given input paths and whose `outputs` reflect the inferred targets, so `rulette transform --config <generated> --check` on a freshly scaffolded repo exits 0. Only the five core tools this repo targets or has committed to (`codex`, `claude`, `cursor-mdc`, `cursor-mcp`, `opencode`/`antigravity` pending `rulette-5bk.9`/`.10`) are recognized; an input matching no known convention is kept in `inputs` but produces no `outputs` entry and a warning.
- CLI flags continue to compose with and override config-file values (unchanged precedence: explicit CLI flag > config-file entry > built-in default), now applied per-pipeline-step and per-output-field instead of per top-level flag.

## Capabilities

### New Capabilities

- `transform-config`: The declarative transform-config file — its multi-syntax parsing (`.toml`/`.json`/`.jsonc`/`.json5`), schema (`inputs`, `pipeline`, `outputs`), CLI/config precedence rules, and the `--to transform-config` scaffold-output direction.

### Modified Capabilities

- `transform-pipeline`: The `--config` flag's behavior changes from "load a flat TOML file that seeds `--filter`/`--exclude`/`--rename`/`--set`/`--to`/`--out`" to "load a multi-syntax `transform-config` document (see the new capability) whose `pipeline` and `outputs` compose with CLI flags under the same override precedence." The existing `--filter`/`--exclude`/`--rename`/`--set` CLI-flag requirements are unchanged; only their interaction with `--config` changes.
- `frontends-and-backends`: `inspect --to <format>`'s existing exhaustive target handling gains a new, explicit usage-error case for the new `transform-config` target, since `inspect` has no access to the original invocation's input paths that `transform --to transform-config` needs (see design.md Decision 7a).

## Impact

- `src/cli/commands/transform.rs`: `TransformArgs::execute` (including reordering config-load ahead of `read_inputs`, a real control-flow change, not a localized edit — see design.md Decision 3), the `TransformArgs::input` arg definition (drops its clap default), the `TransformConfig` struct (replaced), `OutputTarget` (gains `entities`/`drop`/`strict` fields), config-loading logic, and the emission loop (per-output `entities`/`drop`/`strict` filtering and the new `--to transform-config` output arm). `parse_targets` itself needs no changes — its existing bare-path fallback already produces the right `OutputTarget` shape for `--to transform-config --out <path>` (design.md Decision 7).
- `src/cli/formats.rs`: add an `OutputFormat::TransformConfig` (`transform-config`) variant.
- `src/cli/commands/inspect.rs`: its own exhaustive `match` over `OutputFormat` needs a new arm rejecting `inspect --to transform-config` as a usage error (design.md Decision 7a) — otherwise adding the variant above is a compile error there.
- `Cargo.toml`: new dependency, the `json5` crate (parsing only; see design.md Decision 2a for why scaffold output never depends on it for serialization).
- `docs/2026-04-11-prd.md:162`: update the `--config <path>` description from "TOML file" to reflect multi-syntax support.
- `openspec/specs/transform-pipeline/spec.md`: delta for `--config` requirement change.
- New `openspec/specs/transform-config/spec.md`.
- No change to the IR (`src/ir/`), to existing emitters' `emit()`/`capabilities()` behavior, or to non-`--config` CLI flags.
- Tracking: beads `rulette-5bk.6` (parent `rulette-5bk`).
