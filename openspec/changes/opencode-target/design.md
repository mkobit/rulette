## Context

OpenCode is one of the five core AI coding tools supported by Rulette (`docs/2026-08-18-cli-ux-design.md`).
OpenCode organizes project-level configurations under `.opencode/` (or the project root), supporting:

1. Configuration in `opencode.json` / `opencode.jsonc`, defining MCP servers (`mcp`), agent definitions (`agent`), and referenced instructions.
2. Prompt-heavy agent definitions in `.opencode/agents/<name>.md` with YAML frontmatter (`description`, `mode`, `model`, `permission`, etc.) and system prompt markdown bodies.
3. Standard Agent Skills in `.opencode/skills/<name>/SKILL.md` or `skills/<name>/SKILL.md`.
4. Project rules in Markdown files (such as `AGENTS.md` or `.opencode/` rule files).

## Goals / Non-goals

**Goals:**

- Introduce `InputFormat::OpenCode` and `OutputFormat::OpenCode` across the CLI and library.
- Implement OpenCode parser data structures in `src/parsers/opencode.rs` and parsing logic in `src/parsers/frontend.rs`.
- Support parsing `opencode.json` / `opencode.jsonc` (MCP servers and inline agents) and `.opencode/agents/*.md` (agent definitions).
- Implement `OpenCodeEmitter` in `src/emitters/opencode.rs` supporting rules (`<name>.md`), agent definitions (`agents/<name>.md`), skills (`skills/<name>/SKILL.md`), and MCP configurations (`opencode.json`).
- Register OpenCode in `TOOL_PATH_CONVENTIONS` for manifest scaffolding.
- Implement capability reporting in `OpenCodeEmitter::capabilities` with strict parity testing.

**Non-goals:**

- Native hooks execution engine for OpenCode (hooks report `Dropped` in capability reporting and warn/error appropriately).
- Advanced tool permission enforcement engines beyond metadata emission.

## Decisions

**1. Data structures and modeling for OpenCode.**

OpenCode structures are defined in `src/parsers/opencode.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpenCodeConfigFile {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp: HashMap<String, OpenCodeMcpServer>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agent: HashMap<String, OpenCodeAgentConfig>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpenCodeMcpServer {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpenCodeAgentFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
```

**2. Output layout in `OpenCodeEmitter`.**

When emitting to an OpenCode target:

- `Entity::Agent` entities emit to `agents/<name>.md` with YAML frontmatter and system prompt body.
- `Entity::Skill` entities emit to `skills/<name>/SKILL.md`.
- `Entity::McpServer` entities emit to `opencode.json` under `"mcp"`.
- `Entity::Rule` entities emit to `<name>.md` (or `rule_{i}.md`).
- `Entity::Hook` and `Entity::Permissions` entities report `Dropped` in `capabilities()` and error under strict mode.

**3. Auto-detection and path conventions.**

- Input JSON files containing `"$schema": "https://opencode.ai/config.json"` or `"mcp"`/`"agent"` configurations are detected as OpenCode.
- Markdown files in `.opencode/agents/` or containing `mode: subagent` frontmatter are detected as OpenCode agents.
- `TOOL_PATH_CONVENTIONS` in `src/cli/commands/transform.rs` maps `.opencode` path components to `OutputFormat::OpenCode` with default scaffold path `".opencode/"`.

## Adversarial review

**1. JSON vs. Markdown representation of Agents.**

- *Issue*: OpenCode supports defining agents both inline in `opencode.json` under `"agent"` and as standalone markdown files in `.opencode/agents/<name>.md`.
- *Mitigation*: When parsing, both inline JSON agents and Markdown agent files are translated to `Entity::Agent`. When emitting, `OpenCodeEmitter` emits prompt-heavy agents as `agents/<name>.md` to maximize prompt readability, while preserving tool permissions in frontmatter.

**2. MCP server type field defaults.**

- *Issue*: OpenCode's MCP server configuration supports optional `"type": "local"`, whereas IR's `McpServerConfig` has standard `command`, `args`, and `env`.
- *Mitigation*: The emitter includes `"type": "local"` when serializing local command servers in `opencode.json` to ensure compatibility with OpenCode schema requirements.

**3. Ambiguity in auto-detection between Agent Skills and OpenCode Agents.**

- *Issue*: Both OpenCode agents and Agent Skills use YAML frontmatter with `description:`.
- *Mitigation*: OpenCode agent files are distinguished by `mode:` frontmatter, `permission:` frontmatter, or path location (`.opencode/agents/` or `agents/`), whereas Agent Skills have `name:` and `description:` in `SKILL.md`.

**4. Capabilities parity with strict emission.**

- *Issue*: If `capabilities()` reports `Supported` for an entity kind that `emit(strict=true)` rejects, coverage checking fails.
- *Mitigation*: `OpenCodeEmitter` is added to `assert_parity("opencode", &OpenCodeEmitter)` in `src/emitters/mod.rs` to guarantee strict agreement across all six entity kinds.

## Risks / Trade-offs

- **[Trade-off]** Dropping `Hook` and standalone `Permissions` entities in `OpenCodeEmitter`.
  → Mitigation: OpenCode does not have native lifecycle hook scripts like Claude Code; permissions are integrated directly into agent frontmatter and config where supported.
- **[Risk]** Overlapping path conventions with generic markdown files.
  → Mitigation: Path matching checks specific `.opencode` components before generic markdown fallbacks.

## Open Questions

None.
