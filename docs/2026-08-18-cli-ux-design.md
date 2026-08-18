# CLI and UX design: transform as the single lever

2026-08-18

## Framing: one mental model, four new dimensions

The whole tool is one sentence:

> holistically ingest a set of things, and "compile"/transform them to different things — filter to a target, then output.

That is the existing `transform` verb (PRD "transform — the workhorse"), not a new idea.
This document does not add verbs.
It applies the ingest → filter/transform → output model along four dimensions that are currently underspecified:

1. **Declarative transform-config** — the invocation itself becomes an artifact you can pass in (`--config`) or scaffold out (`--to transform-config`).
2. **Output destination scope** — a second axis orthogonal to target *format*: project, user, enterprise, and local-override tiers.
3. **Per-target override blocks** — one canonical rule carries per-tool refinements through the *typed* model, not an ad hoc namespace.
4. **Deeper entity primitives** — hooks, MCP tool-filtering, and permission precedence generalized past Claude-only shapes.

Everything below is `transform` (and its dry-run sibling `inspect`) applied to those dimensions.
No `init`, no local state, no config-file discovery — every new capability is an explicit input or an explicit output of the one command, consistent with the hard constraints in `AGENTS.md`.

### Scope of tools

Core supported tools are **Codex, OpenCode, Claude, Cursor, and Antigravity**.
Every other harness (Windsurf, Copilot, Gemini CLI, Cline, and so on) is deferred until the domain model is solid across these five.
Gemini CLI is not on the roadmap; Antigravity replaces it.
Of the five, only Codex, Claude, and Cursor have any implementation today (`src/parsers/`, `src/emitters/`); OpenCode and Antigravity are net new.

Target tokens used throughout this document: `codex`, `opencode`, `claude`, `cursor-mdc` / `cursor-mcp`, `antigravity`.
These follow the existing kebab-case `OutputFormat` convention.

### Explicitly out of scope for this document

- **Network and source retrieval** (git/http fetch, auth, proxy, lockfiles) — tracked as `rulette-oe6`, deferred to its own epic.
  Every example here reads from the local filesystem, stdin, directories, or tar archives only.
- **Runtime resolution of what an emitted MCP entry does.**
  When Rulette emits `command: npx, args: [...]`, its determinism guarantee ends at the bytes it writes.
  What a downstream `npx` invocation resolves to at the target tool's runtime is the target tool's problem, not Rulette's.
  This is a stated boundary, not an omission: Rulette compiles configuration, it does not execute or pin it.

---

## 1. Declarative transform-config

### The problem it solves

A repo that fans out to five tools accumulates a long `transform` invocation with many `-o` targets, filters, and per-target settings.
Rerunning that by hand — or worse, reconstructing it from shell history in CI — is the exact "state to be synced" failure mode Rulette rejects.
The competing tool `rulesync` checks in a `rulesync.jsonc` for this.
Rulette gets the same ergonomics without violating "no config files, no init phase" by treating the config as **just another input artifact you pass explicitly**, never as tool state that is discovered or initialized.

Two rules keep this honest:

- The config is **never auto-discovered.**
  There is no `rulette.transform.jsonc` that Rulette picks up implicitly from the working directory.
  It is loaded only when you write `--config <path>`, exactly like passing a rule file as input.
- The config **produces no state.**
  It is a serialized `transform` invocation, deterministic in and deterministic out.

### Format: one schema, several surface syntaxes

`--config <path>` is already documented as TOML (PRD options list).
Detect the concrete syntax by extension and accept all of `.toml`, `.json`, `.jsonc`, `.json5` against one internal schema — the same posture the IR already takes by accepting both `*.rulette.json` and `*.rulette.toml`.

**Decision: `.jsonc` is the canonical form** — documented by example and the syntax every example in this document uses.
Comments and trailing commas matter for a human-edited fan-out manifest, and it matches the mental model of engineers migrating from `rulesync.jsonc`.
`.toml`, `.json`, and `.json5` stay accepted (TOML is already documented) but are not featured in docs or examples, so there is one blessed form without breaking the others.

### File shape

The config is a literal transcription of a `transform` invocation: inputs, an ordered pipeline, and outputs.

