# Generalized cross-tool hook event taxonomy design

## Context

`docs/2026-08-18-cli-ux-design.md` (Journey 6) identifies hook taxonomy as a cross-cutting Intermediate Representation (IR) blocker for multi-entity governance.
Rulette compiles AI rules, skills, subagents, and permissions across five core harnesses: Claude Code, Cursor, Google Antigravity, OpenAI Codex, and OpenCode.
However, hook configurations remain fragmented across these tools.
Historically, `src/ir/mod.rs` and hook emission hardcoded the five hook events supported by Claude Code (`PreToolUse`, `PostToolUse`, `Notification`, `Stop`, and `SubagentStop`).
Furthermore, `src/emitters/hooks.rs` and `src/cli/commands/hook_eval.rs` only support `PreToolUse` for shell execution gating.
Other tools define different hook events, use different casing conventions, or lack declarative hook manifests entirely.
Without a generalized taxonomy, Rulette cannot reliably inspect, translate, or lower hook manifests.

### Empirical cross-harness hook survey

Each harness implements a distinct hook surface, configuration path, event naming scheme, and execution protocol:

| Harness | Native config surface | Native hook events | Matcher support | Payload and protocol | Key limitations |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Claude Code** (`claude`) | `.claude/settings.json`, `hooks/hooks.json` | `PreToolUse`, `PostToolUse`, `Notification`, `Stop`, `SubagentStop` | Regex or exact tool name (`matcher`) on tool events | Receives JSON via stdin; expects JSON on stdout (`permissionDecision: "allow" \| "deny"`) | Cannot intercept shell executions separately from other tools without matching `"Bash"`. Does not support session bootstrap hooks. |
| **Cursor** (`cursor`) | `.cursor/hooks.json`, `hooks/hooks.json` | `beforeShellExecution`, `preToolUse`, `postToolUse`, `beforeSubmitPrompt`, `afterResponse` | Shell command prefix on `beforeShellExecution`; tool name on `preToolUse` | Receives JSON via stdin; returns exit code 0 (`allow`) or exit code 2 (`deny`) | Uses camelCase event names. Lacks notification and subagent lifecycle hooks. |
| **Antigravity** (`antigravity`) | `.agents/hooks.json`, root `hooks.json` | `PreToolUse`, `PostToolUse`, `SessionStart`, `SessionStop`, `SubagentStart`, `SubagentStop`, `Notification` | Exact tool name matcher (e.g. `run_command`, `view_file`) | Receives JSON via stdin; expects JSON on stdout (`decision: "auto_approve" \| "deny"`) | Requires PascalCase event names. Shell tool is named `run_command` rather than `Bash`. |
| **Codex** (`codex`) | `.codex/hooks.json`, `hooks/hooks.json` | `PreToolUse`, `PostToolUse`, `Stop` | Regex matcher on tool name (e.g. `^Bash$`) | Receives JSON via stdin; expects JSON on stdout (`permissionDecision: "allow" \| "deny"`) | Narrow hook surface. Requires interactive trust approval before executing hooks. Lacks prompt and subagent hooks. |
| **OpenCode** (`opencode`) | No static manifest (`opencode.json` plugins) | None (programmatic TypeScript plugins only) | N/A | TypeScript plugin callbacks | Does not support declarative JSON hook manifests. All declarative hooks drop. |
| **AgentPlugin** (`agent-plugin`) | `hooks/hooks.json` | `PreToolUse`, `PostToolUse`, `Stop`, `SubagentStop`, `Notification` | Tool name matcher | Standardized JSON payload via stdin and stdout | Standardized open format. Lacks session bootstrap and prompt submission hooks. |

## Goals and non-goals

### Goals

- Define a canonical hook event taxonomy (`CanonicalHookEvent` / `HookEventKind`) capturing tool execution, prompt submission, turn completion, session lifecycle, and subagent lifecycle.
- Implement a strongly typed intermediate representation in `src/ir/hooks.rs` with `HookDefinition`, `HookMatcher`, `HookAction`, `HookScope`, and `TargetHookPolicy`.
- Promote hooks into first-class compilation graph units using `PackageKind::Hook` and `SemanticItem::Hook` in `src/ir/graph.rs`.
- Define deterministic target lowering rules for all five core harnesses and the AgentPlugin format.
- Provide structured capability analysis via `Emitter::capabilities()` with seven stable machine-readable reason codes.
- Enforce fail-closed security for pre-tool execution gating hooks under strict transformation mode.
- Maintain a static binary architecture with zero runtime language dependencies.

