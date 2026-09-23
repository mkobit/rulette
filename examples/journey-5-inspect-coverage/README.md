# Journey 5: inspect and coverage reporting

This example demonstrates Journey 5 from the CLI architecture design: debugging intermediate representation and inspecting target capabilities and coverage before publishing or committing to CI.

## Source rules

The rules directory contains `./rules/code-style.md`:

```markdown
# Code style and standards

- Keep functions small and single-purpose.
- Prefer explicit immutable data structures.
- Document non-obvious architecture and business logic decisions.
```

## Workflow

### 1. Inspect target-specific lowering

Inspect how the compilation graph lowers to a specific target without writing any files:

```sh
rulette inspect ./rules/ \
  --from antigravity \
  --to cursor
```

This prints the intermediate representation and target capability findings (supported, lossy, or dropped) with clear reason codes.

### 2. Evaluate cross-target coverage matrix

Generate an end-to-end capability matrix across all supported targets:

```sh
rulette inspect ./rules/ \
  --from antigravity \
  --coverage
```

Output displays the support status for every observed package kind across Codex, OpenCode, Claude, Cursor, and Antigravity.

### 3. Generate machine-readable coverage for automation

For automated PR comments or CI assertions, output coverage as JSON:

```sh
rulette inspect ./rules/ \
  --from antigravity \
  --coverage \
  --json
```

The structured JSON output can be piped directly into tooling to enforce that no packages are dropped or degraded.
