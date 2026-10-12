# Rulette and Rulesync

Rulesync v29.0.0 manages unified guidance sources and generates configuration for many agent tools.
Rulette v0.1 compiles explicit local rules and skills into reviewed publication plans for five harness domains.
This review supplies candidate responses for `rulette-l8r.3`; it approves no product or implementation changes.

## Evidence boundary

The review pins [Rulesync v29.0.0][release], published October 8, 2026, to commit `4efba24d515ff2214214d8b1c1616b89178fae7d`.
Official npm `latest` metadata, the GitHub tag reference, release metadata, and the tagged source archive were retrieved on October 10, 2026, inside the existing `rulette-l8r-skills` sandbox.
The [retrieval record](benchmarks/rulesync-29.0.0-sources.json) retains exact URLs, archive and document hashes, methods, and the observation boundary.
The retrieval environment reported Node.js `v22.22.1` and Python `3.14.4`; the npm package declares Node.js `>=22.0.0`.
The [release assets][release] also include Linux, macOS, and Windows standalone binaries and `SHA256SUMS`.
Standalone distribution does not establish that those binaries satisfy Rulette's fully static requirement; linkage was not inspected.

**Stated** findings below come from official documentation at the pinned commit.
**Source-reviewed** journeys describe that documentation and release changes, rather than execution of the competitor CLI.
**Observed** evidence is limited to source retrieval and the environment above.
No competitor lifecycle probes were needed to resolve a material uncertainty, and none were run.
The user's preference for official stated behavior narrows the issue's original broad experiment acceptance criteria.
The 40 Agent Skills experiments from `rulette-l8r.1` and the migration fixtures and tests from `rulette-um4` were not repeated.

Rulette findings use the [current PRD](2026-04-11-prd.md), [CLI reference](2026-04-11-man-page.md), and shipped source reviewed by the Rulette scout.
The [IR](../openspec/specs/ir-core/spec.md), [transform](../openspec/specs/transform-pipeline/spec.md), and [frontend/backend](../openspec/specs/frontends-and-backends/spec.md) specifications retain older model and command descriptions in places.
Where they diverge, this comparison reports shipped behavior and leaves reconciliation to synthesis.

## Configuration, state, and precedence

Rulesync discovers project-root `rulesync.jsonc`, not `.rulesync/rulesync.jsonc`, and supports an explicit `--config` path.
Its documented priority is CLI options, `rulesync.local.jsonc`, `rulesync.jsonc`, then defaults.
An XDG user configuration seeds new `init` projects only; `generate` does not read it.
Configuration has JSON Schema support and validated target/feature combinations, so describing all options as untyped is inaccurate.
Unknown top-level configuration keys can still be ignored by normal loading; `doctor` identifies them with suggestions.
Sources: [configuration][configuration], [CLI commands][commands].

Rulesync keeps authored `.rulesync/` sources, generated destinations, curated installed artifacts, source lockfiles, and optionally hooks ownership sidecars.
Its `convert` command offers a one-shot path without writing `.rulesync/` sources, so initialization is not a universal prerequisite.
Rulette has no initialization or ambient configuration discovery; its explicit `--config` manifest is an input artifact rather than persistent local state.
Sources: [Rulesync CLI][commands], [Rulesync configuration][configuration], [Rulette PRD](2026-04-11-prd.md).

Rulesync supports separate input and output roots for shared sources and monorepos.
Later input roots override earlier roots; within one root, local rules and skills override same-named curated artifacts.
Overlay identity comparison is case-insensitive and Unicode-normalized, with warnings for case-only conflicts.
The first input root must exist; later roots may be absent.
Some global tool-home environment variables override even explicit output roots; CI must control these too.
Sources: [input roots][roots], [declarative sources][sources], [CLI home overrides][commands].

## Source-reviewed user journeys

Command counts describe documented happy paths, excluding source editing and executable installation.
These are stated workflows, not newly observed CLI outcomes.

