# RuleSync test fixture layout

This directory contains a realistic test fixture demonstrating a repository configured with [RuleSync](https://github.com/dyoshikawa/rulesync).
It serves as the test fixture for migration testing and comparative validation against Rulette.

## Fixture layout

```text
tests/fixtures/rulesync/
├── .rulesync/
│   ├── rulesync.jsonc
│   └── rules/
│       ├── testing.md
│       └── typescript.md
└── README.md
```

## Structure and semantics

The fixture models standard RuleSync synchronization patterns across five target coding assistants:

- `.rulesync/rulesync.jsonc`: Top-level configuration declaring targets (`cursor`, `claudecode`, `codexcli`, `opencode`, `antigravity`) and active feature sets (`rules`).
- `.rulesync/rules/typescript.md`: Rule with global `targets: ["*"]`, glob filters, and per-tool override blocks (`cursor.alwaysApply: true`, `antigravity.trigger: "model_decision"`).
- `.rulesync/rules/testing.md`: Rule targeting explicit tools with path-scoped globs (`**/*.test.ts`, `**/*.spec.ts`) and tool-specific triggers.

## Migration mapping to Rulette

The fixture demonstrates concepts translated during migration into Rulette IR and declarative transform configurations:

- `.rulesync/rulesync.jsonc` maps to `rulette.transform.jsonc` with target and scope selections.
- Tool targets `cursor`, `claudecode`, `codexcli`, `opencode`, and `antigravity` map to Rulette target tokens `cursor@project`, `claude@project`, `codex@project`, `opencode@project`, and `antigravity@project`.
- Per-tool frontmatter blocks map to typed `rulette:activation` default and override structures described in [docs/2026-08-18-cli-ux-design.md](../../../docs/2026-08-18-cli-ux-design.md).
