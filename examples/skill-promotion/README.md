# Example: Skill Promotion

This example demonstrates how to "promote" a simple Markdown rule into a structured agent skill by injecting metadata during the transformation.

## Source

A simple Markdown file `simple/guidelines.md` with no frontmatter.

## Transformation

Run the following command to promote it to a skill with a name and description:

```sh
rulette transform simple/guidelines.md \
  --to agent-skills \
  --name "ci-guidelines" \
  --description "Rules for CI and code quality" \
  --out skills/
```

The output in `skills/ci-guidelines.skill.md` will contain the injected metadata in its frontmatter.
