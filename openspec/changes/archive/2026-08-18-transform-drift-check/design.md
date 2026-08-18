## Context

`src/cli/commands/transform.rs`'s write loop already does two passes: it first renders every target's full output into memory (`generated_outputs`), then writes each rendered file to disk, tracking `written_paths` so that a mid-loop write failure can roll back (`fs::remove_file`) everything written so far in this invocation — this was added to make multi-target emission genuinely all-or-nothing on disk (see the existing "All-or-nothing multi-target emission" requirement).

This change adds a third concern to that same write loop: before writing a target, compare its rendered content to what's already on disk and skip the write if they match. This interacts directly with the existing rollback logic, which is the main source of design risk here (see Decisions below).

## Goals / Non-Goals

**Goals:**

- Skip writing target files whose rendered content is unchanged, so re-running `transform` on unchanged input doesn't touch mtimes or produce git noise.
- Report per-file status (created / updated / unchanged).
- Add a check mode that computes drift and exits non-zero without writing, replacing the `transform` + `git diff --exit-code` two-step CI pattern with one command.
- Preserve the existing all-or-nothing rollback guarantee for files this invocation actually writes.

**Non-Goals:**

- Fetch/source-side work (registries, lockfiles, checksums) — tracked separately, deferred.
- Detecting drift for stdout targets (no `-o` path) — there is nothing on disk to compare against; see Decisions.
- A general file-watching / continuous-sync mode. This is a single-invocation check, not a daemon.
- Changing emitter (`src/emitters/*.rs`) behavior. Emitters already produce full rendered content; this change only affects what `transform.rs` does with that content before touching the filesystem.

## Decisions

### Rollback must restore, not delete, files it overwrote

The existing rollback logic (`fs::remove_file` on every path in `written_paths`) is correct today because every tracked path was newly created by this invocation. Once writes can also be "updates" to pre-existing files, blind `fs::remove_file` on rollback would destroy a file that existed before this invocation ever ran — strictly worse than leaving the partial write in place.

Decision: track each write as `Written::Created(path)` or `Written::Updated { path, original_content }`, capturing `original_content` (read before the overwrite) at write time. On rollback: `Created` paths are removed; `Updated` paths are restored to `original_content`. This keeps the existing "no partial writes" guarantee accurate for both new and existing files.

Alternative considered: keep `fs::remove_file` for everything and accept that a rolled-back "update" leaves the file missing rather than restored. Rejected — deleting a user's pre-existing file as a side effect of an unrelated write failure elsewhere in the invocation is a worse outcome than what atomicity was meant to prevent in the first place.

### Content comparison is a plain byte/string equality check, but an unreadable existing file is a hard error, not a write

Decision: read the existing file with `fs::read_to_string`. If it succeeds and matches the rendered output, the target is `Unchanged`. If it succeeds and differs, the target is `Updated` (with `original_content` captured for rollback — see above). If the path does not exist, the target is `Created`. If the path exists but `fs::read_to_string` fails for any other reason (non-UTF-8 content, permission denied, or any other I/O error), Rulette SHALL treat this as a hard error for the whole invocation, before any target in this run is written, rather than silently proceeding to overwrite something it couldn't read.

This closes a gap found in adversarial review: an earlier version of this decision folded "can't read the existing file" into the same `Updated` bucket as an ordinary content change. But `Updated` requires `original_content` to make rollback safe, and there is no content to capture when the read itself failed — silently writing anyway would either (a) skip capturing `original_content` and make a later rollback fall back to deleting the file, reintroducing the exact destructive-rollback bug this change exists to fix, or (b) capture a wrong/empty placeholder and restore corrupted content on rollback. Failing loudly for that one target, before any writes happen (i.e. as part of validation, alongside the existing render phase), avoids both.

Alternative considered: normalize whitespace/line-endings before comparing, to avoid "unchanged" writes flip-flopping between platforms. Rejected for this change — Rulette's emitters already produce their own consistent output, and normalizing on read risks masking a real emitter bug (e.g. inconsistent line endings) that a byte-exact comparison would surface as a spurious "always updated" status instead of hiding it.

### Target paths must be regular files or absent — never symlinks or directories

Decision: before reading or writing a target path, check it with `fs::symlink_metadata` (which does not follow symlinks, unlike `fs::metadata`/`fs::read_to_string`/`fs::write`). If the path exists and is anything other than a regular file (a symlink, a directory, or another special file type), Rulette SHALL treat this as a hard error for that target, before any writes happen in this run, rather than reading through or writing through it.

This closes a gap found in adversarial review: a target path could be a symlink (e.g. planted by an untrusted PR that CI then runs `rulette transform` against). Following it on read would make the new drift-comparison step a read oracle — `--check`'s exit code and reported status would reveal whether an arbitrary symlinked-to file's content matches the rendered output, without ever printing that content. Following it on write would silently overwrite whatever the symlink points to, which is a strictly worse version of the "clobber a file this invocation doesn't own" problem the `Updated`-rollback decision above already exists to prevent for regular files. Rejecting non-regular-file targets closes both.

### Check mode performs no filesystem mutation of any kind

