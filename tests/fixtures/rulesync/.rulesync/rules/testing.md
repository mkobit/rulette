---
root: true
targets: ["cursor", "claudecode", "codexcli", "opencode", "antigravity"]
description: Testing conventions and test suite isolation standards
globs: ["**/*.test.ts", "**/*.spec.ts", "tests/**/*.ts"]
cursor:
  alwaysApply: false
claudecode:
  target: project
antigravity:
  trigger: glob
---
# Testing conventions

Practices for reliable and reproducible test suites.

- Co-locate unit tests alongside implementation code or under `tests/`.
- Use consistent naming conventions with `.test.ts` or `.spec.ts` extensions.
- Clean up temporary files and directories in teardown hooks.
- Assert on observable external behavior rather than internal state.
