# SBX delegation workflow

## Boundary and prerequisites

Use a host coordinator and one sandbox executor per checkout.
The coordinator selects ready work, claims it, resolves scope, and provides the full Bead context.
The coordinator owns all Beads writes, Git and GitHub writes, credentials, final review, publication, and cleanup.
The executor reads the host-supplied Bead context and edits only the assigned repository paths.
Do not invoke `bd` from the executor, except for the `bd --version` runtime check.
Even a nominal read command can have database or metadata effects that this workflow has not verified.
Do not ask AGY to design or review a design-heavy change.
Obtain an independent adversarial design review before implementation tasks are staged.

Both environment files use `clone: false`.
The checkout keeps its host path, not `/workspace`.
Edits appear on the host at once.
The container does not isolate `.git`, `.beads`, untracked files, or build output.
Permission instructions do not prevent a trusted executor from writing shared state.
Do not mount unrelated host directories or pass host publication credentials to the executor.

Capture `git status --short`, `git diff`, and `git diff --cached` before dispatch.
Keep these records outside the checkout, with access restricted to the coordinator.
Preserve existing edits and untracked files.
Give the executor an explicit allowed-path list.
Serialize builds, formatters, and Git operations across the host and executor.
Do not use concurrent agents in this shared checkout.
Use a separate checkout if concurrent work is required.

## Command selection

The repository CI pins host SBX v0.45.1.
Check the installed host version and help before use.
Do not install the host SBX manager inside a sandbox.

| Command | Purpose |
| --- | --- |
| `sbx run --name <name>` | Start or resume a named sandbox with its configured agent entry point. |
| `sbx env run <env-file>` | Use an environment's default run path, not the bounded dispatch contract below. |
| `sbx env exec <env-file> -- <command>` | Run the explicit command after `--` and return its result. |

An `agent` setting configures environment integration; it does not add a missing executable command to `env exec`.
Use direct `env exec` for noninteractive delegation.
Do not add `-i` for noninteractive delegation.
In this session, a host PTY invocation waited at `Reading additional input from stdin` until Ctrl-D.
Do not assume that omitting `-i` closes stdin for every host invocation.
Supply the full prompt as one quoted argument.
Use a shell wrapper only when the command needs shell syntax or login-shell PATH setup.
Do not replace direct agent launch with a login shell to hide a tool installation defect.

## Host preflight

Run these commands on the host, from the checkout root.

```sh
sbx --version
sbx run --help
sbx env run --help
sbx env exec --help
mise run validate-sbx
```

The `validate-sbx` task validates the kit and plans both environments without running a model.
It is separate from the in-sandbox `check` task.
After a kit change, use fresh environments through the host's installed SBX lifecycle commands.
Do not reuse an AGY runtime home that can contain a previously copied OAuth token.
Do not delete shared checkout data when replacing an environment.

The AGY environment pins the upstream Antigravity kit at commit `21e1928b5fe0036163307ea8047e48390f2d6fec`.
That kit selects a mutable `latest` agent image, so record the resolved image identity in host acceptance evidence.
That kit supports host-managed Google API-key access and selects the Gemini provider for that mode.
The repository no longer mounts the host OAuth directory or searches home directories for tokens.
The coordinator must approve the pinned Git kit source and configure Google access through the host credential policy.
An OAuth-only account needs a separate reviewed host solution and is not supported by the repository bootstrap.
Never copy tokens into prompts, logs, Git, or the shared kit.
Audit AGY user hooks and tools on the host before dispatch so they cannot perform Beads or publication writes automatically.

Run the runtime check for each environment with the actual absolute checkout path.
The reduced PATH omits mise shims and shell profiles on purpose.

```sh
checkout=$(pwd -P)
timeout --signal=TERM --kill-after=30s 5m sbx env exec .sbx/sbxenv.yaml -- env -u BASH_ENV PATH=/usr/local/share/npm-global/bin:/usr/local/bin:/usr/bin:/bin sh .sbx/kit/check-runtime.sh codex "$checkout"
timeout --signal=TERM --kill-after=30s 5m sbx env exec .sbx/sbxenv.agy.yaml -- env -u BASH_ENV PATH=/usr/local/share/npm-global/bin:/usr/local/bin:/usr/bin:/bin sh .sbx/kit/check-runtime.sh agy "$checkout"
```