Decision: in check mode, the write loop still runs the same render → compare pipeline, but every filesystem-mutating call is skipped — not just `fs::write`, but also `fs::create_dir_all` (currently called unconditionally for every target's parent directory before the write, in the existing code this change builds on) and `fs::remove_file` (rollback never runs in check mode because nothing was written). Rendered content is never printed to stdout in check mode, only per-target status lines. Exit code is non-zero iff any target's status is `Created` or `Updated`.

This closes a gap found in adversarial review: the original wording of this decision only mentioned skipping `fs::write`, which would have left `fs::create_dir_all` running unconditionally and creating empty directories as a side effect of a mode whose entire premise is "never touches disk." The implementation must gate the create-dir-all call behind the same check-mode flag as the write itself, not just guard the write call in isolation.

Alternative considered: have check mode print full unified diffs per file (closer to `terraform plan`). Rejected as more than this change needs — Rulette already has `inspect --to <format>` for humans who want to see rendered content; check mode's job is a fast, scriptable yes/no for CI, matching the `prettier --check` / `black --check` convention rather than a diff tool.

### Check mode requires at least one file target

Decision: if check mode is requested but every target is stdout (`-o` omitted, or a target explicitly written to `-`), Rulette exits with an error explaining that there is nothing on disk to check drift against. Silently succeeding would let a CI script that forgot `-o` pass its "check" step without ever having verified anything.

Alternative considered: silently treat stdout targets as always "unchanged" (no-op) in check mode. Rejected — this could mask a misconfigured CI invocation (e.g., a typo in `-o`) as a passing check.

### Flag name: `--check`, not `--dry-run`

Decision: name the flag `--check`. Non-goal framing matters here: "dry-run" conventionally means "show me what would happen" (often verbose, content-preview-oriented), while "check" conventionally means "tell me pass or fail" (terse, exit-code-oriented, e.g. `prettier --check`, `black --check`, `terraform validate`-adjacent). The latter matches this feature's actual CI use case exactly.

### Status reporting respects `--quiet`

Decision: per-target status lines (created/updated/unchanged) are suppressed under `-q/--quiet`, consistent with the existing quiet behavior that already suppresses the "Emitted to `<path>`" line. Combined with `--check`, `--quiet --check` relies purely on the exit code — a normal and expected pattern for scripted CI usage.

## Risks / Trade-offs

- **Read-before-write on every target** → one extra `fs::read_to_string`/`fs::symlink_metadata` per target file compared to today. Negligible for the text files Rulette emits; not a concern at Rulette's expected scale (rule/skill counts, not build-artifact scale).
- **`Updated` rollback restores stale content, not "no file"** → if a target legitimately shouldn't exist anymore (e.g. a renamed skill), a failed later target in the same invocation restores the old (soon-to-be-stale) file rather than leaving the new one. This matches the existing all-or-nothing contract (nothing persists from a failed invocation) and is the same trade-off the current rollback already makes for `Created` files; it does not introduce a new failure mode, just extends the existing one correctly to `Updated` files.
- **`--check` and normal writes now share one code path with a status enum** → slightly more branching in the write loop. Mitigated by keeping the render/compare/write phases as three distinct, individually testable steps rather than interleaving them further.
- **Accepted limitation: no protection against concurrent writers.** Flagged in adversarial review: two `transform` invocations (or `transform` racing an editor's format-on-save) targeting the same output path can both read the same pre-write content, both decide to write, and if one later rolls back due to a downstream failure, it restores the content it originally read — silently clobbering whatever the other writer produced in the meantime. Rulette is a single-shot CLI tool with no daemon and no lock file (per its own design principles), so this change does not add file locking to close the race; it is the same class of limitation as any CLI that reads-then-writes a path without an advisory lock. Not mitigated; documented as a known limitation rather than solved, since solving it would mean adding exactly the kind of local-state/locking machinery the project's hard constraints rule out.

## Adversarial Review

This design was adversarially reviewed before task breakdown, per the project's mandatory review step. The review surfaced three real gaps, which are now reflected in the Decisions above rather than left as findings: (1) the original rollback decision didn't say what happens when the existing file can't be read back as text, which could have reintroduced the destructive-delete-on-rollback bug this change exists to fix -- now a hard, pre-write error; (2) target paths that are symlinks or directories were never considered, which would have turned the new drift-read into an arbitrary-file read oracle exploitable via `--check`'s exit code, and made overwrites clobber whatever a symlink pointed to -- now rejected as a hard, pre-write error; (3) the check-mode "never touches disk" guarantee didn't account for the existing unconditional `fs::create_dir_all` call -- now explicitly gated. The review also confirmed rollback correctly telescopes back to true pre-invocation content when the same path is written by multiple targets in one invocation (no bug there), and flagged the cross-process race noted above as a real but accepted limitation rather than something this change should attempt to solve.

## Open Questions

- Exact flag name (`--check` assumed above) and whether it should be a `transform`-only flag or promoted to a global flag alongside `--strict`/`--quiet`. Global placement would make it consistent with those, but check mode is meaningful only for `transform` (not `inspect` or `schema`), which argues for keeping it `transform`-scoped. Leaning `transform`-scoped; confirm during task breakdown.
- Whether "unchanged" status lines should print by default (noisier but explicit) or only under a future `--verbose`, with only created/updated printing by default. Leaning toward printing all three by default since the proposal commits to per-file status reporting and the existing `--quiet` flag already covers the "I don't want any of this" case.
