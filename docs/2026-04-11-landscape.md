# Rulette 0.1 landscape

## Problem

AI-agent harnesses load guidance from different directory layouts, frontmatter formats, and configuration files.
The differences are not only text formatting differences.
They determine package boundaries, resource eligibility, scope, activation, and which native features have no portable meaning.

## Rulette position

Rulette is a local compiler layer between documented harness layouts and reviewed native artifacts.
It does not run agents or route their inference requests.
Its OpenRouter-like role is format portability at the authoring and publication boundary, not model routing or harness execution.

## Core comparison

| Concern | Native harness tooling | Agent Skills ecosystem | Rulette 0.1 |
| --- | --- | --- | --- |
| Primary unit | Tool-specific file or directory | `SKILL.md` directory package | Rule or skill graph package |
| Native semantics | Full tool behavior | Skill format behavior | Portable rules and skills only |
| Opaque resources | Tool-defined | Package-defined | Retained with provenance |
| Native-only settings | Executed or loaded by the tool | Outside the format | Unsupported package with diagnostics |
| Input retrieval | Tool-defined | Often external package tooling | Explicit local inputs only |
| Output authority | Tool-defined writes | Tool-defined installation | Reviewed staging before authorized apply |
| Runtime role | Agent harness | Content format and distribution | Static compiler |

## Design constraints

The compiler operates without network access, dynamic loading, automatic configuration discovery, initialization, or local state.
It preserves information rather than inventing portability.
It fails before staging when a requested target would lose information unless loss is explicitly accepted.

## v0.1 boundary

Codex, Claude, Cursor, OpenCode, and Antigravity are the core harness domains.
Rules and skills are the portable semantic units.
Agents, hooks, MCP servers, permissions, and native configuration remain source-specific package content.

Text reducers, arbitrary rewrites, metadata mutation, merging, deduplication, plugins, registries, and remote retrieval are intentionally outside this release.
They can only enter later through explicit compiler semantics, validation, and loss reporting.
