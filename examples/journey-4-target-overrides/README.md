# Journey 4: per-target activation overrides

This example demonstrates Journey 4 from the CLI architecture design: defining default rule activation semantics with per-target overrides in canonical source frontmatter using `rulette:activation`.

## Source rule with typed activation

The canonical rule is defined in `./rules/typescript.md`:

```markdown
---
description: "TypeScript guidelines and conventions"
rulette:activation:
  default:
    mode:
      - glob
    globs:
      - "**/*.ts"
      - "**/*.tsx"
  overrides:
    cursor:
      mode:
        - always
    antigravity:
      mode:
        - model
---
# TypeScript conventions

Guidelines for writing maintainable TypeScript code.

- Prefer `unknown` over `any` for unvalidated inputs.
- Enable strict null checks in compiler options.
- Export explicit type signatures on module boundaries.
```

## Per-target resolution behavior

When lowered into target formats:

- **Cursor**: matches the `cursor` override, producing `alwaysApply: true`.
- **Antigravity**: matches the `antigravity` override, producing `trigger: model_decision`.
- **Claude**: falls back to the `default` activation, applying file glob matching for TypeScript files.

## Workflow

### 1. Stage the publication plan

```sh
rulette transform \
  --config rulette.transform.jsonc \
  --from antigravity \
  --stage /tmp/journey-4-stage \
  --project-root .
```

### 2. Apply the plan to live destinations

```sh
rulette transform \
  --apply /tmp/journey-4-stage/rulette.plan.json \
  --expect-plan-sha256 <PLAN_DIGEST> \
  --allow-project-root .
```

Each tool configuration file receives the exact activation mode tailored for its runtime environment while maintaining one single source of truth.