### Non-goals

- Executing programmatic TypeScript plugins for OpenCode.
- Synthesizing in-process JavaScript mods for Claude Code.
- Managing background server daemons or persistent agent processes.
- Intercepting interactive user terminal keystrokes directly.

## Architectural decisions

### 1. Strongly typed hook domain types

All hook domain types reside in `src/ir/hooks.rs`.
They contain zero dependencies on CLI arguments or file system I/O.
The types re-export through `src/ir/mod.rs` and `src/lib.rs`.

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical hook event taxonomy covering agent and tool lifecycles.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HookEventKind {
    /// Triggers before any tool invocation.
    PreToolUse,
    /// Triggers after any tool invocation completes.
    PostToolUse,
    /// Triggers specifically before a shell or terminal command executes.
    BeforeShellExecution,
    /// Triggers before a user prompt submits to the model.
    PromptSubmit,
    /// Triggers when the model completes an interaction turn.
    TurnStop,
    /// Triggers when an agent session initializes.
    SessionStart,
    /// Triggers when an agent session terminates.
    SessionStop,
    /// Triggers when a child subagent spawns.
    SubagentStart,
    /// Triggers when a child subagent completes and returns control.
    SubagentStop,
    /// Triggers when an agent emits a user-facing notification or alert.
    Notification,
}

/// Pattern matcher applied to filter hook execution.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", tag = "type", content = "value")]
pub enum HookMatcher {
    /// Matches all invocations unconditionally.
    All,
    /// Matches exact tool names (for example, "run_command" or "Bash").
    Tool(String),
    /// Matches tool names via a regular expression.
    Regex(String),
    /// Matches shell command prefixes.
    CommandPrefix(Vec<String>),
}

/// Action performed when a hook triggers.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum HookAction {
    /// Executes a shell command with JSON payload on standard input.
    Command {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout_seconds: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status_message: Option<String>,
    },
    /// An unsupported or platform-specific action (for example, JavaScript mods).
    Unsupported {
        kind: String,
        description: String,
    },
}

/// Scope and authority level for hook execution.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HookScope {
    /// Active for the local workspace or project.
    #[default]
    Project,
    /// Active for the current user across workspaces.
    User,
    /// Active as part of a packaged plugin bundle.
    Plugin,
    /// Active only during execution of a specific skill turn.
    SkillTurn,
    /// Mandated by enterprise administrative policy.
    Enterprise,
}

/// Individual canonical hook definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HookDefinition {
    pub id: String,
    pub event: HookEventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<HookMatcher>,
    pub action: HookAction,
    #[serde(default)]
    pub scope: HookScope,
    #[serde(default = "default_fail_closed")]
    pub fail_closed: bool,
    #[serde(default)]
    pub priority: i32,
}

fn default_fail_closed() -> bool {
    true
}

