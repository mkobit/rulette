# Rulette vs Vercel Labs' Agent Skills

This document provides a comparison between Rulette and Vercel Labs' [Agent Skills](https://agentskills.io/) ecosystem (`vercel-labs/agent-skills`). While both projects aim to solve the problem of sharing and standardizing AI coding assistant instructions, they operate at fundamentally different layers of the stack and have different architectural philosophies.

## Overview

- **Agent Skills** is an **open format specification** and a **content repository**. It defines a specific folder structure (with a required `SKILL.md` containing YAML frontmatter) and provides a central GitHub repository where the community can share these skills. It relies on an npm package (`npx skills add`) to fetch these skills directly from GitHub.
- **Rulette** is a **format-agnostic compiler and build tool**. It is not a format, nor is it a repository of content. Instead, it is a static binary that parses *any* existing format (including Agent Skills, Claude configs, Cursor rules, etc.) into a unified Internal Representation (IR), allowing you to transform and compile them deterministically into whichever format your AI tools require.

## Comparison matrix

| Dimension | Agent Skills (`vercel-labs`) | Rulette |
| --- | --- | --- |
| **Core identity** | Content format & community repository | Format compiler & build tool pipeline |
| **Runtime** | Node.js (via `npx`) | Static binary (zero dependencies) |
| **Format approach** | Prescriptive (you must write in `SKILL.md` format) | Agnostic (reads anything, writes to anything) |
| **Distribution** | Fetches latest commit (`HEAD`) from GitHub | Content-addressed, lockfiles, cryptographic verification |
| **Integrity & Security** | None (trusts the network, mutable refs) | Deterministic, reproducible, safe for air-gapped CI/CD |
| **Architecture** | Fetcher script | Semantic IR compiler |
| **Composability** | Standalone script | Standard Unix pipelines (`jq`, `grep`, `markdownlint`) |

## Key differences

### 1. Scope: Format vs. Compiler

Agent Skills introduces *another format* to the ecosystem. If you use tools that natively understand the Agent Skills format, you can use them directly. If not, the format is a silo.
Rulette acts as a translation layer. It recognizes that fragmentation is inevitable and serves as the bridge. You can write your rules in the Agent Skills format and use Rulette to compile them down into Cursor `.mdc` files, Claude `settings.json`, or Copilot instructions.

### 2. Supply chain and Security

The primary distribution mechanism for Agent Skills (`npx skills add`) fetches the latest `HEAD` of a GitHub repository over the network. It offers no version pinning, no checksum verification, and no guarantees against upstream mutations or prompt injection attacks in the shared repository.
Rulette treats AI rules as critical software supply chain components (system prompts). It supports content-addressed storage, lockfiles, and SHA-256 verification, making it suitable for security-conscious organizations and hermetic build systems like Bazel or Nix.

### 3. Execution environment

Agent Skills tooling relies on the Node.js ecosystem. Rulette is distributed as a single, static binary with zero runtime dependencies. It runs seamlessly in minimalistic Docker containers, Alpine environments, and CI pipelines without requiring a heavy language runtime.

## How they work together

Rulette and Agent Skills are highly complementary. They do not need to be mutually exclusive.

**Rulette can use Agent Skills as an input source.**
Because Agent Skills is a well-structured, open format with a rich library of community-contributed content, it serves as an excellent source of truth. You can use Rulette to fetch a specific, cryptographically verified version of an Agent Skill and deterministically compile it into the formats required by your organization's diverse set of AI assistants.

**Example Pipeline:**

1. A developer identifies a useful skill in the `vercel-labs/agent-skills` repository.
2. The organization uses Rulette to securely fetch this skill and pin its exact SHA-256 hash in a lockfile.
3. In a CI/CD pipeline, Rulette parses the Agent Skill, transforms it into a unified IR, and emits it to `CLAUDE.md` for Claude users, `.cursor/rules/` for Cursor users, and `AGENTS.md` for Codex users—ensuring everyone gets the exact same prompt instructions securely.
