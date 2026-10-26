# `rulette`: Deterministic Transposition Architecture & Decisions for Implementation Agent

## 1. Core Engineering Contract: "Bazel-Like" in Behavior, Speed & Correctness (Not Starlark Syntax)

`rulette` is a **single static binary with zero runtime dependencies** (no Python, Node, or external shell interpreters required). When we describe `rulette` as "Bazel-like," we mean its **execution semantics and correctness guarantees**, not a Starlark configuration language:

1. **Hermeticity & Content-Addressed Action Graph**:
   1. Every transposition step is a pure function over content digests:
      *Digest_out = H(Digest_source_tree || Digest_transform_spec || TargetProfileVersion)*
   2. If the input tree hash and transform config hash match the cached digest in `.rulette/cache/` (or `rulette.lock`), `rulette` is a sub-millisecond no-op.
2. **In-Memory Parallel Pipeline (Execution Speed)**:
   1. Source lifting (*M* native formats $\to$ `BundleIR`), functional middleware transforms (filtering, renaming, reference rewriting, redaction, permission clamping), static security linting, and target lowering (`BundleIR` $\to N$ native target trees) execute entirely in memory across parallel worker threads before a single byte is written to disk.
3. **Deterministic, Canonical Serialization**:
   1. All emitted YAML, JSON, TOML, and Starlark (`.codex/rules/*.rules`) outputs sort map keys deterministically, normalize line endings, and strip non-deterministic timestamps so identical inputs produce byte-for-byte identical outputs on every OS.
4. **Transactional Atomicity & Reversibility**:
   1. Nothing is mutated halfway. `rulette` validates all targets and lossiness constraints in memory first; if any target fails a strict check or security assertion, zero files are written. Every applied deployment records an atomic manifest (`rulette.lock` / `~/.rulette/state/ledger.json`) with pre-image hashes and inverse patches so `rulette revert` restores the exact prior state in one step.

## 2. Direct Decisions for Implementation Agent Blockers (`rulette-o7t.*`)

The following architectural decisions resolve the open Beads blockers (`rulette-o7t.2`, `rulette-o7t.3`, `rulette-o7t.5`, `rulette-o7t.6`, `rulette-4dq.4`, and `rulette-5bk.14.1`):

### Decision 1: CLI UX & Developer Workflow (`rulette-o7t.5` & `rulette-4dq.4`)

1. **Q1.1: Is `rulette.yaml` the single authoring source, or does `rulette` transpose directly between native directories (e.g., `.agents/skills` $\to$ `.claude/skills`)?**
   * **Decision: Native-to-Native ($M \to N$) is the primary mechanism; `rulette.yaml` is an optional pipeline & overlay manifest, NOT a mandatory authoring format.**
   * Developers should **not** be forced to rewrite existing skills, rules, or subagents into a proprietary schema. `rulette` ingests native directories directly (e.g., reading `.claude/`, `.cursor/`, `.agents/`, `.codex/`, or a remote 3P GitHub/GHE/GitLab repo) and lifts them into the in-memory `BundleIR`.
   * `rulette.yaml` (or CLI flags) simply declares **where to read from (`sources`)**, **what transforms/checks to apply (`pipeline`)**, and **which targets/scopes to emit (`targets`)**, plus any optional inline overrides.
