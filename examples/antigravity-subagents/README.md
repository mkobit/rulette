# Antigravity subagent inspection

This example demonstrates how Rulette parses Antigravity subagents and preserves native configuration in its compilation graph.

## Source structure

The source is an Antigravity subagent located in `.antigravity/agents/architect.md`:

```markdown
---
name: architect
description: Specialized in system design and architectural mapping
kind: local
tools:
  - read_file
  - grep_search
  - list_directory
model: inherit
temperature: 0.1
---
# Architect
```

## Compilation and coverage inspection

Rulette v0.1 focuses on portable rules and skills.
Native agent configurations are not portable across harnesses, but Rulette never silently discards them.
Instead, it ingests them into the compilation graph as opaque unsupported packages with complete frontmatter metadata and provenance preserved.

### 1. Inspect the compilation graph

Run `rulette inspect` to view the graph representation:

```sh
rulette inspect . --from antigravity
```

The output contains the package definition and diagnostics indicating that the agent has been preserved:

```json
{
  "kind": "unsupported",
  "semantic_identity": "unsupported:antigravity-agent--antigravity-agents-architect-md-agent",
  "semantic_item": {
    "kind": "unsupported",
    "native_kind": "antigravity-agent"
  }
}
```

### 2. Inspect capability coverage

Run `rulette inspect` with `--coverage` to see how the agent maps across targets:

```sh
rulette inspect . --from antigravity --coverage
```

The resulting matrix shows that the agent is classified as `dropped` across core targets with reason code `unsupported-semantic`.

### 3. Transformation safety

Because native agents cannot be represented portably in target formats, running `transform` strictly blocks:

```sh
rulette transform . \
  --from antigravity \
  --target cursor@project \
  --stage /tmp/antigravity-stage \
  --project-root .
```

This fails with an unaccepted capability loss error, preventing unintended data loss.
Passing `--allow-lossy` acknowledges the drop and allows remaining portable rules or skills to stage cleanly.