This check executes tool versions, Rust check components, and a temporary executable.
Use the timeout recovery and remote-termination checks below for preflight as well as dispatch.
It checks the workspace path and removes only its own temporary directory.
It does not validate model access or Beads write isolation.
The kit exposes a fixed set of mise shims through `/usr/local/bin` and refuses to replace unrelated executables.

## Executor prompt

Use this structure and include the full `bd show <id>` output from the host.
Do not require the executor to claim a second issue.

```text
Complete the host-claimed Bead <id> inside this Docker sandbox.
The user authorizes repository edits and checks inside this boundary.
Read AGENTS.md and relevant skills.
Allowed paths: <explicit paths>.
Preserve the supplied dirty baseline and other changes.
Use the supplied issue context; do not invoke bd except bd --version.
The host owns all Beads database access and mutations.
Do not commit, push, merge, or change credentials or host settings.
Do not install or invoke sbx inside the sandbox.
Use mise run check directly.
Do not stop for routine implementation approval.
Stop for unresolved scope, credential, or environment blockers.
Return changed files, test commands and results, unmet criteria, and blockers.
Bead context: <full host-supplied issue and acceptance criteria>.
```

## Noninteractive dispatch

Select a supported model on the host and set `model` and `prompt` before dispatch.
Do not infer model access from the examples or mock tests.
The Codex command disables hooks for this invocation because project hooks invoke Beads.
The approval bypass flags apply only inside the authorized Docker execution boundary.
Do not use them for a host agent.

```sh
sbx env exec .sbx/sbxenv.yaml -- codex exec --model "$model" --disable hooks --dangerously-bypass-approvals-and-sandbox "$prompt"
sbx env exec .sbx/sbxenv.agy.yaml -- agy --model "$model" --dangerously-skip-permissions --print "$prompt"
```

Choose one command for the assigned executor, not both for the same work.
Check `codex --disable hooks features list` in the runtime if the installed Codex version changes.
The `hooks` feature must be false before delegated execution.
The flag does not grant filesystem protection to Beads or Git.

If a host invocation leaves stdin open, use this shell-syntax fallback.
Pass model and prompt as positional arguments to keep spaces, quotes, and shell metacharacters literal.
The `rulette-codex` argument supplies shell `$0`; `exec` preserves the agent's exit status and `</dev/null` explicitly gives Codex EOF on stdin.
This addresses the observed stdin wait, not authentication, remote termination, or every possible launch failure.
Keep the same timeout, logs, and termination checks as for direct dispatch.

```sh
sbx env exec .sbx/sbxenv.yaml -- sh -c 'exec codex exec --model "$1" --disable hooks --dangerously-bypass-approvals-and-sandbox "$2" </dev/null' rulette-codex "$model" "$prompt"
```

The coordinator must bound execution time and retain the original result.
For a Linux host with GNU `timeout`, this Codex example records duration and separates output streams.
The `mktemp` default creates host logs outside the checkout.

```sh
logs=$(mktemp -d /tmp/rulette-sbx-dispatch.XXXXXX) || exit 1
started=$(date +%s)
status=0
timeout --signal=TERM --kill-after=30s 30m sbx env exec .sbx/sbxenv.yaml -- codex exec --model "$model" --disable hooks --dangerously-bypass-approvals-and-sandbox "$prompt" >"$logs/stdout" 2>"$logs/stderr" || status=$?
finished=$(date +%s)
printf 'status=%s seconds=%s logs=%s\n' "$status" "$((finished - started))" "$logs"
```

For AGY, use the AGY dispatch command in the same timeout and log pattern.
Use the host's equivalent timeout tool on other systems.
A client timeout does not prove the remote agent stopped.
After timeout or interruption, stop the executor with the installed host SBX lifecycle commands and confirm it is no longer running before another dispatch.
Keep the logs and partial diff for review.
Do not restart automatically after timeout.

## Failure handling and handoff

Stop on the first source approval, Docker credential-guard, missing tool, authentication, or model error.
Return the exact command, exit status, and a redacted error.
Do not loop through alternative models, copy credentials, or weaken host policy.
The host resolves approval or credential blockers once, then runs a new bounded attempt.
If ordinary checks try to invoke SBX, report a task-boundary defect instead of installing SBX.

The executor returns its changed paths, test commands, actual results, unmet criteria, and blockers.
The host compares the final diff and untracked files with the saved baseline.
The host reviews failures even when the agent command exits zero.
Only the host updates or closes Beads after review.
Only the host can commit, push, open a PR, or merge under the user's publication policy.
Do not treat this workflow as blanket publication permission.