```jsonc
// rulette.transform.jsonc — an explicit input artifact, never auto-loaded.
{
  "inputs": ["./rules/"],

  // Applied in array order, identical semantics to the CLI flags.
  "pipeline": [
    { "filter": "status == \"stable\"" },
    { "rename": "org:name=internal:name" }
  ],

  "outputs": [
    { "target": "codex",       "scope": "project", "path": "AGENTS.md" },
    { "target": "opencode",    "scope": "project", "path": ".opencode/" },
    { "target": "claude",      "scope": "project", "path": ".claude/" },
    { "target": "cursor-mdc",  "scope": "project", "path": ".cursor/rules/" },
    { "target": "antigravity", "scope": "project", "path": ".antigravity/" }
  ]
}
```

### Per-target feature granularity

Feature toggles live on each output entry and support both coarse and fine control, so the common case stays short and the exceptional case stays expressible.

```jsonc
{
  "outputs": [
    // Coarse: this target only receives rules and skills; everything else is excluded up front.
    { "target": "cursor-mdc", "scope": "project", "path": ".cursor/rules/",
      "entities": ["rule", "skill"] },

    // Fine: take everything except hooks, and treat any remaining loss as a hard failure.
    { "target": "claude", "scope": "project", "path": ".claude/",
      "drop": ["hook"],
      "strict": true },

    // Default: no entity list means "everything this target can represent".
    { "target": "codex", "scope": "project", "path": "AGENTS.md" }
  ]
}
```

- `entities` is an allow-list (coarse include).
- `drop` is a deny-list (fine exclude), applied after `entities`.
- `strict` per-target escalates that target's lossy conversions to errors, mirroring the global `--strict` but scoped to one output.
- Omitting both `entities` and `drop` means "emit everything this target's emitter can represent" — the coverage report (§3, §5) tells you what that is.

### `--config` as input

```sh
# Run the checked-in fan-out. Equivalent to typing the whole -o list by hand.
rulette transform --config rulette.transform.jsonc

# CLI flags compose with and override the config, so CI can add --check without editing the file.
rulette transform --config rulette.transform.jsonc --check
# Expected output: per-target created/updated/unchanged lines; non-zero exit if any drift.
```

Precedence, documented and predictable: **explicit CLI flag > config-file entry > built-in default.**

**Decision: `--config` plus positional inputs is a usage error when both set `inputs`.**
Two sources of truth for `inputs` is a footgun; failing fast beats a silent-override surprise.
The one composable case: if the config omits `inputs`, CLI positionals fill it — so a config can define a reusable pipeline-and-outputs template applied to different input sets on the command line.

### `--to transform-config` as output: scaffold from an existing repo

Onboarding a repo that already has hand-written per-tool files does not need an `import` or `init` verb.
It is `transform` targeting a new output format: read the tools on disk, emit the config that reproduces them.

```sh
# Read the existing hand-written configs and scaffold a declarative manifest from them.
rulette transform .claude/ .cursor/ AGENTS.md \
  --to transform-config \
  --out rulette.transform.jsonc
# Expected output: rulette.transform.jsonc whose `outputs` mirror the tools found on disk,
# with `inputs` seeded to the same set so a re-run is an identity round-trip.
```

The scaffold is a starting point, stated honestly, not magic consolidation:

```jsonc
// rulette.transform.jsonc  (generated by --to transform-config; edit `inputs` to consolidate)
{
  // TODO: point this at one canonical source, e.g. ["./rules/"], then re-run to verify no drift.
  "inputs": [".claude/", ".cursor/", "AGENTS.md"],
  "pipeline": [],
  "outputs": [
    { "target": "claude",     "scope": "project", "path": ".claude/" },
    { "target": "cursor-mdc", "scope": "project", "path": ".cursor/rules/" },
    { "target": "codex",      "scope": "project", "path": "AGENTS.md" }
  ]
}
```

The generated config re-emits the same bytes it was scaffolded from, so `rulette transform --config … --check` on a freshly scaffolded repo exits 0.
That round-trip is the stress test for fidelity — and the audit gaps in §4 are exactly where it fails today.

---

## 2. Output destination scope

### The second axis

Target *format* answers "which tool's shape?".
Target *scope* answers "which tier of that tool's config tree?".
The same Claude rule can be written to four different places with four different precedence and sharing semantics:

| Scope token | Where it lives | Sharing / precedence | Example (Claude) |
| --- | --- | --- | --- |
| `project` | repo-relative, checked in | shared with the team; today's only mode | `.claude/` |
| `user` | `~`-rooted, per-person | personal across all your projects | `~/.claude/` |
| `enterprise` | org/IT-managed root | managed tier, typically higher precedence and read-only to the user | managed policy path |
| `local` | repo-relative, gitignored | per-user override layered on top of `project` | `.claude/settings.local.json` |