| Journey | Commands and setup | Authority, diagnostics, and recovery |
| --- | --- | --- |
| Initialize | `init`, then `generate`: two commands; onboarding also recommends `fetch` | Scaffolds source/configuration files without overwriting existing ones; XDG defaults seed configuration only. [CLI][commands]. |
| Adopt existing tool files | `import --targets <tool>`, then `generate`: two commands | Creates unified sources under `.rulesync/`; import support varies by feature/tool. [CLI][commands]. |
| Convert directly | `convert --from <tool> --to <tools>`: one command | Writes destinations without adopting `.rulesync/`; `--dry-run` previews without destination writes. [CLI][commands]. |
| Fetch once | `fetch <source>`: one command | Copies selected artifacts; default conflict policy is `overwrite`, with `skip` available. Remote selection and pruning safeguards have documented warnings. [CLI][commands]. |
| Install declared sources | Edit `sources`, then `install` and `generate`: two commands | Locks commits and artifact hashes; `install --frozen` rejects missing selection coverage and can fetch missing locked artifacts without updating the lock. [Sources][sources]. |
| Add one source | `add <source>`: one command after existing sources are installed | Preserves manifest comments; failure restores manifest, locks, and curated artifacts. [Sources][sources]. |
| Preview and verify | `generate --dry-run` or `generate --check`: one command | Neither writes generated files; check exits `1` on drift. Flags are mutually exclusive. [Dry run][dryrun]. |
| Regenerate continuously | `generate --watch`: one long-running command | Re-resolves configuration; errors keep watching alive, but changing watched roots requires restarting the command. Incompatible with dry-run, check, and JSON. [CLI][commands]. |
| Diagnose setup | `doctor`, optionally `--strict`: one command | Read-only severity diagnostics, typo suggestions, and JSON output; strict mode makes warnings fail CI. [CLI][commands]. |
| Global generation | `generate --global`: one command after source setup | Writes user-level configuration; availability varies by target and feature. Use the mode-specific [supported-tools table][tools]. |
| Retire target or feature | Edit configuration, preview, then retire: two commands plus edit | `--retire-targets` requires explicit `--features` without `*`; cannot combine with `--targets` or watch. Retirement is idempotent. [CLI][commands], [v29 release][release]. |

## Output ownership, collisions, and fidelity

Rulesync already provides preview and CI verification; the older claim that it offers no audit option was incorrect.
The v29 release changes JSON mutation plans to version 2, including managed shared-configuration key operations, and adds retirement of a feature while its target remains configured.
Omitting a target never retires its files, even with `--delete`; explicit retirement can delete a managed path edited by hand.
Shared settings files and paths still written by configured targets are retained.
Sources: [CLI commands][commands], [v29 release][release].

The JSON mutation plan orders operations deterministically, but paths omit output-root identity and collapse matching paths across multiple roots.
Shared keys owned at deeper levels may appear only as whole-file writes; two documented cleanup rewrites are not listed as operations.
Two features updating the same shared key can report different dry-run and real-run operations because dry-run does not publish intermediate writes.
This is a useful preview contract, with documented limits for a consumer authorizing mutations from the plan.
Source: [CLI JSON mutation plan][commands].

Ownership is format-specific rather than one universal merge policy.
Rulesync preserves unrelated shared-settings keys and edits selected JSONC files to retain comments where safe; fallback rewrites warn, and some unparsable shared-settings files fail rather than being overwritten.
Other formats can still lose comments, including Codex TOML.
`preserveUnownedHooks` is opt-in: default generation replaces destination hook lists; opting in uses `.rulesync-hooks-lock.json` sidecars to retract owned handlers while retaining others.
Sources: [shared configuration][commands], [hooks ownership][configuration].

When targets share a root output path, the documented policy is last target wins, with warnings for folded-rule loss.
Narrowed `--check` still compares a shared root file against its configured final owner, which can differ from the selected check target.
Compatible root outputs compose; unsafe source-root collisions fail, while other modular collisions can warn and retain the last write.
Flattened command naming warns about collisions; path-based names reduce collisions without eliminating them.
Sources: [configuration][configuration], [root composition][global].

