# Comparison and support matrix

Rulette is designed as a **hermetic, deterministic compiler** for AI configuration.
This document tracks how Rulette compares to other tools in the ecosystem and the status of its target format support.

## Core philosophy comparison

| Feature | Rulette | Rulesync | Logic |
| :--- | :--- | :--- | :--- |
| **Runtime** | Static Binary (Rust) | Node.js (NPM) | Rulette is designed for Bazel, Buck, and air-gapped CI. |
| **Initialization** | `rulette init` | `rulesync init` | Scaffolds a `rules/` directory and config. |
| **State** | Stateless (Compiler) | Stateful (Sync) | Rulette treats config as code to be compiled, not state to be synced. |
| **Determinism** | Guaranteed | Best Effort | Same input always produces bit-identical output in Rulette. |
| **Transformation** | Pipe-based (Unix) | Internal Plugin System | Rulette integrates with `jq`, `sed`, and custom scripts via IR. |
| **Hermeticity** | Fully Hermetic | Requires Node/Registry | Rulette has zero runtime dependencies. |

## Target support matrix

This matrix tracks the "Big 4" and other significant formats.

### Input formats (frontends)

| Format | Status | Entity support | Notes |
| :--- | :--- | :--- | :--- |
| **Claude Code** | ✅ | Rules, MCP, Hooks, Permissions | Full `settings.json` and `CLAUDE.md` support. |
| **Gemini CLI** | ✅ | Rules, Subagents | Superior mapping for Gemini-specific metadata. |
| **Cursor** | ✅ | Rules (.mdc), MCP | Supports both legacy `.cursorrules` and modern `.mdc`. |
| **Codex** | ✅ | Rules, Agents | Supports `AGENTS.md`. |
| **Agent Skills** | ✅ | Skills | Native support for the `SKILL.md` format. |
| **Archives** | 🚧 | Mixed | `.tar` and `.tar.gz` support in progress. |
| **Cline/Roo Code** | ❌ | - | Planned for v0.1.2. |
| **Windsurf** | ✅ | Rules | Supports `.windsurfrules`. |

### Output formats (backends)

| Format | Status | Lossy? | Notes |
| :--- | :--- | :--- | :--- |
| **Claude Code** | ✅ | No | Reference implementation for IR. |
| **Gemini CLI** | ✅ | No | Preserves subagent metadata. |
| **Cursor** | ✅ | Yes | Drops MCP/Hooks (Cursor uses separate JSON). |
| **Codex** | ✅ | Yes | Drops complex activation logic. |
| **Agent Skills** | ✅ | No | Full fidelity. |
| **IR (JSON/TOML)** | ✅ | No | The ultimate source of truth. |

## Hermeticity and Build System Integration

Rulette is optimized for use in restricted build environments:

1. **Bazel/Buck**: The static binary can be checked into the repo or fetched via `http_archive` with SHA-256 verification.
2. **Deterministic Outputs**: Ensuring that re-running the transformation on the same input produces the exact same files prevents unnecessary build invalidations or git churn.
3. **No Network by Default**: Unlike tools that might fetch "latest rules" or "registry updates," Rulette only interacts with the filesystem (or stdin/stdout) unless explicit flags like `--allow-mutable` are used (planned).
