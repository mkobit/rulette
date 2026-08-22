## Why

OpenCode is one of the five core tools in the modern AI development ecosystem alongside Codex, Claude, Cursor, and Antigravity.
Previously, Rulette lacked an OpenCode parser frontend and emitter backend, preventing users from translating rules, agents, skills, and MCP configurations to and from OpenCode workspaces.
This change introduces OpenCode format support, including rule parsing and emission, agent definition translation, skill emission, and MCP server configuration, tracked in `rulette-5bk.9`.

## What Changes

- Add `opencode` as a supported input format (`InputFormat::OpenCode`) and output format (`OutputFormat::OpenCode`).
- Implement the OpenCode parser frontend supporting `opencode.json` / `opencode.jsonc` configuration files, `.opencode/agents/*.md` agent definitions, and markdown rules.
- Implement the OpenCode emitter backend supporting rule emission (`<name>.md`), agent definitions (`agents/<name>.md`), skill emission (`skills/<name>/SKILL.md`), and MCP configurations (`opencode.json`).
- Integrate OpenCode into CLI commands (`transform`, `inspect`, format auto-detection, and scaffold conventions).
- Implement capability reporting and strict parity validation for `OpenCodeEmitter`.

## Capabilities

### Modified Capabilities

- `frontends-and-backends`: Support auto-detecting, parsing, inspecting, and emitting OpenCode rules, agent definitions, skills, and configuration files.

## Impact

- `src/cli/formats.rs`: Add `InputFormat::OpenCode` and `OutputFormat::OpenCode`.
- `src/parsers/opencode.rs`: Create OpenCode parser data structures.
- `src/parsers/frontend.rs`: Integrate OpenCode parser and auto-detection.
- `src/emitters/opencode.rs`: Create `OpenCodeEmitter` implementing `Emitter`.
- `src/emitters/mod.rs`: Export `OpenCodeEmitter` and add capability parity tests.
- `src/cli/commands/transform.rs`: Register OpenCode in output targets, `TOOL_PATH_CONVENTIONS`, and emission dispatch.
- `src/cli/commands/inspect.rs`: Register OpenCode in dry-run emissions and `coverage_targets`.
- `openspec/specs/frontends-and-backends/spec.md`: Add requirements for OpenCode parsing and emission.
- Tracking: bead `rulette-5bk.9` (parent `rulette-5bk`).
