## ADDED Requirements

### Requirement: Fail-closed gating hook policy in transform and staged plans

Strict transform compilation (`allow_lossy: false`) SHALL reject any `PackageKind::Hook` lowering that drops or lossily approximates a security gating hook (`fail_closed: true` or `PreToolUse` / `BeforeShellExecution`).
Strict rejection diagnostics SHALL identify the stable hook `CapabilityReasonCode`, `PackageId`, and affected hook identifiers.
`--allow-lossy` SHALL permit staging only with recorded capability findings in `rulette.plan.json`.

#### Scenario: Strict mode rejects unrepresentable security gating hooks

- **WHEN** `rulette transform` targets a harness that cannot represent a security gating hook without `--allow-lossy`
- **THEN** compilation SHALL fail immediately with a non-zero exit code
- **AND** Rulette SHALL NOT write any files to the destination directory.

#### Scenario: Staged publication plans retain dropped hook findings

- **WHEN** a `Hook` package is staged with `--allow-lossy` and produces `hook-event-unsupported` or `hook-target-unsupported` findings
- **THEN** `rulette.plan.json` SHALL record each finding with its reason code, package identity, and target name
- **AND** `parse_plan_with_expected_digest` SHALL round-trip the plan without loss.
