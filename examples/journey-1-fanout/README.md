# Journey 1: author once, fan out to five tools

This example demonstrates Journey 1 from the CLI architecture design: maintaining one canonical rule in `./rules/` and fanning out simultaneously to all five core AI coding tools.

## Source layout

The canonical rule is maintained in `./rules/conventions.md`:

```markdown
# Repository conventions

- Follow sentence case for documentation headings and commit titles.
- Write one sentence per line in markdown files.
- Prefer explicit configuration over automatic discovery.
```

## Declarative fan-out manifest

The transformation manifest `rulette.transform.jsonc` specifies the inputs and five output targets:

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

## Workflow

### 1. Stage the publication plan

Run `transform` using the configuration file to compile the IR and stage outputs:

```sh
rulette transform \
  --config rulette.transform.jsonc \
  --from antigravity \
  --stage /tmp/journey-1-stage \
  --project-root .
```

This compiles `./rules/conventions.md` into the intermediate representation and generates a staged publication plan with its cryptographic SHA-256 digest:

```text
plan digest: sha256_...
```

### 2. Verify destination drift

Run a preflight check against the live project to verify destination files:

```sh
rulette transform \
  --apply /tmp/journey-1-stage/rulette.plan.json \
  --expect-plan-sha256 <PLAN_DIGEST> \
  --allow-project-root . \
  --check
```

### 3. Apply the publication plan

Publish the verified plan to live destination files:

```sh
rulette transform \
  --apply /tmp/journey-1-stage/rulette.plan.json \
  --expect-plan-sha256 <PLAN_DIGEST> \
  --allow-project-root .
```

This populates all five tool configurations atomically:

- `AGENTS.md` for Codex
- `CLAUDE.md` for Claude
- `.cursor/rules/conventions.mdc` for Cursor
- `.opencode/rules/conventions.md` for OpenCode
- `.agents/rules/conventions.md` for Antigravity