The host removes the named runtime only after review and confirmed agent termination.
Use the installed SBX removal command after checking its help.
Never use `git reset`, `git clean`, or recursive checkout deletion as sandbox cleanup.
Keep logs until review is complete, then remove only the recorded run-owned log directory.
If cleanup fails, report that failure separately without hiding the original task status.

## Acceptance evidence

The design and independent review record are in [the design document](2026-10-15-sbx-delegation-design.md).
Local Rust smoke tests use mock executables and do not invoke the host SBX manager.
They cover direct arguments for both agents, runtime tools, workspace checks, failure status, and temporary cleanup.
They do not establish authenticated end-to-end acceptance.

### Sandbox validation on October 15, 2026

- `mise run check` passes strict validation for 12 OpenSpec items, Markdown lint, Rust format, clippy, and all Rust tests.
- `cargo test --test sbx_workflow_tests --test integration_test` passes 8 workflow tests and 67 integration tests.
- The existing release-packaging suite passes all 21 tests inside this sandbox.
- `sh -n .sbx/kit/check-runtime.sh` and `git diff --check` pass.
- `codex --disable hooks features list` reports `hooks` as false in Codex 0.149.1.
- `sh .sbx/kit/check-runtime.sh codex "$PWD"` passes with the current shell PATH.
- The reduced-PATH runtime command fails with `Missing tool: cargo` in the existing image, which does not contain the new kit setup.
- A temporary shim-directory probe executes the real pinned Cargo, Rust, Bun, Beads version, formatter, and clippy commands with `BASH_ENV` unset and no mise shim directory in PATH.

The temporary probe checks the shim approach, not a rebuilt image.
AGY is not installed in this Codex sandbox.
No host SBX manager, source approval, credential setup, or real AGY dispatch was run in that sandbox validation.
The pre-existing untracked `docs/superpowers/` directory remains unchanged.

### Host validation on October 16, 2026

The host independently ran `mise run check` through Rulette SBX; it exited 0 in 62.84 seconds.
The host created the fresh named Codex environment `rulette-hrn-verify` from kit 0.2.0.
Its reduced-PATH runtime preflight passed with `BASH_ENV` unset, verifying Cargo/Rust 1.98.1, mise 2026.9.1, Bun 1.4.3, Beads 1.2.2, `file`, compilers, Git, ripgrep, and Rust components.
The host successfully removed the temporary environment.
This completes fresh Codex image validation, not authenticated AGY acceptance, which remains pending.

### Host acceptance still required

The host must complete one real low-risk Bead with each agent in a fresh environment.
For each run, record the following evidence outside the checkout before copying a redacted summary into the host-owned issue.

| Evidence | Required content |
| --- | --- |
| Environment | SBX and agent versions, kit revision, absolute checkout path, fresh runtime home. |
| Preflight | Exact validation and runtime commands, output, exit status, required tool versions. |
| Dispatch | Agent, exact model, bounded prompt, start and end time, original status, separate logs. |
| Approval | Source, credential-guard, credential, publication, and merge approval points, including none. |
| Result | Real Bead ID, reviewed file diff, actual check results, and unmet criteria. |
| Ownership | No executor Beads writes, Git writes, or credential changes, checked against the baseline. |
| Cleanup | Confirmed agent termination, runtime removal result, and log retention or removal result. |

### SBX skill and system setup notes

- Host skill: version the external dotfiles SBX skill with the role split, allowed-path prompt contract, direct launch and positional-argument stdin fallback, stop rules, bounded execution, and evidence format above.
- Shared checkout: document `clone: false`, capture the dirty baseline, and serialize one executor's edits and builds; named runtimes are not separate checkouts.
- Host setup: verify the installed SBX version and help, approve the pinned AGY kit source, and run `mise run validate-sbx` before creating fresh environments.
- AGY migration: configure host-managed Google API-key-only access under host credential policy; do not mount or copy OAuth directories or reuse a home with stale tokens. Keep credentials out of prompts and logs.
- Runtime setup: rebuild for the pinned mise tools, Rust components, native tools, and direct-command shims; run the reduced-PATH preflight for each agent before authenticated dispatch.
- Acceptance: the host records real model access, reviewed diffs, check results, termination, and cleanup. Source approvals and credential failures are host blockers, not a skill retry loop.

The external skill update and host system setup remain host-owned work, not changes performed by this executor.
Do not mark rulette-hrn complete until external skill work and both real runs have evidence.