`project` is the default when scope is omitted, which keeps every existing invocation in the PRD unchanged.

### Per-tool tier support varies

Not every tool supports all four tiers.
This is a capability question with the same three-way answer as format coverage, so it reuses the coverage vocabulary (Supported / Lossy / Dropped) rather than inventing a parallel one.

| Tool | project | user | enterprise | local |
| --- | --- | --- | --- | --- |
| Claude | Supported | Supported | Supported | Supported |
| Cursor | Supported | Supported | Dropped | Dropped |
| Codex | Supported | Supported | Dropped | Dropped |
| OpenCode | Supported | Supported | Dropped | Dropped |
| Antigravity | Supported | Supported | Dropped | Dropped |

(Illustrative, pending per-tool confirmation — the point is the matrix exists and varies, not these exact cells.)

When a requested scope is not representable for a target, that is a **Dropped** result, reported the same way a dropped entity kind is (§5).
Under `--strict` it fails the build; without it, it warns and the emission omits that scope.

> **Not the same as `rulette:directory-scope`.**
> Codex's `rulette:directory-scope` (a path *within* the repo, e.g. `src/backend/AGENTS.md`) is a within-project placement concern and is unchanged by this section.
> The four tiers above are about *which config tree* (repo vs. `~` vs. managed vs. gitignored overlay), an orthogonal axis.
> A rule can be both `scope=project` and `rulette:directory-scope=src/backend`.

### CLI syntax: scope qualifies the target token

**Decision: scope qualifies the target token as `format@scope`.**
It composes with today's `-o format:path`, keeps the fan-out per-target, and `@scope` is optional (defaults to `project`), so every existing invocation is unchanged.
The two alternatives below are rejected, not open.

```text
-o <format>[@<scope>]:<path>
```

```sh
# Team-shared project config plus a personal user-level overlay, in one invocation.
rulette transform ./rules/ \
  -o claude@project:.claude/ \
  -o claude@user:~/.claude/

# A gitignored local override for one machine, layered on the project config.
rulette transform ./local-overrides/ \
  -o claude@local:.claude/settings.local.json

# Requesting an unsupported tier surfaces as a Dropped result, not a crash.
rulette transform ./rules/ -o cursor-mdc@enterprise:/etc/cursor/
# Expected output (without --strict):
#   warning: scope 'enterprise' is not representable for target 'cursor-mdc' (dropped)
#   … emits nothing for that target; exit 0
# With --strict: same message, exit non-zero.
```

Rejected alternatives, for the record:

- **A separate `--scope` flag.**
  Rejected: a single invocation fans out to multiple scopes (project + user above), so scope must attach per-`-o`, not once globally.
- **Pure path inference** (`~/…` ⇒ user, else project).
  Rejected as the sole mechanism: it cannot distinguish `enterprise` from `local`, both of which can be absolute or repo-relative depending on the tool.
  Inference is fine as a *default* when `@scope` is omitted; it is not sufficient as the only input.

The same `@scope` grammar is available in the declarative config's `"scope"` field (§1).

### Scope in inspect and coverage

`inspect` extends to report scope capability alongside format capability, reusing the coverage matrix rather than a new report.

```sh
# Which (target, scope) combinations are representable for this input?
rulette inspect ./rules/ --coverage
# Expected output (excerpt): a matrix whose columns are target@scope, cells Supported/Lossy/Dropped.

# CI gate: fail if any requested destination can't be represented.
rulette inspect --config rulette.transform.jsonc --coverage --strict
```

This is a natural extension of the shipped `coverage-reporting` design: today a matrix cell is `(target, entity_kind) → status`; scope adds `(target@scope, entity_kind) → status` using the identical Supported/Lossy/Dropped classification and worst-case-wins aggregation.

---

## 3. Per-target override blocks

### The problem

`rulesync` lets one canonical rule file carry per-tool sub-blocks (`cursor: { alwaysApply: true }`, `antigravity: { trigger: "model_decision" }`) alongside shared fields.
Rulette today has only a flat, untyped `extra` passthrough plus a handful of typed `rulette:*` keys.
Two concrete cases in hand argue for extending the *typed* model rather than adding another ad hoc namespace:

- Cursor's `alwaysApply` / `globs` currently land in untyped `extra` and are silently dropped when converting to any other target (audit gap, §4).
- Antigravity's real trigger model is `trigger: always_on | glob | manual | model_decision`, which maps cleanly onto the existing `ActivationMode` enum (`Always, Glob, Pattern, Manual, Model` in `src/ir/mod.rs`).

Both are activation semantics that *differ per target*.
That is a typed concept, not a passthrough blob.

### Shape: make `rulette:activation` per-target-aware

Extend the typed `Activation` model with an optional per-target override layer.
The existing single-value `Activation` remains the resolved shape an emitter consumes; a new wrapper carries the default plus per-target refinements.

```yaml
---
description: TypeScript conventions
rulette:activation:
  default:
    mode: [glob]
    globs: ["**/*.ts", "**/*.tsx"]
  overrides:
    cursor:
      mode: [always]          # Cursor alwaysApply: true
    antigravity:
      mode: [model]           # Antigravity trigger: model_decision
      description: "Apply when editing or reviewing TypeScript"
---
Prefer `unknown` over `any`. Enable `strict` in tsconfig.
```

**Decision: resolution is full replacement, not deep merge.**

> For target `T`, an emitter uses `overrides[T]` if present, else `default`.
> There is no field-level merge between `default` and an override — an override for a target fully replaces the default for that target.

Full replacement is chosen for predictability: a reader of the frontmatter can see exactly what Cursor gets without mentally merging two maps.

### Mapping table (the two concrete cases)

| Canonical `mode` (+fields) | Cursor emission | Antigravity emission |
| --- | --- | --- |
| `[always]` | `alwaysApply: true` | `trigger: always_on` |
| `[glob]` + `globs` | `globs: [...]`, `alwaysApply: false` | `trigger: glob`, `globs: [...]` |
| `[manual]` | manual (no auto-apply) | `trigger: manual` |
| `[model]` + `description` | description-matched | `trigger: model_decision` |

Because the model is typed and shared, the same canonical file resolves correctly for every target at once:

```sh
rulette transform ./rules/typescript.md \
  -o cursor-mdc:.cursor/rules/ \
  -o antigravity:.antigravity/
# Expected output:
#   .cursor/rules/typescript.mdc      → alwaysApply: true
#   .antigravity/typescript.md        → trigger: model_decision, description: "Apply when …"
```

### Where `extra` still lives

The typed override layer is for concepts that generalize across targets (activation is the archetype).
The `extra` passthrough remains for the genuinely tool-unique long tail that never maps to a shared enum.
Keep such keys namespaced by tool so they are self-describing and never collide:

```yaml
rulette:activation:
  default: { mode: [glob], globs: ["**/*.ts"] }
extra:
  antigravity:someExperimentalField: "value that has no cross-tool meaning"
```

Rule of thumb, and the design boundary:

- If a field has a cross-target meaning (an activation trigger, a tool-access rule), it belongs in the **typed** model with per-target overrides.
- If a field is meaningful to exactly one tool and will never generalize, it belongs in **`extra`**, tool-prefixed.
- The IR never drops what it does not understand: unknown `extra` keys pass through untouched, as today.

**Decision: the `default`/`overrides` wrapper is applied to `rulette:activation` only, for now (YAGNI).**
Activation is the only typed key with concrete per-target demand today; building the wrapper generically for keys with no demand would be speculative.
The wrapper type is not specific to activation's fields — it is a parametric `{ default: T, overrides: map<target, T> }` — so `rulette:tool-access` and `rulette:hook-event` can adopt the identical shape later, when a second key shows real per-target divergence, **without a breaking change** to the activation form defined here.

---

## 4. How the audit-flagged gaps resolve

A per-variant audit found concrete fidelity gaps in the three implemented tools.
Here is where each one lands relative to the primitives above — some fall out for free, some need their own targeted fix, and one needs a genuinely new primitive.