/// Target-specific overrides or bare hook policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum TargetHookPolicy {
    Bare(Vec<HookDefinition>),
    Wrapped {
        default: Vec<HookDefinition>,
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        overrides: std::collections::BTreeMap<String, Vec<HookDefinition>>,
    },
}
```

### 2. Compilation graph integration

The compilation graph in `src/ir/graph.rs` integrates hooks as first-class entities:
- Add `PackageKind::Hook`.
- Map `PackageKind::Hook.semantic_kind()` to `Some("hook")`.
- Update `SemanticIdentity::parse` to accept `"hook"`.
- Add `SemanticItem::Hook`:
  ```rust
  SemanticItem::Hook {
      policy: TargetHookPolicy,
      #[serde(skip_serializing_if = "Option::is_none")]
      frontend_payload: Option<FrontendPayload>,
  }
  ```
- Calculate `PackageId` digests using the canonical serialized representation of `TargetHookPolicy`.
- Enforce unique hook IDs within a compilation unit.

### 3. Target capability matrix and lowering semantics

Lowering maps canonical hooks to native configuration manifests.
When a target does not natively support an event, the backend evaluates lowering or drops the hook:

| Canonical hook event | Claude Code | Cursor | Antigravity | Codex | OpenCode | AgentPlugin |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `PreToolUse` | `PreToolUse` (`Supported`) | `preToolUse` (`Supported`) | `PreToolUse` (`Supported`) | `PreToolUse` (`Supported`) | Dropped (`hook-target-unsupported`) | `PreToolUse` (`Supported`) |
| `PostToolUse` | `PostToolUse` (`Supported`) | `postToolUse` (`Supported`) | `PostToolUse` (`Supported`) | `PostToolUse` (`Supported`) | Dropped (`hook-target-unsupported`) | `PostToolUse` (`Supported`) |
| `BeforeShellExecution` | Lowered to `PreToolUse` with `matcher: "Bash"` (`Lossy`) | `beforeShellExecution` (`Supported`) | Lowered to `PreToolUse` with `matcher: "run_command"` (`Lossy`) | Lowered to `PreToolUse` with `matcher: "^Bash$"` (`Lossy`) | Dropped (`hook-target-unsupported`) | Lowered to `PreToolUse` with `matcher: "bash"` (`Lossy`) |
| `PromptSubmit` | Dropped (`hook-event-unsupported`) | `beforeSubmitPrompt` (`Supported`) | Dropped (`hook-event-unsupported`) | Dropped (`hook-event-unsupported`) | Dropped (`hook-target-unsupported`) | Dropped (`hook-event-unsupported`) |
| `TurnStop` | `Stop` (`Supported`) | `afterResponse` (`Supported`) | Lowered to `SessionStop` (`Lossy`) | `Stop` (`Supported`) | Dropped (`hook-target-unsupported`) | `Stop` (`Supported`) |
| `SessionStart` | Dropped (`hook-event-unsupported`) | Dropped (`hook-event-unsupported`) | `SessionStart` (`Supported`) | Dropped (`hook-event-unsupported`) | Dropped (`hook-target-unsupported`) | Dropped (`hook-event-unsupported`) |
| `SessionStop` | Dropped (`hook-event-unsupported`) | Dropped (`hook-event-unsupported`) | `SessionStop` (`Supported`) | Dropped (`hook-event-unsupported`) | Dropped (`hook-target-unsupported`) | Dropped (`hook-event-unsupported`) |
| `SubagentStart` | Dropped (`hook-event-unsupported`) | Dropped (`hook-event-unsupported`) | `SubagentStart` (`Supported`) | Dropped (`hook-event-unsupported`) | Dropped (`hook-target-unsupported`) | Dropped (`hook-event-unsupported`) |
| `SubagentStop` | `SubagentStop` (`Supported`) | Dropped (`hook-event-unsupported`) | `SubagentStop` (`Supported`) | Dropped (`hook-event-unsupported`) | Dropped (`hook-target-unsupported`) | `SubagentStop` (`Supported`) |
| `Notification` | `Notification` (`Supported`) | Dropped (`hook-event-unsupported`) | `Notification` (`Supported`) | Dropped (`hook-event-unsupported`) | Dropped (`hook-target-unsupported`) | `Notification` (`Supported`) |

### 4. Structured capability reporting and reason codes

Rulette classifies capability results as `Supported`, `Lossy`, or `Dropped`.
The system provides seven stable machine-readable reason codes in `CapabilityReasonCode`:
1. `hook-event-unsupported`: Target harness does not support the canonical event.
2. `hook-target-unsupported`: Target harness does not support declarative static hook manifests.
3. `hook-matcher-unsupported`: Target harness does not support the specified matcher type.
4. `hook-action-unsupported`: Target harness does not support the specified hook action.
5. `hook-scope-unsupported`: Target harness does not support hooks declared at this scope.
6. `hook-event-lowered`: Backend lowered the canonical event with semantic approximation.
7. `hook-timeout-unsupported`: Target harness cannot enforce hook timeout configuration.

### 5. Fail-closed security invariant

Gating hooks (`PreToolUse` and `BeforeShellExecution`) enforce security policies on tool execution.
If a security gating hook cannot be represented on a target, silent omission risks unconstrained command execution.
Rulette enforces the Fail-Closed Security Invariant:
- Under strict mode (`allow_lossy: false`), transformation fails immediately if any security gating hook drops.
- Under lossy mode (`allow_lossy: true`), the compiler drops the hook, records a `Dropped` finding with `hook-event-unsupported` or `hook-target-unsupported`, and includes the finding in `rulette.plan.json`.

## Mandatory adversarial review

### Hazard 1: Silent security degradation on targets without hooks

- **Description**:
  OpenCode lacks a declarative static hook system.
  Emitting a workspace configuration without warning would allow tools to execute without configured guard hooks.
- **Attack Vector**:
  An engineer configures a `PreToolUse` hook to block destructive commands (`rm -rf`).
  The engineer lowers the configuration to OpenCode.
  OpenCode runs commands without invoking the hook.
- **Mitigation**:
  Rulette marks all declarative hooks as `Dropped` (`hook-target-unsupported`) on OpenCode.
  In strict mode (`transform` default), Rulette aborts transformation with a non-zero exit code.
  In lossy mode (`--allow-lossy`), Rulette records an explicit finding in the staged publication plan.

### Hazard 2: Shell matcher mismatch across harnesses

- **Description**:
  Harnesses use different internal names for shell execution tools.
  Claude Code uses `Bash`.
  Google Antigravity uses `run_command`.
  Cursor uses `beforeShellExecution` or `Shell`.
  OpenAI Codex uses `^Bash$`.
- **Attack Vector**:
  Lowering a generic `BeforeShellExecution` hook as `PreToolUse` with an unmapped tool name causes the hook to ignore shell commands.
- **Mitigation**:
  The canonical lowering logic maps `BeforeShellExecution` deterministically to each target's exact tool identifier.
  Lowering to Claude Code emits `Bash`.
  Lowering to Antigravity emits `run_command`.
  Lowering to Codex emits `^Bash$`.
  Lowering to Cursor emits native `beforeShellExecution`.

### Hazard 3: Non-zero exit code versus JSON response protocol divergence

- **Description**:
  Different harnesses communicate hook approval and denial through conflicting protocols.
  Cursor treats exit code 0 as allow and exit code 2 as deny.
  Claude Code reads stdout JSON `{"permissionDecision": "deny"}`.
  Antigravity reads stdout JSON `{"decision": "deny"}`.
- **Attack Vector**:
  A custom hook script returns an exit code without stdout JSON, or returns stdout JSON without the target's expected schema, causing the harness to hang or fail open.
- **Mitigation**:
  Polyglot hook manifests emitted by Rulette invoke `rulette hook-eval`.
  `rulette hook-eval` automatically formats responses to match the detected target harness protocol.

### Hazard 4: Infinite loop from hook invoking tool

- **Description**:
  A hook script invokes a tool that triggers the same hook recursively, causing a resource exhaustion loop.
- **Attack Vector**:
  A `PreToolUse` hook script invokes a shell command to log execution.
  The shell command triggers `BeforeShellExecution`, creating infinite subprocess spawning.
- **Mitigation**:
  When executing commands through `rulette hook-eval`, Rulette exports an environment variable `RULETTE_HOOK_ACTIVE=1`.
  Hook runners check this variable and bypass recursive inspection.
  Furthermore, `HookAction::Command` supports a default 10-second timeout.

### Hazard 5: Timeout failure behavior (fail-open vs fail-closed)

- **Description**:
  A hook command times out or hangs indefinitely.
- **Attack Vector**:
  An attacker induces latency in a hook script.
  The agent harness either hangs forever or defaults to approving the tool call.
- **Mitigation**:
  `HookDefinition` specifies `fail_closed: bool` (defaulting to `true`).
  If the hook execution times out, the runner halts the subprocess and returns a deny decision.

### Hazard 6: Platform path separators and Windows binary resolution

- **Description**:
  Hook manifests contain hardcoded executable paths that fail across operating systems.
- **Attack Vector**:
  Emitting Unix paths (`rulette`) on Windows leads to shell invocation errors.
- **Mitigation**:
  `build_hook_command` in `src/emitters/hooks.rs` checks target platform conventions and appends `.exe` on Windows.
