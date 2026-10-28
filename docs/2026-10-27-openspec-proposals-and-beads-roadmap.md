# OpenSpec proposals and Beads roadmap (`rulette-o7t.4`)

## Summary

This document structures the P0-P3 implementation roadmap from [docs/2026-10-24-gemini-argon-external-research.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-24-gemini-argon-external-research.md).
It aligns OpenSpec change proposals and Beads issues with the canonical intermediate representation paradigm.
It incorporates the matrix audit in [docs/2026-10-26-ir-and-pipeline-matrix-audit.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-26-ir-and-pipeline-matrix-audit.md).
It incorporates the API reshaping decisions in [docs/2026-10-26-api-reshaping-and-declarative-schema-evaluation.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-26-api-reshaping-and-declarative-schema-evaluation.md).
It incorporates the algebraic composition design in [docs/2026-10-26-algebraic-composition-and-permission-semilattice-design.md](file:///home/mkobit/workspace/mkobit/rulette/docs/2026-10-26-algebraic-composition-and-permission-semilattice-design.md).
It follows the workflow in [.beads/formulas/openspec-workflow.formula.toml](file:///home/mkobit/workspace/mkobit/rulette/.beads/formulas/openspec-workflow.formula.toml).

---

## 1. Roadmap decomposition matrix

The table shows the mapping from external research tiers to OpenSpec proposals and Beads issues.

| Tier | Domain | Core capability | Target files and formats | OpenSpec change proposal | Tracking bead |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **P0** | Rules and skills | Typed skill frontmatter, sidecars, and scoped rules | `AGENTS.md`, `CLAUDE.md`, `.claude/rules/*.md`, `.cursor/rules/*.mdc`, `.agents/rules/*.md`, `agents/openai.yaml` | `scoped-rules-and-skill-sidecars` | `rulette-o7t.7` |
| **P0** | Rule bundling | Rule bundling to single-file targets | `AGENTS.md`, `CLAUDE.md` | `rule-bundling-and-marketplace-transforms` (existing) | `rulette-4dq` |
| **P1** | Permissions | Permission join-semilattice and static permission configs | `.claude/settings.json`, `.cursor/permissions.json`, `.cursor/cli.json`, `~/.gemini/antigravity-cli/settings.json`, `.codex/rules/*.rules`, `opencode.json` | `permission-precedence` (existing) | `rulette-5bk.14` |
| **P1** | Hooks | Polyglot PreToolUse hook manifests and static evaluation | `rulette hook-eval`, `.agents/hooks.json`, `.cursor/hooks.json`, `.claude/settings.json`, `.codex/hooks.json` | `polyglot-hook-eval` | Merged in PR #201 and PR #202 |
| **P2** | Subagents | Cross-harness custom agent compilation and capability mappings | `.claude/agents/*.md`, `.cursor/agents/*.md`, `.agents/agents/*.md`, `.opencode/agents/*.md`, `.codex/agents/*.toml` | `cross-harness-subagents` | `rulette-o7t.8` |
| **P3** | Plugins | Polyglot plugin manifests and multi-marketplace catalogs | Root `plugin.json` (Antigravity-safe), `.claude-plugin/plugin.json`, `.cursor-plugin/plugin.json`, `.codex-plugin/plugin.json`, `.claude-plugin/marketplace.json` | `polyglot-plugin-and-marketplaces` | `rulette-o7t.9` |

---

## 2. Phase P0: Scoped rules, rule projections, and skill sidecars

### Scope for P0

1. Add typed skill frontmatter fields to `SemanticItem::Skill` in [src/ir/graph.rs](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs).
2. Store `allowed_tools`, `paths`, and execution policy flags.
3. Emit Codex `agents/openai.yaml` sidecar when lowering skills to Codex targets.
4. Emit scoped rules to `.claude/rules/<id>.md` (`paths:`).
5. Emit scoped rules to `.cursor/rules/<id>.mdc` (`globs:` and `alwaysApply: false`).
6. Emit scoped rules to `.agents/rules/<id>.md` (`trigger: glob`).
7. Emit Longest Common Directory Prefix (LCDP) subdirectory `AGENTS.md` projections for Codex.
8. Emit `@AGENTS.md` import bridge in `CLAUDE.md`.

### Artifacts for P0

- Proposal: `openspec/changes/scoped-rules-and-skill-sidecars/proposal.md`
- Design: `openspec/changes/scoped-rules-and-skill-sidecars/design.md`
- Tasks: `openspec/changes/scoped-rules-and-skill-sidecars/tasks.md`
- Tracking bead: `rulette-o7t.7`

---

## 3. Phase P1: Permission precedence and static configuration emission

### Scope for P1

1. Implement the join-semilattice normalizer (`Deny > Ask > Allow`) in `src/ir/permissions.rs`.
2. Map canonical permissions to target static configuration files.
3. Enforce fail-closed rules when actions or patterns are not representable.
4. Connect `rulette hook-eval` with the in-memory permission policy engine.
5. Provide structured capability findings for permission loss.

### Artifacts for P1

- Proposal: [openspec/changes/permission-precedence/proposal.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/changes/permission-precedence/proposal.md)
- Design: [openspec/changes/permission-precedence/design.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/changes/permission-precedence/design.md)
- Tasks: [openspec/changes/permission-precedence/tasks.md](file:///home/mkobit/workspace/mkobit/rulette/openspec/changes/permission-precedence/tasks.md)
- Tracking bead: `rulette-5bk.14`

---

## 4. Phase P2: Cross-harness subagent compiler

### Scope for P2

1. Add `PackageKind::Agent` and `SemanticItem::Agent` to [src/ir/graph.rs](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs).
2. Define abstract capability enums (`file_read`, `code_search`, `file_edit`, `shell_exec`).
3. Add capability mapping table to resolve abstract capabilities to target native tools.
4. Parse subagent markdown and TOML definitions in frontends.
5. Lower subagents to `.claude/agents/<id>.md`, `.cursor/agents/<id>.md`, `.agents/agents/<id>.md`, `.opencode/agents/<id>.md`, and `.codex/agents/<id>.toml`.

### Artifacts for P2

- Proposal: `openspec/changes/cross-harness-subagents/proposal.md`
- Design: `openspec/changes/cross-harness-subagents/design.md`
- Tasks: `openspec/changes/cross-harness-subagents/tasks.md`
- Tracking bead: `rulette-o7t.8`

---

## 5. Phase P3: Polyglot plugin and multi-marketplace bundler

### Scope for P3

1. Emit strict root `plugin.json` with only `$schema`, `name`, and `description` for Google Antigravity targets.
2. Emit target plugin manifests for Claude, Cursor, and Codex.
3. Emit marketplace catalog files for Claude (`.claude-plugin/marketplace.json`), Cursor (`.cursor-plugin/marketplace.json`), and Agent Plugins (`.agents/plugins/marketplace.json`).
4. Generate the 4-tier fidelity report (Tier 0 to Tier 3) during compilation and inspection.

### Artifacts for P3

- Proposal: `openspec/changes/polyglot-plugin-and-marketplaces/proposal.md`
- Design: `openspec/changes/polyglot-plugin-and-marketplaces/design.md`
- Tasks: `openspec/changes/polyglot-plugin-and-marketplaces/tasks.md`
- Tracking bead: `rulette-o7t.9`

---

## 6. OpenSpec lifecycle and adversarial review gates

Every OpenSpec change proposal must execute the standard lifecycle from [.beads/formulas/openspec-workflow.formula.toml](file:///home/mkobit/workspace/mkobit/rulette/.beads/formulas/openspec-workflow.formula.toml):

1. **Proposal step**: Author `proposal.md` with explicit problem statement, scope, capabilities, and non-goals.
2. **Design step**: Author `design.md` with architecture, data models, invariant rules, and trade-off analysis.
3. **Adversarial review step**: Document edge cases, security failures, failure modes, and test mitigations in `design.md`.
4. **Tasks step**: Author `tasks.md` with numbered, testable implementation tasks.
5. **Beads step**: Create atomic Beads issues linked to tasks before starting code edits.
