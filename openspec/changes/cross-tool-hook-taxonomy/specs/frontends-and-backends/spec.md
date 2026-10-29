## ADDED Requirements

### Requirement: Native hook ingestion and target lowering

Frontends SHALL parse native hook configurations from Claude Code (`.claude/settings.json`, `hooks/hooks.json`), Cursor (`.cursor/hooks.json`), Antigravity (`.agents/hooks.json`), and Codex (`.codex/hooks.json`) into `PackageKind::Hook`.
Target backends SHALL lower `PackageKind::Hook` packages into target-specific hook manifests (`NativeArtifactClass::HookConfig`).

#### Scenario: Lowering BeforeShellExecution across target harnesses

- **WHEN** a `Hook` package containing `BeforeShellExecution` is lowered to target backends
- **THEN** Cursor SHALL emit native `beforeShellExecution` in `.cursor/hooks.json`
- **AND** Claude Code SHALL emit `PreToolUse` with `matcher: "Bash"` and report `hook-event-lowered`
- **AND** Antigravity SHALL emit `PreToolUse` with `matcher: "run_command"` and report `hook-event-lowered`
- **AND** Codex SHALL emit `PreToolUse` with `matcher: "^Bash$"` and report `hook-event-lowered`.

#### Scenario: Lowering hook packages to OpenCode reports dropped status

- **WHEN** a `Hook` package is lowered to `opencode`
- **THEN** the backend SHALL classify the hook package as `Dropped` with reason code `hook-target-unsupported`
- **AND** SHALL NOT emit any hook manifest files.

### Requirement: Structured hook capability reporting

`Emitter::capabilities()` SHALL classify every `PackageKind::Hook` package as `Supported`, `Lossy`, or `Dropped` using stable hook `CapabilityReasonCode` variants.

#### Scenario: Reporting stable hook capability reason codes

- **WHEN** capability analysis evaluates hook compatibility across targets
- **THEN** it SHALL report `hook-event-unsupported`, `hook-target-unsupported`, `hook-matcher-unsupported`, `hook-action-unsupported`, `hook-scope-unsupported`, `hook-event-lowered`, or `hook-timeout-unsupported`.

#### Scenario: Reporting unsupported hook events on narrow harnesses

- **WHEN** a `Hook` package contains `Notification` or `SubagentStop` hooks and targets `cursor` or `codex`
- **THEN** capability analysis SHALL report `Dropped` with reason code `hook-event-unsupported`.
