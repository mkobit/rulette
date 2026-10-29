## ADDED Requirements

### Requirement: Typed hook definitions and hook policies

The compilation graph SHALL represent portable hook configurations using `PackageKind::Hook` and `SemanticItem::Hook` containing a `TargetHookPolicy`.
Each `HookDefinition` SHALL specify an `id`, a `HookEventKind`, an optional `HookMatcher`, a `HookAction`, a `HookScope`, and a `fail_closed` boolean.
Validation SHALL fail closed by rejecting empty IDs, invalid regex patterns, or negative timeout values.

#### Scenario: Parsing a valid hook policy with per-target overrides

- **WHEN** a `Hook` package defines a `TargetHookPolicy` with a `default` policy and target overrides for `cursor` and `antigravity`
- **THEN** Rulette SHALL construct a validated `PackageKind::Hook` package whose `PackageId` digest covers the canonical hook policy
- **AND** target resolution SHALL return `overrides[target]` with full replacement before falling back to `default`.

#### Scenario: Rejecting a hook definition with missing or invalid event kind

- **WHEN** a source document or graph interchange payload contains a `HookDefinition` with an omitted or unknown `event` value
- **THEN** Rulette SHALL fail validation immediately before constructing a `CompilationGraph`
- **AND** SHALL NOT default the event to `PreToolUse`.

### Requirement: Canonical hook event taxonomy

Rulette SHALL model hook events using `HookEventKind` with nine canonical events (`PreToolUse`, `PostToolUse`, `BeforeShellExecution`, `PromptSubmit`, `TurnStop`, `SessionStart`, `SessionStop`, `SubagentStart`, `SubagentStop`, and `Notification`).

#### Scenario: Canonical representation of shell execution hooks

- **WHEN** a hook intercepts shell execution
- **THEN** Rulette SHALL represent the event as `HookEventKind::BeforeShellExecution`
- **AND** preserve command prefix matchers in `HookMatcher::CommandPrefix`.

#### Scenario: Canonical representation of agent lifecycle hooks

- **WHEN** a hook configures session start, session stop, or subagent completion callbacks
- **THEN** Rulette SHALL represent them as `SessionStart`, `SessionStop`, or `SubagentStop` in the intermediate representation.