| Gap | Resolution path |
| --- | --- |
| Cursor `globs`/`alwaysApply` stuck in untyped `extra`, dropped on conversion | **Fixed by §3.** The Cursor parser promotes them into the typed `rulette:activation` model; from there they resolve to every other target. This is the primary motivation for §3. |
| Claude skill emission drops all frontmatter silently, no lossy warning even under `--strict` | **Fixed by coverage-reporting + a correctness fix.** The Claude skill emitter must (a) preserve frontmatter so the PRD's "nothing lost round-tripping Claude" claim holds, and (b) if any field genuinely can't be carried, classify it `Lossy` with a `reason` via `capabilities()`, so `--strict` catches it. The silent-drop-with-no-warning state is a bug the structured capability path is designed to make impossible. |
| Codex `rulette:directory-scope` is write-only (emitter sets it, parser never infers it from a nested `AGENTS.md` tree) | **Targeted parser fix, flagged separately.** The Codex parser must infer `rulette:directory-scope` from a real nested-directory `AGENTS.md` layout, so the PRD's flagship "Codex Scoping" example is derivable from parsing real input, not only settable via `--set`. Not solved by any of the four dimensions; it is a parser round-trip fix. |
| No emitter writes a file literally named `SKILL.md` (Claude writes `<name>.md`, Agent Skills writes `<name>.skill.md`) | **Targeted emitter fix, folded into §2's path work.** The agent-skills target must write `SKILL.md` (directory form `<name>/SKILL.md`) to preserve the format's namesake round-trip identity. §2 already touches emitter destination-path logic, so this rides along, but it is a distinct correctness fix. |
| Agent Skills companion files under `scripts/*` (non-`.md`/`.json`/`.toml`/`.yaml`) silently skipped; no binary-safe passthrough anywhere; IR assumes UTF-8 `body: String` | **Needs a new primitive — not covered by the four dimensions.** See below. |

### The one gap that needs its own primitive: companion files

A skill with a `scripts/` or assets directory carries files that are not text rules — shell scripts, binaries, images.
The IR's text entities assume UTF-8 `body: String`, so today those files are dropped on a directory walk: real data loss.
None of the four dimensions above fix this, and it should not be forced into one of them.

Sketch (for a separate design, flagged here so it is not silent):

- A skill entity gains a typed companion-files collection: `{ path, content }` where `content` is either UTF-8 text or base64-encoded bytes with an explicit encoding tag.
- The directory walk stops filtering by extension for files inside a skill directory and instead captures everything under it, tagging binary content as base64.
- Emitters that support companion files (agent-skills, Claude skills) write them back verbatim; emitters that don't report them `Dropped` via `capabilities()`.

This is called out as its own follow-up, not designed in depth here, because it changes the core IR text assumption and deserves its own adversarial review.

---

## 5. Worked end-to-end journeys

### Journey 1 — author once, fan out to all five tools

```sh
# One canonical source, five targets, one invocation.
rulette transform ./rules/ \
  -o codex:AGENTS.md \
  -o opencode:.opencode/ \
  -o claude:.claude/ \
  -o cursor-mdc:.cursor/rules/ \
  -o antigravity:.antigravity/
# Expected output: five created/updated/unchanged lines; all-or-nothing (no partial writes on failure).
```

Or the same thing as a checked-in manifest:

```sh
rulette transform --config rulette.transform.jsonc
```

### Journey 2 — CI enforcement (kept intentionally light)

Two independent gates, both already grounded in shipped/in-flight behavior:

```sh
# Drift gate: fail if any target on disk is out of date. (--check is shipped.)
rulette transform --config rulette.transform.jsonc --check

# Capability gate: fail if anything canonical can't be represented at a requested target/scope.
rulette inspect --config rulette.transform.jsonc --coverage --strict
```

The exact division of labor between these two gates (drift vs. coverage, and whether one implies the other in CI) is still fuzzy and left open in §6.

### Journey 3 — consolidate hand-written per-tool config into one canonical source

```sh
# 1. Scaffold a manifest from whatever exists on disk today.
rulette transform .claude/ .cursor/ AGENTS.md \
  --to transform-config --out rulette.transform.jsonc

# 2. Move the shared substance into ./rules/, then point the manifest's `inputs` there.

# 3. Verify the consolidation is lossless: re-emitting from ./rules/ reproduces the original files.
rulette transform --config rulette.transform.jsonc --check
# Expected output: all targets `unchanged`, exit 0  → consolidation is faithful.
# Any `updated`/`created` line is a round-trip fidelity gap — exactly the §4 audit cases.
```

This journey is the fidelity stress test; a clean `--check` here is the acceptance criterion for the §4 fixes.

### Journey 4 — per-target enrichment of one canonical rule

`./rules/typescript.md` with the override block from §3:

