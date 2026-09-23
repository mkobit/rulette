# Journey 3: consolidate disparate tool configuration

This example demonstrates Journey 3 from the CLI architecture design: migrating disparate, hand-written AI tool configurations into one canonical source under `./rules/` and verifying lossless fidelity using `transform --check`.

## Context

Many projects start by manually writing configuration files across multiple AI tools:

- `AGENTS.md` for Codex
- `CLAUDE.md` for Claude
- `.cursor/rules/conventions.mdc` for Cursor
- `.opencode/rules/conventions.md` for OpenCode
- `.agents/rules/conventions.md` for Antigravity

Over time, these configurations drift and create maintenance overhead.

## Consolidation approach

### 1. Consolidate canonical rules

Consolidate shared rule content into the canonical source file `./rules/conventions.md`.

### 2. Define the multi-target manifest

Define the multi-target configuration in `rulette.transform.jsonc`:

```jsonc
{
  "inputs": ["./rules/"],
  "targets": [
    { "target": "codex", "scope": "project" },
    { "target": "claude", "scope": "project" },
    { "target": "cursor", "scope": "project" },
    { "target": "opencode", "scope": "project" },
    { "target": "antigravity", "scope": "project" }
  ]
}
```

### 3. Verify lossless consolidation

Run `transform --check` to verify the canonical rule reproduces existing files losslessly:

```sh
rulette transform \
  --config rulette.transform.jsonc \
  --from antigravity \
  --project-root . \
  --check
```

When all targets report `unchanged` and the command exits with code 0, the migration is verified to be completely faithful.
