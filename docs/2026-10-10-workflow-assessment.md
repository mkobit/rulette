# Rulette workflow assessment

This assessment covers the six journeys approved for `rulette-ka2` in the [competitor synthesis](2026-10-10-competitor-synthesis.md#required-user-journeys).
It measures the current CLI without changing product behavior or approving D09–D11 or D14–D17.
The [command reference](2026-04-11-man-page.md) and [generated help](cli/rulette.md) supply the publication contract and flags.

## Method and evidence

The [replay collector](benchmarks/rulette-workflow-assessment.py) creates a Codex `AGENTS.md` source, publication roots and stages under sandbox `/tmp`.
It records the binary digest/version, arguments, stdout, stderr, exit status, process elapsed time and before/after destination bytes, hashes and modes for every journey invocation.
The version lookup is collection metadata rather than a journey command.
Its elapsed times are agent execution proxies, excluding source edits, tool overhead and human review; they are not human usability measurements.
Two explicit help lookups precede the first preview.
Expected negative gates count as failed commands, separately from accidental command errors.
The collector requires the repository debug binary and does not acquire competitors or use a live project destination.

```bash
sbx env exec .sbx/sbxenv.yaml -- cargo build --bin rulette
sbx env exec .sbx/sbxenv.yaml -- python3 docs/benchmarks/rulette-workflow-assessment.py
sbx env exec .sbx/sbxenv.yaml -- mise run lint:md
```

## Journey boundaries

| Journey | Exercised path | Required concepts and limit |
| --- | --- | --- |
| Preview/publication | `inspect --to codex`; `transform --stage`; digest-authorized `--apply` | Explicit source decoder, target/scope, stage, root and digest authority; review remains a human step. |
| Multi-target | Codex and Claude project targets, with a repeated Codex target; one stage and one apply | Target normalization and explicit shared project root; this does not establish skill-resource fidelity. |
| CI drift/coverage | Unchanged and revised-source `--check`; `inspect --coverage --json --strict` | Exit status gates destination drift; coverage describes reported package-kind loss rather than every field. |
| Update/migration | Revised native snapshot; stage; conflicting apply; `--replace` | External source production remains caller-owned; this is snapshot replacement, not an installer update or Rulesync parity experiment. |
| Failure recovery | Wrong digest, missing input, conflicting sources and unsupported user scope; corrected plan check | Preflight/no-write recovery only; observed late-write rollback and crash recovery are not measured. |
| Retirement/shared destinations | Re-stage Codex alone after Codex/Claude publication; retain unrelated caller-owned file | Target omission grants no deletion authority; shared-key merge and stale-output retirement remain unsupported design questions. |

## Observed outcomes

The [runtime records](benchmarks/rulette-workflow-assessment.json) contain every command and its complete output.
The final replay completed on October 11, 2026, after `cargo build --bin rulette` succeeded against current source.
It completed all 20 journey invocations, with six intentional negative gates and no accidental command errors.
The measured binary reports `rulette 0.1.0`; its SHA-256 is `a48127f3c7a87bdb0902d050765c94b5d89853ec2d5b979772a7e2fd32528d0c`.

The table summarizes the final replay's process times, including its help lookups.
Failed attempts are intentional negative probes, not evidence of human confusion; accidental command failures were zero in every journey.

| Journey | Commands | Help lookups | Intentional failed attempts | Accidental failures | Agent process seconds |
| --- | --- | --- | --- | --- | --- |
| Preview/publication | 5 | 2 | 0 | 0 | 0.2380 |
| Multi-target | 2 | 0 | 0 | 0 | 0.0431 |
| CI drift/coverage | 3 | 0 | 1 | 0 | 0.0239 |
| Update/migration | 3 | 0 | 1 | 0 | 0.0526 |
| Failure recovery | 5 | 0 | 4 | 0 | 0.0705 |
| Retirement/shared destinations | 2 | 0 | 0 | 0 | 0.0275 |

Preview showed the exact package identifier, instruction body, provenance and target findings in one invocation.
The two help lookups supplied the staging and apply flags; output itself did not supply a next publication command.
Stage produced a digest on stderr without changing destination files, and apply created `AGENTS.md`.
Multi-target staging normalized the repeated Codex target; apply reported one unchanged entry and created `CLAUDE.md`.

The unchanged destination check exited `0`; changing the source made the check exit `1` with an entry identifier and `conflict` status.
Coverage stdout parsed as JSON and reported the rule as supported for all five core targets; strict coverage exited `0` for this fixture.
These outputs establish the measured rule case, not field or skill-resource fidelity.

Updating the snapshot required stage, a denied conflicting apply, and an explicit `--replace` apply.
The conflict diagnostic named the entry and required replacement permission; it did not print its destination path.
Only the authorized replacement changed `AGENTS.md`, leaving the independently published Claude snapshot intact.
Migration from another tool's configuration, skill layouts or resources was not exercised; [existing migration regression evidence](../tests/rulesync_migration_tests.rs) remains separate.

Wrong-digest, missing-input and Cursor user-scope commands each exited `1` without changing destination file bytes or modes.
The digest error printed expected and actual digests; the missing-input error printed the path and cause; the scope error named Cursor's unavailable user mapping.
The corrected digest-authorized plan check exited `0` and reported `unchanged`.
Two explicit Codex snapshots with different `AGENTS.md` bodies failed aggregation with `aggregate package collisions` and the shared semantic identity `rule:AGENTS.md`.
The diagnostic printed both package IDs, Codex provenance values and outer input identities; those identities are opaque hashes rather than readable source paths.
It provided no corrective action, leaving the caller to map identities to inputs and resolve the conflicting source snapshots.
The recorded destination files and modes remained unchanged.
This single two-origin collision report does not establish complete reporting for a larger monorepo, and D14 remains open.
No observed late-write failure was induced, so this assessment does not measure rollback diagnostics or crash recovery.

Omitting Claude from the final Codex-only stage/apply left `CLAUDE.md` and the unrelated caller-owned file unchanged.
Its Claude instructions remained the older snapshot, making caller-owned retirement visible.
The fixture demonstrates retention of separate files, not preservation or merging of shared keys within one native file.
Stale-output cleanup, shared ownership and edited shared-file behavior remain unresolved at D15.

All preview, stage, check and denied operations retained the recorded destination files and their modes.
The snapshots cover regular-file bytes and modes beneath the temporary live root; they do not prove the absence of empty-directory creation, detect every race or establish crash atomicity.

## Usability limits and open decisions

Publication succeeded with an explicit native source and documented flags, requiring source decoder, target/scope, stage, root and digest concepts.
Entry-only conflict reports add a plan lookup when a caller needs the destination path; no general corrective command is printed.
The agent followed documentation and scripted commands, so these results cannot establish discoverability, human concept burden or time to first success.
No first-time participant, interactive discovery study or side-by-side competitor timing was measured.
D09–D11 and D14–D17 remain open; the assessment approves neither configuration/loss authority nor field fidelity, skill resources/layouts, collision policy, retirement or plugin scope.
No implementation or product-contract change is proposed by the observed outcomes.
