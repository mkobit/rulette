# Claude skill to Cursor rule

This example demonstrates transforming a structured Claude skill into a Cursor MDC rule using Rulette's staged publication pipeline.

## Source structure

The source is a Claude skill package located in `.claude/skills/typescript-expert/SKILL.md`.

```markdown
---
name: typescript-expert
description: Guidelines for high-quality TypeScript and React development
version: 1.0.0
---
# TypeScript expert
```

## Transformation workflow

Rulette compiles the Claude skill into its compilation graph and stages the Cursor output for review before publication.
Cursor has no native skill-package format, so Rulette lowers the primary instruction into a rule and records a `skill-lowered-as-rule` finding.
This requires `--allow-lossy` to explicitly acknowledge the representation change.

### 1. Stage the transformation plan

Run `transform` targeting `cursor@project` and specifying a stage directory outside the project tree:

```sh
rulette transform . \
  --from claude \
  --target cursor@project \
  --allow-lossy \
  --stage /tmp/claude-to-cursor-stage \
  --project-root .
```

Rulette outputs the compilation graph and prints the plan digest to standard error:

```text
plan digest: sha256_...
```

Alternatively, run the transformation using the declarative configuration file `rulette.transform.jsonc`:

```sh
rulette transform \
  --config rulette.transform.jsonc \
  --allow-lossy \
  --stage /tmp/claude-to-cursor-stage \
  --project-root .
```

### 2. Verify destination drift

Inspect the staged plan before modifying any live project files:

```sh
rulette transform \
  --apply /tmp/claude-to-cursor-stage/rulette.plan.json \
  --expect-plan-sha256 <PLAN_DIGEST> \
  --allow-project-root . \
  --check
```

### 3. Apply the plan

Apply the verified publication plan to write `.cursor/rules/typescript-expert.mdc`:

```sh
rulette transform \
  --apply /tmp/claude-to-cursor-stage/rulette.plan.json \
  --expect-plan-sha256 <PLAN_DIGEST> \
  --allow-project-root .
```
