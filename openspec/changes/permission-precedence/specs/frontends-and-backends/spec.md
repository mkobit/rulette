## ADDED Requirements

### Requirement: Fail-safe permission lowering and Asymmetric Approximation Law

Target backends SHALL lower `PackageKind::Permissions` packages into mechanical native permission artifacts (`NativeArtifactClass::PermissionConfig`) and SHALL NEVER fail open or emit advisory Markdown prose.
Under `--allow-lossy`, backends SHALL withhold unrepresentable `Allow` rules and `Allow` rules overlapped by unrepresentable `Deny` rules, promote overlapping `Ask` carve-outs to `Deny` on `Allow`/`Deny`-only targets, and join collapsed tools fail-closed (`deny > ask > allow`).

#### Scenario: Sequential OpenCode emission preserves 4-tier fail-closed precedence

- **WHEN** a `Permissions` package containing `ask("bash", "*")`, `allow("bash", "git *")`, `ask("bash", "git push *")`, and `deny("bash", "git push --force *")` is lowered to `opencode`
- **THEN** the OpenCode backend SHALL emit the `"bash"` permission map in strict ascending tier order (`"*"` $\to$ `"git *"` $\to$ `"git push *"` $\to$ `"git push --force *"`)
- **AND** OpenCode's sequential last-match-wins evaluation SHALL produce the exact same `Allow`, `Ask`, and `Deny` decisions as Claude Code's set-based `deny > ask > allow` evaluation.

#### Scenario: Cursor promotes Ask carve-out to Deny under allow-lossy

- **WHEN** a `Permissions` package containing `allow("bash", "git *")` and `ask("bash", "git push *")` is lowered to `cursor`
- **THEN** strict lowering (`allow_lossy: false`) SHALL fail with `permission-action-unrepresentable`
- **AND** lossy lowering (`allow_lossy: true`) SHALL emit `"Shell(git:*)"` in `permissions.allow` and `"Shell(git:push*)"` in `permissions.deny` while reporting a `Lossy` finding with reason code `permission-ask-promoted-to-deny`.

#### Scenario: Antigravity withholds unrepresentable wildcard Allow and overlapping Allow rules

- **WHEN** a `Permissions` package containing `allow("bash", "git *")` and `allow("bash", "cargo test")` is lowered to `antigravity` under `allow_lossy: true`
- **THEN** the Antigravity backend SHALL emit `"command(cargo test)"` in `permissions.allow`, withhold the wildcard rule `allow("bash", "git *")` without widening it to `"command(git)"`, and report `permission-pattern-unrepresentable`.

#### Scenario: Permissions are never lowered to advisory Markdown prose

- **WHEN** a `Permissions` package is lowered to `agent-plugin` or contains non-`bash` rules lowered to `codex`
- **THEN** the backend SHALL NOT emit natural-language permission prose into `AGENTS.md`, `CLAUDE.md`, `.mdc`, or `SKILL.md`
- **AND** SHALL classify the unrepresentable rules as `Dropped` or `Lossy` with structured capability findings.

### Requirement: Structured permission capability reporting

`Emitter::capabilities()` / `emitters::lowering::capabilities()` and `lower()` SHALL classify every `PackageKind::Permissions` package as `Supported`, `Lossy`, or `Dropped` using stable permission `CapabilityReasonCode` variants.
Each package and target SHALL emit at most one finding per `CapabilityReasonCode`, aggregating affected rules in canonical order with package provenance and the emitted native artifact path when an artifact is produced.

#### Scenario: Stable permission capability reason codes

- **WHEN** capability analysis evaluates unrepresentable permission semantics across targets
- **THEN** it SHALL report `permission-target-unsupported`, `permission-scope-unrepresentable`, `permission-tool-unrepresentable`, `permission-action-unrepresentable`, `permission-ask-promoted-to-deny`, `permission-pattern-unrepresentable`, or `permission-precedence-unrepresentable`.

#### Scenario: Reporting shadowed rules via capability inspection

- **WHEN** `rulette inspect <input> --to claude` or `rulette inspect <input> --coverage` analyzes a `Permissions` package containing `deny("bash", "rm *")` and a shadowed `allow("bash", "rm /tmp/a")`
- **THEN** capability analysis SHALL report a `Lossy` finding with reason code `permission-precedence-unrepresentable` identifying the shadowed rule.

#### Scenario: Reporting completely unrepresentable permission packages as Dropped

- **WHEN** a `Permissions` package is analyzed against `agent-plugin` (or against `codex` when only `read`/`mcp` rules are present)
- **THEN** capability analysis SHALL report `Dropped` (`permission-target-unsupported` or `permission-tool-unrepresentable`) and emit zero permission artifact bytes.
