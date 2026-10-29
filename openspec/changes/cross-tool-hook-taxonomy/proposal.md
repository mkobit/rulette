## Why

Multi-entity governance across Rulette's five core harnesses (Claude Code, Cursor, Antigravity, OpenAI Codex, and OpenCode) requires a generalized hook event taxonomy.
Currently, `src/ir/mod.rs` and hook handling hardcode the five hook events of Claude Code (`PreToolUse`, `PostToolUse`, `Notification`, `Stop`, and `SubagentStop`).
Furthermore, `src/cli/commands/hook_eval.rs` and `src/emitters/hooks.rs` only support `PreToolUse` for shell command approval.
The five core harnesses implement disparate hook architectures:
- Claude Code supports five lifecycle and tool events in `.claude/settings.json` and `hooks/hooks.json`.
- Cursor defines distinct camelCase events (`beforeShellExecution`, `preToolUse`, `postToolUse`, `beforeSubmitPrompt`, and `afterResponse`) in `.cursor/hooks.json`.
- Antigravity supports `PreToolUse`, `PostToolUse`, `SessionStart`, `SessionStop`, `SubagentStart`, `SubagentStop`, and `Notification` in `.agents/hooks.json` and `hooks.json`.
- Codex supports a narrow set of hook events (`PreToolUse`, `PostToolUse`, and `Stop`) in `.codex/hooks.json` with an explicit trust requirement.
- OpenCode provides no declarative static JSON hook manifests and relies on programmatic TypeScript plugins.
Without a canonical hook event taxonomy, Rulette cannot represent, inspect, or lower hooks across these surfaces.
Downstream tools either drop hooks silently or fail without structured diagnostics.
This change (`rulette-5bk.12`) defines a canonical cross-tool hook event taxonomy, a strongly typed hook intermediate representation, deterministic target lowerings, and structured capability reporting.

## What changes

- Add a pure hook domain module in `src/ir/hooks.rs`, re-exported in `src/ir/mod.rs` and `src/lib.rs`.
- Define `CanonicalHookEvent` and `HookEventKind` with nine canonical lifecycle events (`PreToolUse`, `PostToolUse`, `BeforeShellExecution`, `PromptSubmit`, `TurnStop`, `SessionStart`, `SessionStop`, `SubagentStart`, `SubagentStop`, and `Notification`).
- Define `HookMatcher` for tool names, regular expressions, and command prefix patterns.
- Define `HookAction` for command executions with timeout and status messaging.
- Define `HookScope` for project, user, plugin, skill turn, and enterprise levels.
- Add `PackageKind::Hook` and `SemanticItem::Hook` to the compilation graph in `src/ir/graph.rs`.
- Implement native hook ingestion in frontends for Claude Code, Cursor, Antigravity, and Codex.
- Implement deterministic hook lowering to native target formats in `src/emitters/lowering.rs` and `src/emitters/hooks.rs`.
- Enforce fail-closed security for unrepresentable pre-tool gating hooks under strict transform mode.
- Report structured capability findings using seven stable hook reason codes (`hook-event-unsupported`, `hook-target-unsupported`, `hook-matcher-unsupported`, `hook-action-unsupported`, `hook-scope-unsupported`, `hook-event-lowered`, and `hook-timeout-unsupported`).

## Scope

- Canonical hook event taxonomy and data types in `src/ir/hooks.rs`.
- Integration into the compilation graph in `src/ir/graph.rs`.
- Hook parsing frontends for Claude Code, Cursor, Antigravity, and Codex.
- Hook lowering backends and polyglot manifest emission for all core targets.
- Structured capability analysis and reporting in `inspect --to` and `inspect --coverage`.
- Staged publication integration for `NativeArtifactClass::HookConfig`.

## Non-goals

- OpenCode programmatic runtime plugin execution (OpenCode lacks static hook manifests and reports `Dropped`).
- In-process JavaScript hook mods or UI render hooks (Claude Code `register.js` mods report `Dropped`).
- Interactive stdin prompt hijacking or arbitrary shell daemon management.
- Dynamic runtime process supervision outside of static configuration emission and `rulette hook-eval`.

## Capabilities

### Modified capabilities

- `ir-core`: Add `PackageKind::Hook`, `SemanticItem::Hook`, `CanonicalHookEvent`, `HookEventKind`, `HookMatcher`, `HookAction`, `HookScope`, and `TargetHookPolicy`.
- `frontends-and-backends`: Add native hook parsing, target lowering, capability inspection, and reason codes across all five harnesses.
- `transform-pipeline`: Enforce fail-closed verification of hook capabilities, report lossy lowerings and dropped hooks in staged publication plans.

## Impact

- `src/ir/hooks.rs` and `src/ir/mod.rs`: Create hook domain module and export public types.
- `src/ir/graph.rs`: Add `PackageKind::Hook` and `SemanticItem::Hook`.
- `src/parsers/{claude,cursor,antigravity,codex}.rs`: Parse native hook manifests into `PackageKind::Hook`.
- `src/emitters/hooks.rs`: Expand polyglot hook manifest generators to support all canonical hook events.
- `src/emitters/lowering.rs`: Add `NativeArtifactClass::HookConfig`, hook reason codes, and lowering logic.
- `src/publication/mapping.rs`: Register verified `HookConfig` publication paths across scopes.
- `src/cli/commands/hook_eval.rs`: Connect evaluation with expanded hook event context.
- `src/cli/commands/inspect.rs`: Report `hook` package kinds in inspection commands.
- Tracking bead: `rulette-5bk.12`.
