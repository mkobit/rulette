# Example: Gemini Subagents

This example demonstrates Rulette's native support for Gemini CLI subagents.

## Source

The source is a Gemini-formatted subagent with YAML frontmatter in `agents/architect.md`.

## Transformation

You can transform this agent to other formats (like a Claude skill) or just inspect its IR:

```sh
rulette transform agents/ --to agent-skills --out skills/
```

Rulette preserves Gemini-specific metadata like `temperature`, `max_turns`, and `model` in the IR's `extra` fields, ensuring they are not lost during round-trips.
