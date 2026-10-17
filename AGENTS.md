# Rulette context

## Hard constraints

The binary must be fully static with no runtime dependencies.
The CLI is a thin wrapper; all logic lives in the library.
No initialization phase, no local state, no configuration files.

## Documentation Reference

For executable capabilities, product requirements, and specifications, see:

- [openspec/specs/ir-core/spec.md](openspec/specs/ir-core/spec.md)
- [openspec/specs/transform-pipeline/spec.md](openspec/specs/transform-pipeline/spec.md)
- [openspec/specs/frontends-and-backends/spec.md](openspec/specs/frontends-and-backends/spec.md)

For CLI command documentation and options, see:

- [docs/2026-04-11-man-page.md](docs/2026-04-11-man-page.md)

For superseded planning and comparison documents (historical only; active specs reside in OpenSpec), see:

- [docs/2026-04-11-prd.md](docs/2026-04-11-prd.md)
- [docs/2026-04-11-announcement.md](docs/2026-04-11-announcement.md)
- [docs/2026-04-11-landscape.md](docs/2026-04-11-landscape.md)
- [docs/2026-04-25-comparison-matrix.md](docs/2026-04-25-comparison-matrix.md)

## Build and test

```bash
mise run check         # Run markdown linting, cargo fmt, clippy, unit and integration tests
mise run spec-validate # Validate all OpenSpec specifications strictly
mise run bd-check      # Verify beads database health and unblocked work
```

## Docker sandbox execution

The host coordinator owns Beads mutations, Git and GitHub writes, credentials, final review, and sandbox cleanup.
The sandbox executor reads the host-supplied Bead context, edits only the allowed paths, runs checks, and returns evidence.
An explicit executor assignment overrides the general Beads session protocol below.
Do not claim, close, create, update, or sync Beads from the executor.
Use the supplied issue context instead of invoking `bd` from the executor, except for the `bd --version` runtime check.
Do not commit, push, merge, change host settings, or install or invoke SBX inside the sandbox.
Use `mise run check` directly when already inside the sandbox.
Run `mise run validate-sbx` only on the host.

Both environments use `clone: false`.
The checkout keeps its host path; do not assume `/workspace` exists.
Edits, `.git`, `.beads`, and build output are shared, not isolated or rolled back.
Record pre-existing changes and preserve them.
Use one executor per checkout and serialize builds, formatters, and Git operations with the coordinator.
Never use `git reset`, `git clean`, or shared-directory deletion as recovery.

The `agent` property in `.sbx/sbxenv.yaml` or `.sbx/sbxenv.agy.yaml` configures the sandbox environment integration.
This setting configures the container environment, but it does not launch the agent automatically.
The `sbx env exec` command runs only the explicit command provided after `--`.

Direct execution is the normal command without a shell wrapper; omit `-i` for noninteractive tasks.
In this session, a host PTY invocation waited at `Reading additional input from stdin` until Ctrl-D.
Do not assume stdin closes in every host invocation.
Reserve shell wrappers (`sh -c` or `sh -lc`) only for commands that require shell syntax or login-shell PATH setup.

Execute validation checks within the Docker sandbox environment by running:

```bash
sbx env exec .sbx/sbxenv.yaml -- mise run check
```

Execute non-interactive tasks using Antigravity (`agy`) with an explicit model and skipped permission prompts:

```bash
sbx env exec .sbx/sbxenv.agy.yaml -- agy --model <model> --dangerously-skip-permissions --print "<prompt>"
```

Execute non-interactive tasks using Codex with an explicit model:

```bash
sbx env exec .sbx/sbxenv.yaml -- codex exec --model <model> --disable hooks --dangerously-bypass-approvals-and-sandbox "<prompt>"
```

If the host invocation leaves stdin open, use this shell-syntax fallback after setting `model` and `prompt`.
The shell receives them as positional arguments, not interpolated shell source, and redirects the executor's stdin explicitly.

```bash
sbx env exec .sbx/sbxenv.yaml -- sh -c 'exec codex exec --model "$1" --disable hooks --dangerously-bypass-approvals-and-sandbox "$2" </dev/null' rulette-codex "$model" "$prompt"
```

Use the Codex bypass flag only inside the authorized Docker sandbox.
The hook flag disables project Beads hooks for this invocation.
Do not use AGY for design-heavy work.
The AGY kit uses host-managed Google API-key access; no host OAuth directory is mounted or copied.
Missing credentials or source approval are host blockers, not permission to change settings.

Create and run a named sandbox from the host for interactive work:

```bash
# Antigravity (agy)
sbx env create --name rulette-task-<id> .sbx/sbxenv.agy.yaml
sbx run --name rulette-task-<id>

# Codex
sbx env create --name rulette-task-<id> .sbx/sbxenv.yaml
sbx run --name rulette-task-<id>
```

Named sandboxes still share the checkout when `clone: false` is set.
Use `sbx env exec` for bounded noninteractive commands, not an implicit agent launch.
See [the SBX workflow](docs/2026-10-15-sbx-delegation.md) for preflight, dispatch, failure handling, and host acceptance.

## Inputs

Single files (path or stdin) and tar archives containing multiple files are both valid inputs.

## Pipeline

Rulette uses a single `transform` command to handle the compilation and staged publication lifecycle of AI rules and skills.
It reads from explicit sources, compiles a deterministic package graph, supports exact package selection, and stages artifacts for authorized publication.
This command replaces separate parse, convert, and emit verbs with a unified, source-to-sink engine.

## Specs

This project uses OpenSpec for spec-driven development.
Run `mise run spec-list` to see current changes and their status.
Run `mise run spec-validate` to validate all specs strictly.
Note that `mise run spec-validate` runs as a PR check on changes under `openspec/**`.
Specs live in `openspec/changes/` and follow the `proposal → design → tasks` artifact flow.
Workflow execution uses the `.beads/formulas/openspec-workflow.formula.toml` formula.
All design proposals must undergo a mandatory adversarial review phase prior to staging implementation tasks.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:970c3bf2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See <https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md> for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:

   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   bd dolt push
   git push
   git status
   ```

5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**

- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->

<!-- BEGIN BEADS CODEX SETUP: generated by bd setup codex -->
## Beads Issue Tracker

Use Beads (`bd`) for durable task tracking in repositories that include it. Use the `beads` skill at `.agents/skills/beads/SKILL.md` (project install) or `~/.agents/skills/beads/SKILL.md` (global install) for Beads workflow guidance, then use the `bd` CLI for issue operations.

### Quick Reference

```bash
bd ready                # Find available work
bd show <id>            # View issue details
bd update <id> --claim  # Claim work
bd close <id>           # Complete work
bd prime                # Refresh Beads context
```

### Rules

- Use `bd` for all task tracking; do not create markdown TODO lists.
- Run `bd prime` when Beads context is missing or stale. Codex 0.129.0+ can load Beads context automatically through native hooks; use `/hooks` to inspect or toggle them.
- Keep persistent project memory in Beads via `bd remember`; do not create ad hoc memory files.

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See <https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md> for details and anti-patterns.
<!-- END BEADS CODEX SETUP -->
