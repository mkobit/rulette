---
name: example-skill
description: An example Claude skill
version: 1.0.0
license: MIT
compatibility: claude-3-5-sonnet
models: ["claude-3-5-sonnet", "claude-3-opus"]
seeded: true
allowed-tools: Bash(git:*)
rulette:activation:
  - manual
  - model
rulette:tool-access:
  - rule: Bash(git:*)
    allowed: true
---
# Example Skill

This is the content of the example skill.