Rulette checks collisions before lowering and uses deterministic selection, staged plan digests, explicit publication-root authorization, and replacement checks.
Apply skips identical content and requires `--replace` for conflicting existing content.
Strict loss handling applies to reported capability findings; it does not prove every field loss is detected.
Current Codex, OpenCode, and Claude rule lowering emits body bytes without findings for every omitted activation or frontmatter field.
Sources: [PRD](2026-04-11-prd.md), [CLI reference](2026-04-11-man-page.md), [lowering](../src/emitters/lowering.rs).

## Capability and trust boundaries

Rulesync's summary checkmarks mean support in at least one mode, not native support in every scope.
The [supported-tools reference][tools] distinguishes project, global, and simulated support; [simulation][simulation] provides prompt-driven commands, subagents, and skills rather than proving native agent execution.
Plugin packaging targets are project-only and excluded from `--targets "*"`.
Cursor rules are project-only, Codex CLI commands are global-only, and v29 Zed permissions are global-only.
The simulation guide documents `s/<command>` prompts and subagent/skill instructions; those are separate from native support.
The deprecated `kiro` alias remains alongside `kiro-cli` and `kiro-ide`; `continue` is a frozen-compatibility target, and `roo` retains compatibility after its upstream end of life.
The ignore feature is deprecated, and npm transport is experimental.
The global-mode tutorial retains older narrow support claims; use the current mode-specific table and release notes for availability.
Sources: [pinned README][readme], [tools][tools], [sources][sources], [global guide][global].

Rulesync documents warnings and diagnostics for ignored configuration, collisions, and some fidelity losses; a blanket claim of silent representation loss was unsupported.
These diagnostics do not establish a uniform strict semantic-loss contract for every mapping.
Unreadable single-file feature inputs fail nonzero while other features still execute, and their orphan sweeps are skipped.
Malformed subagents/checks warn and skip; invalid rule/command/skill frontmatter aborts generation.
This allows partial publication before a reported failure, so recovery depends on the affected feature and a corrected rerun.
Source: [CLI unreadable-source behavior][commands].

Rulette retains unsupported native semantics as opaque packages and checks supported/lossy/dropped findings for selected targets.
Portable v0.1 semantics remain rules and skills only; agents, hooks, MCP, permissions, and plugin behavior are outside that contract.
Sources: [PRD](2026-04-11-prd.md), [lowering](../src/emitters/lowering.rs).

Rulesync trusts chosen source trees and follows ordinary source symlinks beyond their roots; skill companion handling adds narrower exclusions and warnings.
Output writes and sweeps refuse links escaping the output root.
Experimental npm transport verifies registry integrity, rejects extraction traversal, skips links/devices, and caps extraction at 10,000 files and 100 MB.
These safeguards do not certify downloaded instructions, scripts, hooks, MCP settings, or permission policies as safe for an agent to consume.
Rulette's offline inputs and opaque resources likewise do not certify guidance content, although Rulette does not execute or fetch it.
Sources: [symlink trust][roots], [output boundaries][commands], [npm transport][sources], [Rulette PRD](2026-04-11-prd.md).

Rulesync's locks and drift checks support reproducible CI when versions, CLI options, local overrides, roots, target order, installed inputs, and tool-home variables are controlled.
`--frozen` can still require network access for missing locked artifacts.
Documented rollback for `add` does not establish a transaction for every generated destination or digest-bound replay of a reviewed plan.
Rulette's explicit local snapshot and separate digest-authorized apply provide the relevant comparison, without implying universal rollback for Rulette.
Sources: [configuration][configuration], [sources][sources], [PRD](2026-04-11-prd.md).

## Delta candidates for synthesis

These classifications are research candidates, not approved roadmap decisions.
`rulette-l8r.1` and `rulette-l8r.2` precede `rulette-l8r.3`, which feeds `rulette-ka2`; the `rulette-onj` human gate remains open.

