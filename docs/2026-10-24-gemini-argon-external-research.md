# `rulette`: Abbreviated Design & Scoping Spec (Agent Handoff)

## 1. Agent Instructions & Objective

You are reviewing the `rulette` repository. **`rulette`** is a cross-harness transposition compiler and bundler that lets developers author agentic coding extensions once—Rules, Skills, Subagents, Permissions, Hooks, and Plugins—and compile them into native targets for:

1. **Claude Code CLI** (`claude`) ([Claude Code Docs](https://code.claude.com/docs/en/features-overview))
2. **Cursor** (IDE & `cursor` CLI) ([Cursor Docs](https://cursor.com/docs))
3. **Google Antigravity** (`agy` CLI & IDE) ([Antigravity Docs](https://antigravity.google/docs/cli/features/))
4. **OpenAI Codex CLI** (`codex`) ([Codex CLI Docs](https://learn.chatgpt.com/docs/codex/cli))
5. **OpenCode** (`opencode`, secondary target) ([OpenCode Docs](https://opencode.ai/docs/skills/)) + Open Standards ([Agent Skills](https://agentskills.io/specification), [Agent Plugins 1.0.0](https://agent-plugins.org/specification))

**Your Immediate Task**:

1. Audit the current `rulette` codebase against the **Target Specification Matrix (Section 2)** and **Algebraic Composition Rules (Section 3)**.
2. Output a gap analysis identifying which parsers, Intermediate Representation (IR) types, and target emitters already exist vs. which are missing or broken.
3. Scope and execute an implementation plan following the **Prioritized Roadmap (P0 → P3) in Section 5**.

## 2. Target Specification Matrix (Paths, Schemas & Gotchas)

| Primitive | Claude Code (claude) | Cursor (IDE & cursor CLI) | Google Antigravity (agy CLI & IDE) | OpenAI Codex CLI (codex) | OpenCode (opencode) |
| :---- | :---- | :---- | :---- | :---- | :---- |
| **Skills (Project & `$HOME` Paths)** | `.claude/skills/<id>/SKILL.md`; `~/.claude/skills/<id>/SKILL.md` ([Claude Skills](https://code.claude.com/docs/en/skills)). *Does NOT read `.agents/skills/`.* | `.agents/skills/<id>/SKILL.md` or `.cursor/skills/`; `~/.cursor/skills/` or `~/.agents/skills/` ([Cursor Skills](https://cursor.com/docs/skills)). | `.agents/skills/<id>/SKILL.md`; CLI: `~/.gemini/antigravity-cli/skills/`; IDE: `~/.gemini/config/skills/` ([Antigravity Skills](https://antigravity.google/docs/skills/)). | `.agents/skills/<id>/SKILL.md` (scans `$CWD` to root); `~/.agents/skills/<id>/SKILL.md` ([Codex Skills](https://learn.chatgpt.com/docs/build-skills)). | `.agents/skills/<id>/SKILL.md or .opencode/skills/; ~/.config/opencode/skills/ (OpenCode Skills).` |
| **Skill Frontmatter & Sidecars** | `name`, `description`, `allowed-tools` (`Bash(git *)`), `paths`, `disable-model-invocation`, `user-invocable`, `context: fork`, `agent` ([Skills Guide](https://hidekazu-konishi.com/entry/claude_code_skills_complete_guide.html)). | `name`, `description`, `paths` (or `globs`), `disable-model-invocation`, `icon`, `color`, `metadata` ([Cursor Skills](https://cursor.com/docs/skills)). | Strictly `name` (optional) and `description` (required). Auto-exposes `/<id>` slash command ([Antigravity Skills](https://antigravity.google/docs/skills/)). | `SKILL.md` (`name`, `description`) + optional `agents/openai.yaml` (`interface`, `policy.allow_implicit_invocation`, `dependencies.tools`) ([Codex Skills](https://learn.chatgpt.com/docs/build-skills)). | `name, description, metadata: { "opencode/slash": "true", "opencode/autoinvoke": "false" } (OpenCode Skills).` |
| **Always-On Rules** | `CLAUDE.md` (use `@AGENTS.md` import bridge so both load) ([Claude Memory](https://code.claude.com/docs/en/memory)). | `AGENTS.md` (root & nested subdirectories) ([Cursor Rules](https://cursor.com/docs/rules)). | `AGENTS.md` or `GEMINI.md` (plain Markdown, no frontmatter) ([Antigravity Rules](https://antigravity.google/docs/rules/)). | `AGENTS.md` / `AGENTS.override.md` (root, nested, and `~/.codex/`) ([Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)). | `AGENTS.md or opencode.json instructions (OpenCode Agents).` |
| **Scoped / Conditional Rules** | `.claude/rules/<id>.md` (recursive). Frontmatter: `paths: ["src/**/*.ts"]` ([Claude Memory](https://code.claude.com/docs/en/memory)). | `.cursor/rules/<id>.mdc` (**must** use `.mdc`; `.md` is ignored!). Frontmatter: `description`, `globs`, `alwaysApply: false` ([Cursor Rules](https://cursor.com/docs/rules)). | `.agents/rules/<id>.md` (**flat** dir only unless in `.agents/rules.json`). **Must** have `trigger: always_on \| model_decision \| glob \| manual`, `description`, `globs` (camelCase is silently dropped!) ([Antigravity Rules](https://antigravity.google/docs/rules/)). | *No glob rule files* (`.codex/rules/` is for Starlark command security). Lower subtree globs to `<subdir>/AGENTS.md` or demote to Skill ([Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)). | `AGENTS.md or opencode.json instructions (OpenCode Agents).` |
| **Subagents (Custom Agents)** | `.claude/agents/<id>.md`. Keys: `name`, `description`, `tools` (`Read, Grep, Glob, Bash`), `disallowedTools`, `model`, `isolation: worktree`, `skills` ([Claude Subagents](https://code.claude.com/docs/en/sub-agents)). | `.cursor/agents/<id>.md`. Keys: `name`, `description`, `model`, `readonly: bool`, `is_background: bool` ([Cursor Subagents](https://cursor.com/docs/subagents)). | `.agents/agents/<id>.md`. Keys: `name`, `description`, `tools` (**must** map to exact names: `view_file`, `replace_file_content`, `grep_search`, `run_command` or process hangs!), `subagent: true`, `mainAgent: false`, `model: inherit\|flash\|pro`, `commandExecutionPolicy: sandbox` ([Antigravity Subagents](https://antigravity.google/docs/subagents/)). | **TOML file**: `.codex/agents/<id>.toml`. Keys: `name`, `description`, `developer_instructions = """..."""`, `model`, `model_reasoning_effort`, `[features] shell_tool = bool` ([Codex Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)). | `.opencode/agents/<id>.md. Keys: description, mode: subagent, model, permission: { edit: deny, shell: ask } (OpenCode Agents).` |
| **Static Permission Configs** | `.claude/settings.json` (`permissions.allow` / `ask` / `deny`: `Bash(npm test *)`, `WebFetch(domain:...)`) ([Claude Permissions](https://code.claude.com/docs/en/permissions)). | IDE: `.cursor/permissions.json` (`terminalAllowlist: ["git status"]`, `mcpAllowlist`) ([Cursor Permissions](https://cursor.com/docs/reference/permissions)). CLI: `.cursor/cli.json` (`Shell(git:status*)`, `Read(...)`, `Write(...)`, `Mcp(...)`) ([Cursor CLI Permissions](https://cursor.com/docs/cli/reference/permissions)). | `~/.gemini/antigravity-cli/settings.json` (`permissions.allow` / `ask` / `deny`: `command(git status)`, `command(regex:npm run (test\|lint))`, `read_file(...)`, `write_file(...)`, `read_url(...)`, `mcp(srv/tool)`) ([Antigravity Permissions](https://antigravity.google/docs/permissions/)). | Starlark `.codex/rules/default.rules`: `prefix_rule(pattern=["npm", "run", ["test", "lint"]], decision="allow"\|"prompt"\|"forbidden", justification="...")` ([Codex Rules](https://learn.chatgpt.com/docs/agent-configuration/rules)). | `opencode.json permission.shell: ordered map ("*": "ask" first, allow second, deny last) (OpenCode Agents).` |
| **Plugin & Marketplace Files** | `.claude-plugin/plugin.json` + `.claude-plugin/marketplace.json` + `.mcp.json` + `hooks/hooks.json` ([Claude Manifest Ref](https://code.claude.com/docs/en/plugins/manifest-reference)). | `.cursor-plugin/plugin.json` + `.cursor-plugin/marketplace.json` + `mcp.json` + `hooks/hooks.json` ([Cursor Plugins Ref](https://cursor.com/docs/reference/plugins)). | Root `plugin.json` (**strict**: only `$schema`, `name`, `description`; `additionalProperties: false`!) + `mcp_config.json` + root `hooks.json` ([Antigravity Plugins](https://antigravity.google/docs/plugins/)). | Root `plugin.json` ([Agent Plugins 1.0](https://agent-plugins.org/specification)) + `.codex-plugin/plugin.json` + `.agents/plugins/marketplace.json` + `mcp.json` ([Codex Plugins](https://learn.chatgpt.com/plugins/build/plugins)). | `.opencode/plugins/*.ts or npm array in opencode.json (OpenCode Plugins).` |

## 3. Algebraic Composition & Lossiness Rules for `rulette`

### A. Permission Join-Semilattice (Deny ≻ Ask ≻ Allow)

* **Commutative Lattice Targets (Claude, Cursor, Antigravity, Codex)**: Evaluate permissions over $\mathcal{D} = \{\text{Allow}, \text{Ask}, \text{Deny}\}$ via $d_1 \sqcup d_2 = \max_{\prec}(d_1, d_2)$ where $\text{Allow} \prec \text{Ask} \prec \text{Deny}$. Order within a file does not matter; combining rule bundles $R_1 \cup R_2$ is associative, commutative, and idempotent.
* **Non-Commutative Right-Zero Semigroup Target (OpenCode)**: OpenCode evaluates `permission.shell` in order where the **last matching key wins** ($d_1 \triangleright d_2 = d_2$) ([OpenCode Agents](https://opencode.ai/docs/agents/)). `rulette` must topologically sort emitted keys:
  1. `"*": "ask"` (baseline default)
  2. All `allow` rules
  3. All explicit `ask` rules
  4. All `deny` rules last (guaranteeing $d_{\text{allow}} \triangleright d_{\text{deny}} = d_{\text{deny}}$).

### B. Guarded Rule Projections (Handling Missing Glob Support in Codex)

Model every rule as $(g, m)$ where $g \in \{\text{Always}, \text{Glob}(p), \text{ModelDecision}(d), \text{Manual}\}$ and $m$ is Markdown:

* **Claude $\leftrightarrow$ Cursor $\leftrightarrow$ Antigravity**: Bijective mapping between Claude `paths: [p]`, Cursor `.mdc` `globs: p, alwaysApply: false`, and Antigravity `trigger: glob, globs: p`.
* **Codex Projection**: Compute the Longest Common Directory Prefix $\text{LCDP}(p)$:
  * If $p$ is a subtree glob (`src/api/**`), emit into `src/api/AGENTS.md` (**Tier 1: Lossless**).
  * If $p$ is an extension glob across directories (`**/*.test.ts`), either emit a model-invoked Skill in `.agents/skills/` (**Tier 2: Semantic Approximation**) or inline into root `AGENTS.md` under a scoped heading (**Tier 2: Token Over-Approximation**) and log a build diagnostic.

### C. Zero-Mutation Command Approval via Polyglot `PreToolUse` Hook Shim

Only Claude Code honors `allowed-tools: Bash(...)` inside `SKILL.md` ([Claude Skills](https://code.claude.com/docs/en/skills)). To avoid mutating user dotfiles (`~/.gemini/antigravity-cli/settings.json`, `~/.cursor/permissions.json`), `rulette` should compile `allowed_commands` into a Polyglot Hook Script (`rulette-guard.py`) wired into plugin/repo `hooks.json` across all harnesses:

| Harness | Hook Config File | Event & Matcher | stdout JSON Emitted by rulette-guard.py on Match |
| :---- | :---- | :---- | :---- |
| **Antigravity** ([Antigravity Hooks](https://antigravity.google/docs/hooks/)) | `hooks.json` (plugin root) or `.agents/hooks.json` | `PreToolUse` (`matcher: "run_command"`) | `{"decision": "auto_approve", "reason": "rulette allowlist", "permissionOverrides": ["command(<cmd>)"]}` |
| **Cursor** ([Cursor Hooks](https://cursor.com/docs/hooks)) | `hooks/hooks.json` or `.cursor/hooks.json` | `beforeShellExecution` or `preToolUse` (`matcher: "Shell"`) | `{"permission": "allow"}` (exit `0`; exit `2` or `"deny"` to block) |
| **Claude Code** ([Claude Hooks](https://code.claude.com/docs/en/hooks)) | `hooks/hooks.json` or `.claude/settings.json` | `PreToolUse` (`matcher: "Bash"`) | `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow"}}` |
| **Codex CLI** ([Codex Hooks](https://learn.chatgpt.com/docs/hooks)) | `hooks/hooks.json` or `.codex/hooks.json` | `PreToolUse` (`matcher: "^Bash$"`) | `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow"}}` (requires 1x `/hooks` trust) |

`rulette-guard.py` inspects the incoming `stdin` JSON keys (`toolCall.args.CommandLine` $\to$ Antigravity; `hook_event_name == "beforeShellExecution"` $\to$ Cursor; `tool_input.command` $\to$ Claude/Codex), parses compound shell operators (`&&`, `||`, `;`, `|`), verifies every subcommand against the compiled prefix list, and prints the matching harness JSON.

### D. Four-Tier Compiler Diagnostic Output

Every rulette build run must report a per-target fidelity summary:

* **Tier 0 (Zero-Copy)**: `SKILL.md` (`name`, `description`, body), `AGENTS.md`, `scripts/`.
* **Tier 1 (Lossless AST Lowering)**: Subagent `.md` $\leftrightarrow$ Codex `.toml`; `.mdc` $\leftrightarrow$ `trigger: glob` $\leftrightarrow$ `paths:`; Permission semilattice $\leftrightarrow$ Starlark `prefix_rule()` / Antigravity `action(target)` / OpenCode sorted map.
* **Tier 2 (Runtime Hook / Skill Emulation)**: `allowed-tools` $\to$ Polyglot `PreToolUse` hook shim; Codex extension globs $\to$ Skill or `AGENTS.md` section.
* **Tier 3 (Lossy / Unrepresentable — Emit Warning)**: Claude Code in-process JS `Mods` (`hooks/register.js` `ui.render`) ([Claude Mods](https://code.claude.com/docs/en/plugins/mods/overview)), Claude `` !`cmd` `` prompt pre-execution, Antigravity 10-level subagent peer messaging.

## 4. Target Canonical IR Schema (TypeScript / Rust / Python Reference)

```yaml
# rulette.yaml — Canonical IR Input Schema
name: my-team-kit
version: 1.0.0
description: Shared rules, skills, subagents, and command policies.

rules:
  - id: global-style
    trigger: always_on          # always_on | glob | model_decision | manual
    body_file: ./rules/global.md
  - id: api-zod
    trigger: glob
    description: Validate API inputs with Zod schemas.
    globs: ["src/api/**/*.ts"]
    body_file: ./rules/api-zod.md

skills:
  - id: pr-check
    description: Run preflight tests and inspect git diffs before opening a PR.
    allow_implicit_invocation: true
    user_invocable: true
    paths: ["src/**"]
    allowed_commands:
      - ["git", ["status", "diff", "log"]]
      - ["npm", "run", ["test", "lint"]]
    body_file: ./skills/pr-check/BODY.md

agents:
  - id: security-auditor
    description: Read-only security reviewer for auth and injection flaws.
    readonly: true
    background: true
    isolation: worktree
    models:
      claude: opus
      cursor: "claude-opus-5[effort=high]"
      antigravity: pro
      codex: { model: "gpt-6.1-sol", reasoning_effort: "high" }
      opencode: "anthropic/claude-sonnet-4-20250514"
    capabilities: [file_read, code_search, shell_exec] # mapped to native tool names per harness
    prompt_file: ./agents/security-auditor.md

permissions:
  mode: both                    # "hook_shim" (zero-mutation) | "static_config" | "both"
  commands:
    allow:
      - prefix: ["git", ["status", "diff", "log"]]
        justification: Read-only git inspection
      - prefix: ["npm", "run", ["test", "lint", "build"]]
        justification: Standard verification scripts
    ask:
      - prefix: ["git", "push"]
    deny:
      - prefix: ["rm", "-rf"]
      - prefix: ["sudo"]
```

## 5. Prioritized Implementation Roadmap for the Coding Agent

When scoping work against the current `rulette` codebase, prioritize in this exact order:

### P0 — Core IR, Rules Lowering & Shared `.agents/skills/` Emitter (Highest ROI, Lowest Friction)

1. Implement/verify the Canonical IR parser (`Rule`, `Skill`, `Subagent`, `PermissionPolicy`).
2. Emit `AGENTS.md` + `CLAUDE.md` (`@AGENTS.md` bridge) for `always_on` rules.
3. Emit scoped rules to `.claude/rules/<id>.md` (`paths:`), `.cursor/rules/<id>.mdc` (`globs:` + `alwaysApply: false`), and flat `.agents/rules/<id>.md` (`trigger: glob|model_decision|always_on|manual`), plus $\text{LCDP}$ subdirectory `AGENTS.md` projection for Codex.
4. Emit portable skills to `.agents/skills/<id>/SKILL.md` (+ `agents/openai.yaml` sidecar for Codex) and enriched `.claude/skills/<id>/SKILL.md` (`allowed-tools`, `paths`).

### P1 — Permission Semilattice & Zero-Mutation Polyglot Hook Shim

1. Build the prefix-expansion and join-semilattice normalizer ($\text{Deny} \succ \text{Ask} \succ \text{Allow}$).
2. Generate the zero-mutation `rulette-guard.py` polyglot hook script + `hooks.json` / `hooks/hooks.json` wiring for Antigravity (`auto_approve`), Cursor (`permission: allow`), Claude Code (`permissionDecision: allow`), and Codex.
3. Generate static permission configs for workspace/home mode: `.claude/settings.json`, `.cursor/permissions.json` + `.cursor/cli.json`, `~/.gemini/antigravity-cli/settings.json`, `.codex/rules/default.rules` (Starlark `prefix_rule`), and topologically sorted `opencode.json`.

### P2 — Cross-Harness Subagent Compiler

1. Map abstract capability enums (`file_read`, `code_search`, `file_edit`, `shell_exec`) to exact per-harness tool lists (preventing Antigravity's unmapped-tool hang bug).
2. Emit `.claude/agents/<id>.md`, `.cursor/agents/<id>.md`, `.agents/agents/<id>.md`, `.opencode/agents/<id>.md`, and Codex `.codex/agents/<id>.toml`.

### P3 — Polyglot Plugin & Multi-Marketplace Bundler

1. Emit unified plugin directory containing strict root `plugin.json` (Antigravity-safe: only `$schema`, `name`, `description`) alongside `.claude-plugin/plugin.json`, `.cursor-plugin/plugin.json`, and `.codex-plugin/plugin.json`.
2. Emit all three root marketplace catalogs (`.claude-plugin/marketplace.json`, `.cursor-plugin/marketplace.json`, `.agents/plugins/marketplace.json`) and print the Tier 0–3 fidelity report.
