#!/bin/bash
git commit --amend -m "Migrate Docker Sandbox environment filenames to current standard

Renamed legacy Docker Sandbox files:
- .sbx/.sbxenv.yaml -> .sbx/sbxenv.yaml
- .sbx/.sbxenv.agy.yaml -> .sbx/sbxenv.agy.yaml

Updated all corresponding internal file references in:
- src/bin/check_sbx_env.rs
- src/sbx.rs
- tests/sbx_validation_tests.rs

### Setup and execution
Project-only remains the default. To execute tasks within the Sandbox:
\`\`\`sh
sbx env exec .sbx/sbxenv.yaml -- mise run validate-sbx
\`\`\`

Users may optionally overlay a personal sandbox configuration. The overlay is an optional host configuration and should not be added to git:
\`\`\`sh
sbx env exec .sbx/sbxenv.yaml ~/.local/share/sbx/personal/personal.sbxenv.yaml -- mise run validate-sbx
\`\`\`
For AGY, substitute \`.sbx/sbxenv.agy.yaml\` as the first path argument to maintain its agent-kit reference.

### Evidence-Based Requirements Inventory

Based on the actual files and configuration currently committed to this repository:

**1. Required standalone tool versions (from `mise.toml` and `.sbx/kit/spec.yaml`)**
- Rust: `1.98.1` (with `rustfmt`, `clippy`, `llvm-tools`)
- Bun: `1.4.2`
- Beads (`bd`): `1.2.2` (gastownhall/beads)
- Mise: `v2026.9.1` (installed via `.sbx/kit/spec.yaml`)
- These versions are verified during tests via `check_toolchain_parity` to ensure alignment across tools.

**2. Project-required skills/plugins and dependencies**
- **Beads Issue Tracker:** Configured via `AGENTS.md`, `.claude/settings.json`, and native hooks in `.codex/hooks.json`. The skill file exists at `.agents/skills/beads/SKILL.md`. It tracks tasks locally in Dolt DB.
- **OpenSpec:** Used for spec-driven development (`bun x @fission-ai/openspec`). Validated in `.claude/settings.json` hooks (`.claude/hooks/openspec-validate-hook.sh`).
- **Markdown Linting:** `markdownlint-cli2` executed via bun (e.g. `bun x markdownlint-cli2` in `mise.toml`).
- **Claude Permissions:** Allowed commands are strictly defined in `.claude/settings.json` (`mise run *`, `bd *`).
- **AGY kit:** The AGY sandbox (`.sbx/sbxenv.agy.yaml`) explicitly references `https://github.com/shelajev/agy-sbx-kit.git#ref=3e7016f108f3cf09922cf351b55a49e38d97f9f2`.

**3. Optional personal behavior / Unsupported items**
- **Host Overlays:** The `personal.sbxenv.yaml` from dotfiles (e.g., `~/.local/share/sbx/personal/personal.sbxenv.yaml`) is not tracked or strictly required by this repository.
- **Unverified plugins/skills:** There are no native plugins or MCP server configurations present in `.claude/settings.json` other than permissions and hooks. The project primarily uses standard CLI abstractions (`mise run`).
- **Standalone execution:** The project environment (sandboxes, tests, validation) works perfectly without the personal overlay. `cargo test sbx` and `mise run validate-sbx` all pass using strictly the project's own configuration.

**4. Duplicated installation mechanics**
- The `.jules/env_setup.sh` script duplicates some of the environment setup (downloading mise manually, version `v2026.5.15`) that is also managed by the official sandbox `.sbx/kit/spec.yaml` (which uses mise `v2026.9.1` and handles rust/bun/beads via mise commands). This redundancy could potentially be abstracted into a single reusable kit or initialization script in the future to ensure single-source-of-truth for tool versions.

### References
- [Docker AI Sandboxes: Environment files](https://docs.docker.com/ai/sandboxes/configuration/environment-files/)
- [Docker AI Sandboxes: Customize with kits](https://docs.docker.com/ai/sandboxes/customize/kits/)
- [Optional dotfiles context (unmerged)](https://github.com/mkobit/dotfiles/pull/855)

### Validation
- \`cargo test sbx\` passed.
- \`mise run validate-sbx\` passed.
- \`mise run check\` passed.
- No changes to \`.sbx/kit/spec.yaml\`."
