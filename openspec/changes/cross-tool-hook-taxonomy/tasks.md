## Execution blueprint and dependency map

This task breakdown structures downstream implementation into sequential layers:

- Layer 1 (Task 1) defines the pure hook domain types and canonical taxonomy.
- Layer 2 (Task 2) integrates hooks into the compilation graph and schema generators.
- Layer 3 (Task 3) implements native frontend hook parsers across core harnesses.
- Layer 4 (Task 4) implements target lowering, capability analysis, and polyglot manifest generation.
- Layer 5 (Task 5) connects publication path mapping and CLI inspection commands.
- Layer 6 (Task 6) validates end-to-end integration through automated test suites.

## 1. Layer 1: Pure hook domain types and canonical taxonomy

- [x] 1.1 Create `src/ir/hooks.rs` and export public types through `src/ir/mod.rs` and `src/lib.rs`:
  - Define `HookEventKind` with nine canonical events (`PreToolUse`, `PostToolUse`, `BeforeShellExecution`, `PromptSubmit`, `TurnStop`, `SessionStart`, `SessionStop`, `SubagentStart`, `SubagentStop`, and `Notification`).
  - Implement `HookMatcher` (`All`, `Tool`, `Regex`, and `CommandPrefix`).
  - Implement `HookAction` (`Command` with timeout and status message, and `Unsupported`).
  - Implement `HookScope` (`Project`, `User`, `Plugin`, `SkillTurn`, and `Enterprise`).
  - Implement `HookDefinition` with `fail_closed` and `priority` fields.
  - Implement `TargetHookPolicy` (`Bare` and `Wrapped`).
- [x] 1.2 Implement serialization and deserialization helpers for hook types:
  - Add JSON schema derivation using `schemars::JsonSchema`.
  - Enforce fail-closed validation on empty IDs, invalid regex patterns, and negative timeouts.
- [x] 1.3 Write unit tests in `src/ir/hooks.rs`:
  - Verify round-trip JSON serialization for all canonical events.
  - Verify rejection of malformed hook definitions.

## 2. Layer 2: Compilation graph and schema integration

- [x] 2.1 Extend `src/ir/graph.rs` to support hook packages:
  - Add `PackageKind::Hook` and map `PackageKind::Hook.semantic_kind()` to `Some("hook")`.
  - Update `SemanticIdentity::parse` to accept `"hook"`.
  - Add `SemanticItem::Hook` containing `TargetHookPolicy` and optional `FrontendPayload`.
  - Update `Package::validate()` to verify hook package invariants.
  - Update `Package::package_id()` to hash canonical hook policy bytes.
  - Implement constructor `Package::hook()`.
- [x] 2.2 Update schema generator in `src/cli/commands/schema.rs`:
  - Register new hook types in generated JSON schemas.
- [x] 2.3 Write unit tests in `src/ir/graph.rs`:
  - Verify `PackageId` sensitivity to changes in hook policy.
  - Verify graph JSON and TOML round-trip with hook packages.

## 3. Layer 3: Frontend hook parsers

- [ ] 3.1 Update Claude Code parser in `src/parsers/claude.rs`:
  - Parse `"hooks"` in `.claude/settings.json` and `hooks/hooks.json` into `PackageKind::Hook`.
  - Map `PreToolUse`, `PostToolUse`, `Notification`, `Stop`, and `SubagentStop`.
- [ ] 3.2 Update Cursor parser in `src/parsers/cursor.rs`:
  - Parse `.cursor/hooks.json` and `hooks/hooks.json` into `PackageKind::Hook`.
  - Map `beforeShellExecution`, `preToolUse`, `postToolUse`, `beforeSubmitPrompt`, and `afterResponse`.
- [ ] 3.3 Update Antigravity parser in `src/parsers/antigravity.rs`:
  - Parse `.agents/hooks.json` and root `hooks.json` into `PackageKind::Hook`.
  - Map `PreToolUse`, `PostToolUse`, `SessionStart`, `SessionStop`, `SubagentStart`, `SubagentStop`, and `Notification`.
- [ ] 3.4 Update Codex parser in `src/parsers/codex.rs`:
  - Parse `.codex/hooks.json` into `PackageKind::Hook`.
  - Map `PreToolUse`, `PostToolUse`, and `Stop`.
- [ ] 3.5 Write parser unit tests in `src/parsers/`:
  - Verify correct parsing of native hook configurations into canonical representations.
  - Verify rejection of invalid event names.

## 4. Layer 4: Target lowering, capability analysis, and manifest emission

- [ ] 4.1 Update `src/emitters/lowering.rs`:
  - Add `NativeArtifactClass::HookConfig`.
  - Add seven stable hook reason codes to `CapabilityReasonCode` (`hook-event-unsupported`, `hook-target-unsupported`, `hook-matcher-unsupported`, `hook-action-unsupported`, `hook-scope-unsupported`, `hook-event-lowered`, and `hook-timeout-unsupported`).
  - Implement `lower_hook_package()` for Claude, Cursor, Antigravity, Codex, OpenCode, and AgentPlugin.
  - Classify OpenCode hook packages as `Dropped` (`hook-target-unsupported`).
  - Lower `BeforeShellExecution` to `PreToolUse` with harness-specific shell tool matchers.
- [ ] 4.2 Update polyglot hook manifest generation in `src/emitters/hooks.rs`:
  - Expand `HookManifestTarget` generation to support all canonical hook events.
  - Ensure compatibility with Windows executable naming (`rulette.exe`).
- [ ] 4.3 Write lowering unit tests in `src/emitters/lowering.rs`:
  - Verify capability severity and reason codes for supported, lossy, and dropped hooks.
  - Verify fail-closed behavior for dropped gating hooks under strict mode.

## 5. Layer 5: Publication mapping and CLI integration

- [ ] 5.1 Update `src/publication/mapping.rs`:
  - Register default relative publication paths for `NativeArtifactClass::HookConfig` across targets.
- [ ] 5.2 Update `src/cli/commands/inspect.rs`:
  - Report `hook` package kinds in `inspect --to` and `inspect --coverage`.
- [ ] 5.3 Update `src/cli/commands/hook_eval.rs`:
  - Handle invocation arguments across all canonical hook events.
- [ ] 5.4 Write CLI unit tests:
  - Verify inspect coverage reports for hook packages across all five targets.

## 6. Layer 6: Verification and end-to-end testing

- [ ] 6.1 Execute full test suite with `cargo test`.
- [ ] 6.2 Execute linter checks with `cargo clippy --all-targets --all-features`.
- [ ] 6.3 Execute code formatting checks with `cargo fmt -- --check`.
- [ ] 6.4 Validate specifications strictly using `mise run spec-validate`.
