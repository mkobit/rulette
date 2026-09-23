---
description: "TypeScript guidelines and conventions"
rulette:activation:
  default:
    mode:
      - glob
    globs:
      - "**/*.ts"
      - "**/*.tsx"
  overrides:
    cursor:
      mode:
        - always
    antigravity:
      mode:
        - model
---
# TypeScript conventions

Guidelines for writing maintainable TypeScript code.

- Prefer `unknown` over `any` for unvalidated inputs.
- Enable strict null checks in compiler options.
- Export explicit type signatures on module boundaries.
