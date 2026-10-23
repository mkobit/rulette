# SBX delegation design

## Scope and authority

This design covers rulette-hrn and repository workflow files only.
The host coordinator owns Beads writes, Git and GitHub writes, credentials, review, publication, and sandbox removal.
The sandbox executor reads the host-supplied Bead context, changes the allowed files, runs checks, and returns evidence.
The current executor must not stage tasks or change Beads.
This document is a design record, not a second task tracker.
The application OpenSpec requirements do not change.

## Existing evidence

The closed child rulette-hrn.1 isolates release test mocks from `BASH_ENV` and installs `file`.
The closed child rulette-hrn.2 documents direct agent launch with explicit models.
Keep both fixes.
The general `check` task does not call SBX.
The separate `validate-sbx` task needs the host SBX manager.
Before these changes, the AGY environment pinned its upstream kit but mounted a host credential directory.
The shared kit scanned home directories for a token and could retain an old token when the source disappeared.
Direct commands cannot depend on shell profile PATH changes.

## Decisions

Keep `clone: false` in both environments.
Use the actual checkout path, not `/workspace`.
All edits, `.git`, `.beads`, and build output can be shared with the host.
Use one executor per checkout and non-overlapping host edits.
Instruction-based authority is not filesystem isolation.
Do not describe the container as a credential or shared-state security boundary.

Keep direct `sbx env exec` launch with one prompt and an explicit model.
Add an unattended Codex example that disables nested sandbox approvals only inside Docker.
Use AGY only for bounded implementation, not design-heavy work.
Check runtime tools without invoking SBX from the container.
Expose mise tool shims through `/usr/local/bin` so direct commands work with a standard PATH.
Keep profile PATH setup for interactive shells.
Install the tool components used by project checks.

Remove broad token discovery from the shared kit.
Use the pinned AGY kit's host-managed Google API-key-only support, as explicitly selected by the user's replacement instructions.
Remove the default host OAuth directory mount from the AGY environment.
Do not automate OAuth copying without a reviewed host-specific source and lifecycle policy.
Report OAuth-only setups as an external blocker rather than import credentials silently.

Add a runtime smoke script that checks the agent executable, checkout path, required tools, and temporary file cleanup.
Add local tests for both agent paths, direct arguments, failure status, workspace checks, and cleanup.
Use mock agents for deterministic tests and label them as mocks.
These tests cannot prove Docker integration or model authentication.
The host must rebuild both environments and run real smoke tasks before full acceptance.

## Failure and handoff

Stop on the first credential, source allowlist, model, or environment failure.
Return the command, status, and a redacted error.
Do not repeat speculative probes or change host settings.
Keep logs outside the shared repository and remove them only after host review.
Do not delete shared files when removing a sandbox.
The host records duration, approval points, final diff, tests, and remaining risks for each real task.

## Review gate

An independent Codex reviewer completed adversarial review on October 15, 2026, before implementation.
The reviewer allowed implementation with the following required controls.

- Check shim targets and refuse to replace unrelated executables.
- Disable Codex hooks for each delegated invocation because project hooks call Beads.
- Use the host-supplied issue context instead of executor-side database commands.
- Capture the dirty baseline and serialize builds, formatters, and Git operations.
- Use a fresh AGY runtime home so an old copied OAuth token cannot mask missing API-key access.
- Separate mock tests, runtime checks, and authenticated host acceptance.
- Preserve exit status, bound execution time, clean only run-owned temporary files, and retain redacted logs.

These controls are accepted as implementation requirements.
The reviewer did not use AGY or change files.
The follow-up implementation review required explicit Rust component command status, host-only Beads database access, and bounded preflight.
These corrections are included in the implementation and its tests.
No new Beads tasks can be staged by this executor because the host owns Beads mutations.

### Host review finding and migration

Removing OAuth import requires a host migration, not just deletion of the mount and copy code.
The user's current replacement instructions now explicitly select host-managed API-key-only AGY access with no OAuth mount.
This resolves the access-mode decision but does not establish that host credentials or authenticated model access work.
The host must configure approved Google API-key access and use a fresh AGY runtime home without stale OAuth tokens.
Authenticated AGY acceptance remains pending; mock tests and runtime tool checks do not satisfy it.

The host also observed a PTY invocation waiting at `Reading additional input from stdin` until Ctrl-D in this session.
Keep direct launch as the normal command, but document the positional-argument `sh -c` fallback with `exec codex` and `</dev/null`.
Do not claim universal stdin closure or remote-process termination from runner defaults or local exit alone.
