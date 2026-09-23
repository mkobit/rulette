# Cross-harness skill publication

This example demonstrates how Rulette adapts structured Agent Skills across multiple target harnesses using staged publication.

## Source structure

The source is a skill package located in `.agents/skills/ci-guidelines/SKILL.md`:

```markdown
---
name: ci-guidelines
description: Standards for continuous integration and code review
version: 1.0.0
---
# CI guidelines
```

## Transformation workflow

Different AI tools model guidance differently:

- Claude natively supports directory skill packages (`.claude/skills/<name>/SKILL.md`).
- Cursor uses rule files (`.cursor/rules/<name>.mdc`) and has no dedicated skill format.

Rulette preserves skill packages when emitting to Claude, while adapting the primary instruction into a rule for Cursor.
Lowering a skill to a rule format reports a `skill-lowered-as-rule` finding and requires `--allow-lossy`.

### 1. Stage the multi-target plan

Stage the plan targeting both Claude and Cursor:

```sh
rulette transform . \
  --from antigravity \
  --target claude@project \
  --target cursor@project \
  --allow-lossy \
  --stage /tmp/skill-publication-stage \
  --project-root .
```

Or run using the declarative configuration file `rulette.transform.jsonc`:

```sh
rulette transform \
  --config rulette.transform.jsonc \
  --allow-lossy \
  --stage /tmp/skill-publication-stage \
  --project-root .
```

The command records the plan digest to standard error:

```text
plan digest: sha256_...
```

### 2. Verify destination drift

Check the plan against live destinations:

```sh
rulette transform \
  --apply /tmp/skill-publication-stage/rulette.plan.json \
  --expect-plan-sha256 <PLAN_DIGEST> \
  --allow-project-root . \
  --check
```

### 3. Apply the plan

Apply the verified plan to publish the artifacts:

```sh
rulette transform \
  --apply /tmp/skill-publication-stage/rulette.plan.json \
  --expect-plan-sha256 <PLAN_DIGEST> \
  --allow-project-root .
```

This writes:

- `.claude/skills/ci-guidelines/SKILL.md` (structured skill)
- `.cursor/rules/ci-guidelines.mdc` (adapted rule)