```sh
rulette transform ./rules/typescript.md \
  -o cursor-mdc:.cursor/rules/ \
  -o antigravity:.antigravity/ \
  -o claude:.claude/
# Expected output:
#   .cursor/rules/typescript.mdc  → alwaysApply: true            (cursor override)
#   .antigravity/typescript.md    → trigger: model_decision      (antigravity override)
#   .claude/…/typescript.md       → default glob activation      (no override → default)
```

One source, three tools, three correct activation shapes, from the typed model — no untyped `extra` drop.

### Journey 5 — debug and inspect before committing to CI

```sh
# What does Cursor actually receive, and what gets dropped?
rulette inspect ./rules/ --to cursor-mdc
# Expected output: surviving fields listed; dropped/lossy fields warned with reasons.

# Full matrix across every target and scope at once.
rulette inspect ./rules/ --coverage
# Expected output: entity-kind × target@scope matrix of Supported/Lossy/Dropped.

# Machine-readable, for a script or a PR comment.
rulette inspect ./rules/ --coverage --json | jq '.[] | select(.status == "dropped")'
```

`--coverage` and `--to` remain mutually exclusive (single-target detail vs. all-targets summary), per the shipped coverage design.

### Journey 6 — multi-entity governance across all five surfaces

Text rules are the easy case; the durable value is MCP servers, hooks, and permissions staying consistent across tools that model them differently.
This is where the three cross-cutting IR gaps (below) get exercised.

```sh
# Govern MCP servers, hooks, and permissions from one canonical source, fanned to five tools.
rulette transform ./governance/ \
  -o codex:AGENTS.md \
  -o opencode:.opencode/ \
  -o claude:.claude/ \
  -o cursor-mcp:.cursor/mcp.json \
  -o antigravity:.antigravity/
# Expected output: each target receives the entity kinds it supports; the rest report Dropped
#   in `inspect --coverage`, not silently vanish.
```

The three primitive gaps this journey depends on, each needing design beyond this document:

- **Hook taxonomy is Claude-only.**
  `HookEventKind` is hardcoded to Claude's five events (`PreToolUse, PostToolUse, Notification, Stop, SubagentStop`) with no cross-tool translation.
  A general hook model must express "this tool has a narrower hook surface, or none," and map or drop per target — reported through `capabilities()`.
- **MCP servers have no tool-filtering concept.**
  `McpServerConfig` is only `command`/`args`/`env`.
  `rulesync` models `enabledTools`/`disabledTools` per server; the IR needs an allow/deny concept on `mcp-server` to govern which tools a server exposes, or that governance is silently lost.
- **Permissions have no precedence model.**
  `ToolAccessRule` defines no conflict resolution when multiple rules could apply to one tool.
  Multi-entity governance across tools with different precedence semantics needs a defined ordering (most-specific-wins, deny-overrides-allow, or explicit order) before it can be deterministic.

These three are named here as the concrete blockers for Journey 6, to be designed separately; this document establishes that they route through the existing `transform`/entity model, not new verbs.

---

## 6. What this document does not decide yet

The pure engineering-judgment calls within this design have been resolved in place as settled decisions in §1, §2, and §3 (canonical config syntax, `--config`/positional precedence, `format@scope` grammar, override full-replacement, and activation-only override wrapper).
What remains open genuinely needs input this document cannot supply — empirical tool verification, a product-priority call the maintainer flagged as fuzzy, or a separate design effort with its own review:

1. **Per-tool scope support matrix.**
   The §2 table is illustrative.
   The real Supported/Dropped cells for user/enterprise/local across Codex, OpenCode, Cursor, and Antigravity need empirical per-tool verification, not a document decision.

2. **CI gate division of labor (Journey 2).**
   Drift-check (`--check`) and coverage-gate (`--coverage --strict`) are two separate gates today.
   Is that the intended split, and does either imply the other in a standard CI setup? (Maintainer explicitly flagged this journey as still fuzzy — not forced here.)

3. **Companion-files primitive (§4).**
   The base64/text-tagged companion-file model is a sketch.
   It changes the core "IR bodies are UTF-8 text" assumption and needs its own design and adversarial review before commitment.

4. **The three multi-entity primitive gaps (Journey 6).**
   Generalized hook taxonomy, MCP tool-filtering, and permission precedence each need their own design.
   This document commits only that they extend the entity model and route through `transform`, not that any particular shape is chosen.
