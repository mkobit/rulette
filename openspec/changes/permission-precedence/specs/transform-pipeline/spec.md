## ADDED Requirements

### Requirement: Strict fail-closed permission loss policy in transform and staged plans

Strict transform compilation (`allow_lossy: false`) SHALL reject any `PackageKind::Permissions` lowering that produces a `Lossy` or `Dropped` capability finding before creating a staging directory or modifying any destination file.
Strict rejection diagnostics SHALL identify the stable permission `CapabilityReasonCode`, `PackageId`, and affected `ToolAccessRule` entries.
`--allow-lossy` SHALL permit staging only with fail-closed approximations and retain all findings in `rulette.plan.json`.

#### Scenario: Multi-target permission lowering rejects atomically under strict mode

- **WHEN** `rulette transform` targets both `claude` (which supports a `Permissions` package losslessly) and `cursor` (which encounters an unrepresentable `Ask` rule) without `--allow-lossy`
- **THEN** compilation SHALL fail with a diagnostic naming `permission-action-unrepresentable`
- **AND** Rulette SHALL NOT create the staging directory or write any target artifacts.

#### Scenario: Permitted permission loss round-trips through staged publication plans

- **WHEN** a `Permissions` package is staged with `--allow-lossy` and produces `permission-ask-promoted-to-deny`, `permission-pattern-unrepresentable`, or `permission-precedence-unrepresentable` findings
- **THEN** `rulette.plan.json` SHALL record each finding with its exact reason code, package identity, and artifact descriptor
- **AND** `parse_plan_with_expected_digest` SHALL round-trip the plan without loss.
