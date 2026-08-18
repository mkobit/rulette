# Rulette announcement: the hermetic AI rule compiler

## The problem: configuration drift

AI coding assistants are only as good as the context they have. Today, that context is scattered across `.cursorrules`, `CLAUDE.md`, `.windsurfrules`, and custom MCP server configurations.

As teams adopt multiple tools, they face a new challenge: **configuration drift**. A rule added to Cursor isn't available in Claude Code. An MCP server configured in one editor must be manually recreated in another.

## The solution: Rulette

Rulette is a **stateless, deterministic compiler** for AI coding assistant configuration.

It normalizes diverse rules, skills, and tool configurations into a single, typed Intermediate Representation (IR) and emits them to any target format.

### Key features

- **Universal translation**: Convert between Cursor MDC, Claude Code, Agent Skills, Codex, and more.
- **Hermetic & deterministic**: Built in Rust as a static binary with zero runtime dependencies. Perfect for Bazel/Buck build systems and air-gapped CI.
- **Unix philosophy**: Designed for pipelines. Pipe IR through `jq` or `markdownlint` before emitting.
- **Strict integrity**: Fails fast on identity collisions. Ensures that your rule set is unambiguous and reproducible.

### Quick start

```sh
# Convert rules to Claude skills
rulette transform ./rules/ --to claude --out .claude/skills/

# Convert rules to Cursor MDC
rulette transform ./rules/ --to cursor-mdc --out .cursor/rules/
```

## Why Rulette?

Unlike existing sync-based tools, Rulette treats AI configuration as **code to be compiled**.

It doesn't just copy files; it parses them into a semantic model. This allows for powerful transformations:

- **Filter**: Only emit "stable" rules to production.
- **Promote**: Promote a generic rule to a first-class Agent Skill.
- **Merge**: Safely combine rules from multiple sources with strict collision detection.

## v0.1: foundational targets

The initial release focuses on the most critical formats:

- **Claude Code**: Full support for rules, hooks, and MCP servers.
- **Cursor**: Support for modern `.mdc` rules and MCP.
- **Agent Skills**: Native support for the `SKILL.md` standard.
- **Codex**: Support for `AGENTS.md`.
- **Gemini CLI**: Mapping for rules and subagents.

## Roadmap

- **v0.1.1**: Advanced transform pipelines, MCP normalization, and hook taxonomy.
- **v0.2**: Semantic diffing and coverage reporting.

Rulette is open source and available today.
