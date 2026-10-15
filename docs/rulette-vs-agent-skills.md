# Rulette and the skills CLI

The Agent Skills format defines portable directories centered on `SKILL.md`.
Vercel Labs' [`skills` CLI][upstream] discovers, installs, uses, updates, and removes those directories.
Rulette compiles explicitly supplied native guidance and reports translation loss before reviewed publication.
Directory installation and semantic conversion are different capabilities.

## Evidence boundary

This benchmark supplies `rulette-l8r.1` to the synthesis in `rulette-l8r.3`.
The evaluated npm release is [`skills@1.7.2`](https://www.npmjs.com/package/skills/v/1.7.2), with npm `gitHead` `671e8c320810d36fed80fac5f1a2c1bf7e82d812`, retrieved October 10, 2026.
The npm integrity is `sha512-j+GzyF/oNahsjjjup6y3lJpFkC1lQjIkf4kAMXICA1hboBqi0fAGXAGoE4x9FRZiP1br/YE9+L3/Tqq6PIAe0A==`.
Execution used the dedicated Docker Sandbox `rulette-l8r-skills`, Linux `7.0.14` on x86-64, Node `22.22.1`, and npm `9.2.0`.
Competitor installations used disposable `/tmp` projects and the sandbox's home; the host checkout received only research artifacts.
`DO_NOT_TRACK=1` disabled telemetry and security-audit retrieval, so a null audit result is not a safety verdict.

The [runner](benchmarks/skills-1.7.2.mjs) and [command evidence](benchmarks/skills-1.7.2-results.json) record arguments, exit status, timing, stdout, stderr, file hashes, permission bits, and symlink destinations.
Evidence identifiers below refer to that JSON's `records[].id`.
Observed means executed in that environment; source-reviewed means inspected at the pinned commit; advertised means stated by the pinned documentation without runtime verification.
Rulette findings describe repository implementation and documented intent, not an independently verified distributed release.
No Rulette behavior or specification was changed for this research.

## Journeys and observed behavior

| Journey | Evidence | Result and UX implication |
| --- | --- | --- |
| Discovery | `find`, `help` | Keyword search prints source-qualified installation commands and directory links; live results are time-sensitive. |
| Source inspection | `inspect-missing-command`, `source-preview` | There is no `inspect` command; `add <source> --list` lists names and descriptions without installing, but does not show full instruction bodies or a content diff. |
| First installation | `selective-symlink` | One command with source, skill, agent, and `-y` installs without initialization; preceding it with `--list` makes a two-command preview/install journey. |
| Agent selection | `agent-detection`, `invalid-agent` | Omitting `-a` detects Claude and Codex here and includes universal targets; an invalid agent exits `1` with valid names. |
| Multi-agent placement | `selective-symlink`, `installed-list` | Codex and Cursor share `.agents/skills`; Claude receives a link; reported installed agents differ between add and list, so neither is proof that a harness consumed the skill. |
| Copy mode | `copy` | Companion bytes and mode `755` survive; Claude gets an independent copy and Cursor uses the universal directory. |
| Temporary use | `use` | One command emits only a prompt on stdout and a temporary resource path; supporting files remain available after command exit, without a project install. |
| Local update | `local-update-after-change`, `local-reinstall`, `global-update` | Project update skips local sources even after edits and exits `0`; re-add refreshes them; global update explicitly suggests re-add for local paths. |
| Removal | `remove-one-agent`, `remove-project`, `list-after-remove`, `global-remove` | Agent-specific removal retains other placements; complete named removal yields an empty project list; global removal is separate. |
| Restore | `restore-local` | `experimental_install` restores lock-selected sources to universal `.agents/skills`, rather than recreating every original agent-specific placement or copy choice. |
| Dependency sync | `sync-preview`, `sync` | `experimental_sync --dry-run` leaves files unchanged; sync links the dependency skill and records its package version/path in the project lock. |
| Collision | `collision` | Re-adding the same name from another source overwrites the installed tree, removes a local edit, and replaces provenance; `-y` accepts the printed overwrite summary. |
| Partial failure | `partial-target-failure`, `partial-recovery` | A blocked Claude destination produces exit `1` and JSON failure, but Codex files and a lock entry remain; repairing the destination and re-adding completes installation. |
| Invalid selection/input | `missing-skill`, `invalid-path` | Exit `1` includes available skill names or the missing path; successful prior installations remain. |
| CI output | `json-without-confirmation`, `json-preview-rejected`, `download-json` | Add JSON requires `-y`; `--list --json` is rejected; a generic download URL's well-known dispatch rejects JSON; list JSON works. |
| Search failure | `find-http-failure` | A controlled HTTP `503` produces an empty-result message and exit `0`, making failure indistinguishable from no matches. |

The initial live search encountered a sandbox network approval boundary; after access to `skills.sh:443` was approved, the live journey returned results.
The controlled failure reproduces the diagnostic limitation independently of that boundary; the [search implementation][find] converts non-success responses and exceptions to empty results.
An additional interactive invocation reached the project/global scope picker, displayed keyboard guidance and Project as default, and was cancelled with Ctrl-C at that prompt with exit `0`.
Method selection and final confirmation are source-reviewed rather than measured interactive completions.
Recorded durations are individual observations, not performance estimates or side-by-side Rulette timings.

## Sources and target compatibility

| Dimension | Advertised or source-reviewed support | Hands-on verification |
| --- | --- | --- |
| Local inputs | Directory paths and supported discovery layouts | Two-skill directory, explicit selection, missing path, nested packages |
| Remote Git inputs | GitHub shorthand/URLs/tree paths, GitLab, Azure Repos, arbitrary Git URLs and refs | Source-reviewed; no Git operations or private credentials used |
| Private repositories | Existing Git credentials, GitHub CLI or SSH fallback; optional explicit API tokens | Advertised and source-reviewed; authentication and private updates unverified |
| Downloads | Well-known endpoints and direct `SKILL.md`, ZIP, tar, gzip-tar URLs | Controlled HTTP `SKILL.md` and gzip-tar downloads; ZIP, redirects, and well-known indexes unverified |
| Dependency inputs | `node_modules` and package `skills` fields through experimental sync | One installed dependency with a conventional skills directory |
| Discovery depth | Root `SKILL.md` stops normal discovery; `--full-depth` expands a bounded search; plugin manifests and internal-skill filtering are supported | Root lists one skill by default and two with full depth; plugin/internal handling source-reviewed |
| Installation targets | 79 entries in the pinned [agent definitions][agents] | Explicit Codex, Claude Code, Cursor project placement; Claude global copy; other targets unverified |
| Temporary agent launch | Claude Code, Codex, Sarvam Code in [use implementation][use] | Prompt generation only; no coding agent launched |

The [pinned README][readme] describes network acquisition and directory placement, not per-harness semantic lowering.
The CLI's 79 target entries therefore do not establish 79-way preservation of native rules, activation metadata, permissions, or hooks.
Root/nested behavior follows the [discovery implementation][discovery]; full depth does not mean unlimited recursive traversal.

## Provenance, security, and recovery

The observed project `skills-lock.json` stores a relative local source, source type, and computed SHA-256.
The [project lock implementation][local-lock] supports original URL, ref, skill path, dependency version and requesting package, and writes sorted, timestamp-free names.
Its hash covers relative regular-file paths and bytes, excluding permission bits and symlink entries.
The observed copy included a controlled external symlink target while returning the same source hash as before that link was added.
This demonstrates that the source hash alone does not attest every byte that can enter an installed copy.

The global copy created `~/.agents/.skill-lock.json` in the sandbox.
The [global lock implementation][global-lock] also supports an XDG state location, timestamps, remembered agents, source/ref/path metadata, and GitHub tree hashes.
Project and global lock formats have different purposes and cannot be treated as interchangeable reproducible snapshots.
The [restore implementation][restore] re-resolves the recorded source/ref and does not compare fetched content with the saved computed hash.
Remote update is source-reviewed: it follows recorded source/ref metadata and hashes discovered content; mutable references can change.
Successful remote-update and private-repository recovery journeys remain unverified.

Local copy follows accessible source symlinks, as verified with a fixture sentinel outside the skill directory; the [installer][installer] lacks a realpath boundary for those targets.
The HTTP tar fixture skipped that link.
The [download implementation][downloads] bounds downloads at 10 MiB, extracted content at 25 MiB and 1,000 entries by default; source-reviewed ZIP handling rejects archive links and encrypted entries.
These archive controls do not establish the same boundary for local directory copying.
Installing instructions makes them available to an agent with that agent's permissions; installation is not content sandboxing.

The [telemetry implementation][telemetry] sends source, skill, agent, scope and optional caller metadata by default; opt-out also disables audit retrieval.
The [temporary-use implementation][use] retains materialized resources so later consumers can read them; it is not an automatic cleanup or uninstall contract.
Ordinary add is not transactional across agents, as the partial-failure snapshot demonstrates.
Experimental [sync][sync] has separate ownership and collision rules, including stale-output cleanup; those rules should not be inferred for ordinary add.

## Rulette reconciliation

The [OpenSpec specifications](../openspec/specs/ir-core/spec.md), [man page](2026-04-11-man-page.md), and legacy [PRD](2026-04-11-prd.md) describe five core harnesses, explicit snapshots, exact selection, loss findings, and staged publication.
Repository [input handling](../src/inputs/mod.rs) accepts local files/directories and safe tar/gzip-tar; native plain stdin is not a general skill-package reader.
Repository [Codex parsing](../src/parsers/codex.rs) recognizes `.codex/skills`, whereas the verified skills CLI uses `.agents/skills` for project Codex and Cursor placement.
That layout difference needs a deliberate interoperability decision rather than a claim of automatic compatibility.

Graph retention does not guarantee cross-harness resource emission.
Repository [lowering](../src/emitters/lowering.rs) preserves opaque companions within the source domain, reports their loss across domains, and lowers Cursor skill instructions to rules while dropping companions.
Strict mode rejects non-supported findings; acceptance of loss remains attached to publication evidence.
Repository [inspection](../src/cli/commands/inspect.rs) and [publication orchestration](../src/cli/commands/transform.rs) provide JSON capability inspection, drift checks, staged artifacts, expected-plan digests, and explicit root authorization.
These are implemented repository contracts, not features demonstrated in a released binary by this benchmark.

The base [IR spec](../openspec/specs/ir-core/spec.md), [pipeline spec](../openspec/specs/transform-pipeline/spec.md), and [frontend/backend spec](../openspec/specs/frontends-and-backends/spec.md) still describe the older flat entity model and transform/direct-output surface.
They conflict with the current PRD, command reference, and graph implementation.
`AGENTS.md` prohibits configuration files, while the implementation accepts explicit `--config`; the man page also prohibits configured loss permission, while target configuration implements `allow_lossy`.
Repository code additionally includes an AgentPlugin frontend/target beyond the five documented core harnesses.
These conflicts are unresolved design/documentation questions for synthesis and existing cleanup work, not permission to widen Rulette's architecture.

## Findings handed to synthesis

These classifications are recommendations for `rulette-l8r.3`, not approved product changes.

| Finding | Classification | Proposed measurable criterion or boundary |
| --- | --- | --- |
| Known local source to first useful preview | Match | One explicit inspection command, no initialization, identifies packages and next action; compare actual Rulette journey in synthesis. |
| Scriptable selection and diagnosis | Match | JSON stdout parses without terminal decoration; explicit selection failures exit nonzero and name alternatives. |
| Source/output authority and partial recovery | Beat | Publish only reviewed artifacts; conflicts or failed preflight leave every destination unchanged; verify current stage/apply guarantees separately. |
| Provenance versus reproducibility | Beat | Identify the supplied bytes and accepted loss; document exactly what digests cover; never equate mutable-source restore with byte verification. |
| Failure versus empty result | Beat | Unreadable or unavailable input must not appear as successful empty work; include cause and actionable recovery. |
| Offline operation and bounded inputs | Beat | Explicit local inputs suffice at runtime; document path/link and resource-limit behavior without a network dependency. |
| Cross-agent resource fidelity | Unresolved design question | Decide which resources are portable; preserve strict loss reporting and distinguish package retention from emission. |
| `.agents/skills` interoperability | Unresolved design question | Decide whether to accept this explicit local layout or require adapter preparation; do not silently reinterpret it as `.codex/skills`. |
| Configuration, stale base specs, extra plugin target | Unresolved design question | Resolve authority and release scope before making benchmark commitments; route document cleanup through `rulette-43h`. |
| Marketplace search and network/private-source acquisition | Deliberate non-goal | Source acquisition remains external; Rulette operates on caller-supplied snapshots. |
| Persistent project/global installation, update locks, dependency sync | Deliberate non-goal | No package-manager state or lifecycle is added to the static compiler. |
| Temporary prompt execution and agent launching | Deliberate non-goal | Rulette does not execute skills or launch agents. |
| Matching 79 installation adapters | Deliberate non-goal | Target count is not semantic fidelity; evaluate supported native domains and loss contracts. |

Rulesync evidence from `rulette-l8r.2` must join these findings before `.3` makes final decisions or creates approved follow-up implementation/specification issues.
`rulette-ka2` remains gated by `.3` and its existing human review gate.

[upstream]: https://github.com/vercel-labs/skills
[readme]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/README.md
[agents]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/agents.ts
[find]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/find.ts#L92
[use]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/use.ts
[discovery]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/skills.ts
[local-lock]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/local-lock.ts
[global-lock]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/skill-lock.ts
[restore]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/install.ts
[installer]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/installer.ts
[downloads]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/download-source.ts
[telemetry]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/telemetry.ts
[sync]: https://github.com/vercel-labs/skills/blob/671e8c320810d36fed80fac5f1a2c1bf7e82d812/src/sync.ts