| Material delta | Candidate classification | Evidence and synthesis question |
| --- | --- | --- |
| Preview and CI drift checks | Match | Compare diagnostic usability with Rulette stage/check rather than claiming Rulesync lacks these features. [Dry run][dryrun]. |
| Reviewed publication authority | Beat | Rulette separates staging from digest/root-authorized apply; assess that UX against Rulesync's useful but limited mutation preview. [CLI][commands], [PRD](2026-04-11-prd.md). |
| Ambient state and network-dependent workflows | Deliberate non-goal | Ambient overrides, fetching, locks, and watchers exceed Rulette's state-free runtime contract; Rulesync static linkage remains unverified. [Sources][sources], [PRD](2026-04-11-prd.md). |
| Remote install and updates | Deliberate non-goal | External producers can supply explicit local inputs without an internal fetch lifecycle. [Sources][sources], [PRD](2026-04-11-prd.md). |
| Doctor and actionable JSON diagnostics | Unresolved design question | Assess whether Rulette findings make configuration and capability failures equally discoverable. [CLI][commands]. |
| Retirement and shared-file ownership | Unresolved design question | Rulesync has selective retirement and key operations; Rulette's derived-output lifecycle does not merge shared native configuration. [Release][release], [PRD](2026-04-11-prd.md). |
| Collision and precedence explanations | Unresolved design question | Compare hard errors and provenance with documented ordering and collision warnings. [Configuration][configuration], [roots][roots]. |
| Native versus simulated capability display | Match | Keep modes explicit; broad Rulesync checkmarks need mode-specific interpretation. [Tools][tools]. |
| Semantic-loss visibility | Unresolved design question | Strict reported findings help, but field-fidelity gaps prevent universal superiority claims. [Lowering](../src/emitters/lowering.rs). |
| Monorepo roots and direct conversion | Unresolved design question | Compare explicit multi-root UX and scoped publication without adding hidden state. [Roots][roots], [CLI][commands]. |
| Broad native features and simulation | Deliberate non-goal | Agents, hooks, permissions, MCP, plugins, and simulated execution exceed portable v0.1 rules/skills. [Tools][tools], [PRD](2026-04-11-prd.md). |
| Specification alignment | Unresolved design question | Older specs describe `RuletteDocument`, mutation flags, and direct outputs absent from the current CLI; reconcile through approved follow-up work. [Specs](../openspec/specs/transform-pipeline/spec.md), [CLI](2026-04-11-man-page.md). |

## Reused migration evidence

Closed `rulette-um4` provides hand-translated examples and Rulette regression evidence, not current Rulesync execution or parity results.
The [existing migration tests](../tests/rulesync_migration_tests.rs) manually parse targets, construct typed activation overrides, and check selected emitted paths or body text from authored inputs.
They do not test a Rulesync configuration loader, run Rulesync, or prove complete activation/frontmatter fidelity across all five harnesses.
No fixture or test was added or rerun for this review.

The former migration recipe lacked an explicit native frontend and input layout; Rulette has no `--from rulesync` adapter.
Use documented native frontends or graph inputs with explicit targets and stage/review/apply authorization.
Current selection uses exact `--select` package identifiers; the older speculative filter, rename, set, arbitrary output, and `--to` pipeline is absent from the current CLI.
Plain native stdin is rejected; stdin requires explicitly selected archive or graph formats.
Sources: [PRD](2026-04-11-prd.md), [CLI reference](2026-04-11-man-page.md).

[release]: https://github.com/dyoshikawa/rulesync/releases/tag/v29.0.0
[readme]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/README.md
[configuration]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/guide/configuration.md
[commands]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/reference/cli-commands.md
[sources]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/guide/declarative-sources.md
[roots]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/guide/separate-input-root.md
[dryrun]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/guide/dry-run.md
[global]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/guide/global-mode.md
[tools]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/reference/supported-tools.md
[simulation]: https://github.com/dyoshikawa/rulesync/blob/4efba24d515ff2214214d8b1c1616b89178fae7d/docs/guide/simulated-features.md
