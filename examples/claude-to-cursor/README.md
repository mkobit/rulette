# Example: Claude Skill to Cursor MDC

This example demonstrates how to transform a structured Claude skill into a Cursor MDC rule.

## Source

The source is a structured Markdown skill in `rules/typescript.skill.md`.

## Transformation

Run the following command to generate a Cursor MDC:

```sh
rulette transform rules/ --to cursor-mdc --out .cursor/rules/
```

The output will preserve the description and content while adapting to Cursor's MDC frontmatter format.
