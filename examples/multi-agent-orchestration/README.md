# Multi-agent orchestration example

This example demonstrates how to use Rulette to manage a centralized set of AI rules and skills, then "compile" them into target-specific configurations for different agents (Claude, Cursor, and Gemini).

## Scenario

You have a set of shared organizational rules and individual specialized skills.
You want to:

1.  Sync all stable rules to Claude and Gemini.
2.  Sync experimental skills only to Cursor.
3.  Add a "company-policy" tag to all rules during the process.

## Rules directory structure

```text
rules/
├── shared-rules.md         (General coding standards)
├── security.skill.md       (Security auditing skill, status: stable)
└── experimental-ui.mdc     (Experimental UI rules, status: experimental)
```

## Compilation steps

### 1. Compile stable rules for Claude and Gemini

We filter for entities with `status == "stable"` and emit them to their respective locations.

```sh
rulette transform rules/ \
  --filter 'status == "stable"' \
  --set "org=mycompany" \
  -o claude:.claude/ \
  -o gemini:.gemini/
```

### 2. Compile experimental rules for Cursor

We filter for `status == "experimental"` and emit them as Cursor MDC rules.

```sh
rulette transform rules/ \
  --filter 'status == "experimental"' \
  -o cursor-mdc:.cursor/rules/
```

### 3. Pipeline via `jq` for advanced filtering

Since Rulette outputs typed IR, you can use `jq` for logic that isn't yet built into the Rulette filter engine.

```sh
# Only include skills that have "typescript" in their body
rulette transform rules/ --to ir-json | \
  jq '.entities |= map(select(.kind == "skill" and (.body | contains("typescript"))))' | \
  rulette transform - --to claude -o claude:.claude/skills/
```