2. **Q1.2: How does `rulette` interact with third-party skill managers (e.g., `npx skills add <repo>`) that write directly into `.agents/skills`?**
   * **Decision: Treat `.agents/skills` as an upstream input source and never clobber unmanaged directories.**
   * Because [Cursor](https://cursor.com/docs/skills), [Google Antigravity](https://antigravity.google/docs/skills/), [OpenAI Codex CLI](https://learn.chatgpt.com/docs/build-skills), and [OpenCode](https://opencode.ai/docs/skills/) all read `.agents/skills/<id>/SKILL.md` natively while [Claude Code](https://code.claude.com/docs/en/skills) reads `.claude/skills/<id>/SKILL.md`, a user who runs `skills add <repo>` ends up with skills in `.agents/skills/` that Claude Code cannot see (and without Codex's `agents/openai.yaml` sidecar).
   * When `.agents/skills` is configured as a source, `rulette` reads the third-party skills dropped there, runs its sanitization/security/transform pipeline, and syncs the derived outputs into `.claude/skills/` (and `agents/openai.yaml` if needed), tracking only `rulette`-emitted files in `.rulette/ledger.json` so third-party managers and `rulette` compose cleanly.
3. **Q1.3: What are the primary CLI verbs?**
   * **Decision: Unify around a single compiler-pipeline verb set that works both with `rulette.yaml` and ad-hoc CLI flags:**
   * `rulette sync` (or `rulette build`): Reads configured `sources` (from `rulette.yaml` or `--from <src>`), runs transforms and checks, and writes target outputs (to `--to <targets>` and `--scope repo|user|plugin`).
   * `rulette plan` (or `rulette diff --dry-run`): Computes the full in-memory transposition, prints the **Lossiness & Fidelity Report** and the exact file/AST diff, and exits without touching disk.
   * `rulette check` (or `rulette test`): Runs schema lints, security assertions (forbidding unbounded `Bash(*)` or dangerous script patterns), and max-lossiness tier checks for CI gates.
   * `rulette revert` (or `rulette clean`): Uses `.rulette/ledger.json` to atomically remove all `rulette`-generated files/symlinks and invert any managed config patches.
4. **Q1.4: How does the developer inspect lossy transformations before writing files to disk?**
   * **Decision: `rulette plan` (and `rulette sync --dry-run`) prints a deterministic Lossiness Table and Unified Diff before any disk write.**
   * Every lowered attribute is tagged with a fidelity tier (`Tier 0: Exact`, `Tier 1: Syntactic Lowering`, `Tier 2: Behavioral Approximation/Shim`, `Tier 3: Dropped/Lossy`). In `--strict` mode (or when `max_lossiness_tier` is exceeded), `rulette` aborts with a non-zero exit code before writing any files.

### Decision 2: Runtime Boundary & Zero-Dependency Hook Shim (`rulette-o7t.6` & `rulette-5bk.14.1`)

1. **Q2.1: `rulette` requires a static binary with no runtime dependencies. Does emitting `rulette-guard.py` break environments without Python?**
   * **Decision: NEVER emit a Python script (`rulette-guard.py`). Instead, support two zero-dependency modes:**
   * **Mode A — Self-Invoking Static Binary Hook (`rulette hook-eval`)**: When `rulette` is installed on the machine (`--scope user` or local dev with `rulette` on `PATH`), `hooks.json` invokes the **`rulette` static binary itself**:

     ```json
     {
       "command": "rulette hook-eval --policy .rulette/policy.json"
     }
     ```

     Because `rulette` is a compiled static binary, `rulette hook-eval` starts in `<1ms`, requires zero Python/Node/Bash dependencies, reads the harness's `stdin` JSON (`toolCall.args.CommandLine` on [Antigravity](https://antigravity.google/docs/hooks/), `command` on [Cursor](https://cursor.com/docs/hooks), `tool_input.command` on [Claude Code](https://code.claude.com/docs/en/hooks) and [Codex](https://learn.chatgpt.com/docs/hooks)), evaluates the exact same compiled permission semilattice (`rulette-o7t.3`), and prints the target-native approval JSON (`{"decision": "auto_approve"}` for Antigravity, `{"permission": "allow"}` for Cursor, `{"hookSpecificOutput": {"permissionDecision": "allow"}}` for Claude/Codex).

   * **Mode B — Static Config Emission (`permission_mode = "static"`)**: When exporting a zero-dependency repo or plugin for teammates who do *not* have the `rulette` binary installed on their machines, `rulette` emits native static permission configs (`.claude/settings.json`, `.cursor/permissions.json` + `.cursor/cli.json`, Antigravity `settings.json`, Codex Starlark `.codex/rules/default.rules`, and topologically sorted `opencode.json`).

1. **Q2.2: What happens if a target harness does not support hook interception (or when skill-scoped `allowed-tools` is lowered to a target without skill-scoped permissions)?**
   * **Decision: Configurable via `--strict` / `lossiness_policy`:**
     Note that **scope widening** is itself a Tier 2 semantic approximation: in [Claude Code](https://code.claude.com/docs/en/skills), `allowed-tools: Bash(npm test *)` in `SKILL.md` only auto-approves `npm test` *while that skill is active in the turn*, whereas lowering that rule to `.cursor/permissions.json`, Antigravity `settings.json`, Codex `default.rules`, or a global `PreToolUse` hook approves `npm test` *across the entire session*.
     Therefore, when lowering skill-scoped `allowed-tools` to a session-wide static config or hook:
     Under default mode (`max_lossiness_tier = 2`), `rulette` emits the static permission or `rulette hook-eval` entry and logs a **Tier 2 (Scope Widening: Skill $\to$ Session)** fidelity warning in `rulette plan` / `rulette sync`.
     Under `--strict` mode (`max_lossiness_tier = 1`), `rulette` fails fast and refuses to widen a skill-scoped permission into a session-wide permission unless the user explicitly promotes the rule to the bundle's top-level `permissions` block.

### Decision 3: Long-Term Ecosystem Positioning

1. **Q3.1: Since Cursor, Antigravity, and Codex already read `.agents/skills/<id>/SKILL.md`, is `rulette` a skill distributor, or is the core moat in rules, subagents, permissions, and transforms?**
   * **Decision: The core moat is the Transposition, Security Policy & Functional Transformation Engine across Rules, Subagents, Permissions, Hooks, and Plugins—NOT basic `SKILL.md` file copying.**
   * Basic `SKILL.md` distribution is becoming a commodity (though Claude Code still requires `.claude/skills/` and Codex uses `agents/openai.yaml` sidecars).
   * `rulette`'s defensible value is everything that breaks across harnesses or requires supply-chain hygiene:
     * **Cross-Harness Rules**: Bridging `.cursor/rules/*.mdc` ([Cursor Rules](https://cursor.com/docs/rules)), `.claude/rules/*.md` (`paths:`) ([Claude Memory](https://code.claude.com/docs/en/memory)), flat `.agents/rules/*.md` (`trigger:`) ([Antigravity Rules](https://antigravity.google/docs/rules/)), and `<subdir>/AGENTS.md` ([Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)).
     * **Cross-Harness Subagents**: Bridging Markdown subagents with incompatible frontmatter and tool names ([Claude Subagents](https://code.claude.com/docs/en/sub-agents), [Cursor Subagents](https://cursor.com/docs/subagents), [Antigravity Subagents](https://antigravity.google/docs/subagents/)) and TOML subagents ([Codex Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)).
     * **Unified Permission Algebra**: Compiling one permission lattice (`rulette-o7t.3`) into Claude `settings.json`, Cursor `permissions.json` + `cli.json`, Antigravity `action(target)`, Codex Starlark `prefix_rule()`, OpenCode sorted maps, and `rulette hook-eval`.
     * **Supply-Chain Sanitization Middleware (`rulette-4dq.4`)**: Taking an untrusted or external 3P skill/plugin and deterministically filtering, renaming, redacting, and stripping dangerous shell injections before it ever touches a developer's machine.

1. **Q3.2: How should `rulette` relate to open standards like Agent Skills and Agent Plugins 1.0.0?**
   * **Decision: Treat open standards as Tier-0 First-Class Citizens (both as input lifters and output targets), while positioning `rulette` as the Superset Polyglot Compiler that bridges what the standards leave out.**
   * Every `rulette` plugin build should emit a standards-compliant [Agent Plugins 1.0.0](https://agent-plugins.org/specification) layout alongside the vendor-specific manifests (`.claude-plugin/plugin.json`, `.cursor-plugin/plugin.json`, `.codex-plugin/plugin.json`, and Antigravity's strict root `plugin.json`).

### Decision 4: Resilient Subagent Capability Abstraction

1. **Q4.1: Harnesses use disparate, evolving tool names (e.g., Antigravity hangs if `tools:` contains an unmapped tool identifier). How should abstract capabilities (`file_read`, `code_search`, `shell_exec`) stay resilient when host platforms update or deprecate internal tool names?**
   * **Decision: Use a 3-Layer Resilient Capability Resolution Table with Embeddable + Overridable Target Profiles:**
     * **Versioned Embedded Target Registry (`targets.Lock` / Embedded JSON)**: Rather than hardcoding tool names scattered across Rust/Go emitter code, ship an embedded, versioned data table (`profiles/targets.v1.json`) inside the static binary that maps `(harness, harness_version_range, capability) -> [native_tool_names]`:
       * `claude`: `file_read -> ["Read"]`, `code_search -> ["Grep", "Glob"]`, `file_edit -> ["Write", "Edit"]`, `shell_exec -> ["Bash"]` ([Claude Subagents](https://code.claude.com/docs/en/sub-agents))
       * `antigravity`: `file_read -> ["view_file", "list_dir"]`, `code_search -> ["grep_search", "find_by_name"]`, `file_edit -> ["replace_file_content", "multi_replace_file_content", "write_to_file"]`, `shell_exec -> ["run_command"]` ([Antigravity Hooks & Tools](https://antigravity.google/docs/hooks/))
     * **Coarse-Grained Safe Fallback When Available**: Where a harness supports coarse capability flags that don't depend on brittle internal tool names—such as **Cursor's** `readonly: true` ([Cursor Subagents](https://cursor.com/docs/subagents)), **Codex's** `[features] shell_tool = false` ([Codex Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)), and **OpenCode's** `permission: { edit: "deny", shell: "ask" }` ([OpenCode Agents](https://opencode.ai/docs/agents/))—emit those coarse flags instead of enumerating fragile per-tool names! Only emit explicit tool lists on harnesses that require them (Claude Code and Antigravity).
     * **User-Overridable Escape Hatch (`target_overrides` in `rulette.yaml`)**: Allow users to override or extend the tool mapping table in `rulette.yaml` (e.g., `targets.antigravity.tool_map.file_read = ["view_file"]`) or pass raw passthrough tool names (`raw_tools.antigravity = [...]`) so a host platform tool rename never requires waiting for a new `rulette` binary release.

## 3. Summary of Unblocked Beads Tasks & Execution Order

| Beads Task ID | Resolution Summary | Next Action for Implementation Agent |
| :---- | :---- | :---- |
| **`rulette-o7t.5`** *(CLI UX Contract)* | Unified around native-to-native $M \to N$ transposition (`rulette sync`, `rulette plan`, `rulette check`, `rulette revert`) where `rulette.yaml` is an optional pipeline/overlay config and `.agents/skills` can act as an upstream input. | Implement CLI argument & config loader supporting both `rulette.yaml` and ad-hoc `--from` / `--to` flags. |
| **`rulette-o7t.2`** $\to$ **`rulette-4dq.4`** *(API Reshaping & Transforms)* | Pure in-memory middleware chain `Vec<Box<dyn Transform>>` operating on `BundleIR` (`Filter`, `RenameAndRewriteRefs`, `RedactRegex`, `ClampPermissions`, `MapModels`) with deterministic content hashing. | Reshape IR pipeline API in `rulette-o7t.2`, then implement declarative transform steps in `rulette-4dq.4`. |
| **`rulette-o7t.6`** & **`rulette-o7t.3`** $\to$ **`rulette-5bk.14.1`** *(Runtime Boundary, Semilattice & Skill Command Approval)* | Dropped `rulette-guard.py`. Use **static config emission** by default for zero-dependency portability + **`rulette hook-eval`** (self-invoking static binary subcommand) for zero-mutation hook mode, backed by the $\text{Deny} \succ \text{Ask} \succ \text{Allow}$ join-semilattice and Tier 2 scope-widening diagnostics. | Implement the permission semilattice normalizer (`rulette-o7t.3`), add `rulette hook-eval` + static emitters (`rulette-o7t.6`), and wire skill `allowed-tools` lowering (`rulette-5bk.14.1`). |
