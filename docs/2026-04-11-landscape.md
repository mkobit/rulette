# Landscape analysis

2026-04-11

## The problem

Every AI coding assistant has invented its own rule format.
Claude Code uses `CLAUDE.md` and skills with TOML frontmatter.
Cursor uses `.cursorrules` (deprecated) and `.cursor/rules/*.mdc` with YAML frontmatter.
Windsurf uses `.windsurfrules`.
GitHub Copilot uses `.github/copilot-instructions.md`.
OpenAI Codex uses `AGENTS.md` with hierarchical directory scoping.
Gemini Code Assist uses `GEMINI.md` and `.gemini/` with slash command configuration.
The Agent Skills format (agentskills.io) uses `SKILL.md` with YAML frontmatter, adopted by 20+ tools.

Organizations and individuals who maintain coding standards, security policies, or workflow automations must duplicate this knowledge across every format.
When a new tool appears (and they appear monthly), the duplication grows.

Worse: the ecosystem for sharing and distributing these rules has zero security infrastructure.
Cursor rules are copy-pasted from `cursor.directory` with no integrity verification.
Vercel's skills library (`npx skills add`) fetches latest from GitHub with no pinning, no checksums, no signatures.
Rulesync reads from a config file and writes to multiple formats but offers no supply chain guarantees.
Every tool in this space is a Node.js script that `npm install`s at runtime, fetches mutable refs, and trusts the network.

For security-conscious organizations — anyone with a compliance requirement, an air-gapped build, or a policy against running arbitrary npm scripts in CI — the current ecosystem is unusable.

## Agent Skills format (agentskills.io)

Agent Skills is an open format (originally from Anthropic, now community-governed at agentskills.io) for packaging reusable AI coding agent instructions.
Adopted by 20+ tools: Claude Code, Cursor, GitHub Copilot, Gemini CLI, OpenAI Codex, Kiro, Goose, Roo Code, and others.

- **Format**: Each skill is a directory with a required `SKILL.md` (YAML frontmatter + markdown body), optional `scripts/`, `references/`, and `assets/` directories. Frontmatter fields: `name` (required, lowercase-hyphenated, max 64 chars), `description` (required, max 1024 chars), `license`, `compatibility`, `metadata` (arbitrary kv), `allowed-tools` (experimental). Body under 500 lines recommended. Progressive disclosure: metadata loads at startup (~100 tokens), full instructions on activation, reference files on demand.
- **Distribution**: `npx skills add vercel-labs/agent-skills` clones from GitHub at HEAD. A reference validator exists at `skills-ref validate ./my-skill`.
- **Strengths**: Emerging standard with broad tool adoption. Good structure. Rich content (react-best-practices has 40+ rules across 8 categories). MIT licensed. 24.9k stars on `vercel-labs/agent-skills`.
- **Weaknesses**: No versioning strategy. No integrity verification. No offline mode. JavaScript runtime dependency. Always fetches latest. No way to pin a skill to a known-good state. The `allowed-tools` field has varying support across agents.

## Rulesync (dyoshikawa/rulesync)

Rulesync is a TypeScript CLI (v7.28.0, 995 stars, 3,243 commits, 215 releases) that generates AI tool config files from a unified rule set.

- **Format**: Canonical rules live in `.rulesync/` directory. Configuration in `rulesync.jsonc`. Operates in project mode (writes to repo) and global mode (system-wide rules).
- **Commands**: `rulesync init` (scaffold), `rulesync generate --targets "*" --features "*"` (emit), `rulesync import --targets <tool>` (ingest existing configs), `rulesync fetch` (pull skill packs).
- **Targets (28+)**: Claude Code, Cursor, GitHub Copilot, Windsurf, Gemini CLI, Cline, Kilo Code, Roo Code, Zed, JetBrains Junie, Factory Droid, OpenCode, and more.
- **Features beyond rules**: Ignore files, MCP server configuration, commands, subagents, skills, hooks, permissions. Not all features available for all targets.
- **Distribution**: npm global install, Homebrew, or single-binary install script.
- **Strengths**: Most comprehensive target coverage in the space. Very actively developed. Handles more than just rules (hooks, MCP, permissions). Import capability.
- **Weaknesses**: Node.js/TypeScript runtime. No typed IR — config-to-template generation, not semantic compilation. No supply chain integrity story. No archive/bundle support. Feature support varies by target (documented but complex matrix). `rulesync.jsonc` is the only config format.

## Other tools

- **cursor.directory / pontusab/cursor.directory**: Community hub for sharing Cursor rules. No cross-format conversion. No integrity verification. Known vector for prompt injection.
- **awesome-cursorrules**: Curated rule collections. No format translation.
- No widely-adopted cross-format converter tool exists at meaningful adoption. The space is a clear gap.

## Rulette vs rulesync

Rulesync is the closest existing tool and the most direct comparison.

| Dimension | Rulesync | Rulette |
| --- | --- | --- |
| Architecture | Config-to-template generator | Compiler with typed IR |
| Runtime | Node.js / TypeScript | Static binary, zero deps |
| Config format | `rulesync.jsonc` | None (CLI args + pipes) |
| Transform model | Template expansion | Semantic IR transforms + shell pipelines |
| Supply chain | None | Content-addressed, lockfiles, SHA-256 |
| Build system integration | npm scripts | Bazel genrule, Nix, any hermetic build |
| Target coverage | 28+ targets | Fewer initially, but additive backends |
| Feature scope | Rules + hooks + MCP + permissions | Rules and skills only (Unix: do one thing well) |
| Composability | Standalone tool | Pipes with jq, markdownlint, grep, etc. |
| State | `.rulesync/` directory, `rulesync.jsonc` | Stateless: no config, no local dirs |

Rulesync wins on breadth of target coverage and feature scope today.
Rulette wins on determinism, security, build system integration, and composability.
These are complementary positions — Rulette could even consume rulesync-generated rules as input.

## Common gaps across all existing tools

| Gap | Impact |
| --- | --- |
| No content-addressable storage | Cannot verify rules haven't been tampered with |
| No pinned versions | Builds are non-reproducible |
| JavaScript runtime required | Cannot run in hermetic build environments (Bazel, Nix) |
| No typed IR | Transformations are string manipulation, not semantic |
| No validation | Malformed rules silently propagate |
| No archive support | Cannot bundle and distribute rule sets atomically |
| No eval/check commands | Cannot verify rules meet quality or policy standards |
| No user-scope management | No dotfiles/chezmoi story for personal rule sets |
