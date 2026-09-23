---
root: true
targets: ["*"]
description: TypeScript conventions and type safety standards
globs: ["**/*.ts", "**/*.tsx"]
cursor:
  alwaysApply: true
antigravity:
  trigger: model_decision
  description: Apply when editing or reviewing TypeScript files
---
# TypeScript conventions

Guidelines for writing maintainable, type-safe TypeScript code.

- Prefer `unknown` over `any` for unvalidated dynamic inputs.
- Enable `strict` mode in compiler configurations.
- Avoid non-null assertions (`!`) where optional chaining or type guards apply.
- Declare explicit return types on all exported functions.
