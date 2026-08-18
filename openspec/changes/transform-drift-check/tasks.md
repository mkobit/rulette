## 1. Path safety and content-read helpers

- [ ] 1.1 In `src/cli/commands/transform.rs`, add a helper that classifies an existing target path before it's touched: not present, a regular file (readable as UTF-8), a regular file that fails to read (non-UTF-8 or I/O error), or a non-regular-file (symlink/directory) via `fs::symlink_metadata`.
- [ ] 1.2 Add a `WriteStatus` enum (`Created`, `Updated`, `Unchanged`) and a `Written` enum (`Created(PathBuf)`, `Updated { path: PathBuf, original_content: String }`) alongside the existing write loop.

## 2. Drift comparison in the write loop

- [ ] 2.1 Before writing each target in the existing two-phase write loop, classify it using the 1.1 helper and compare rendered content to existing content to compute its `WriteStatus`.
- [ ] 2.2 If the existing path is a non-regular-file (symlink/directory), abort the whole invocation with a hard error before writing any target, per the "Non-regular-file existing target aborts before any writes" scenario.
- [ ] 2.3 If the existing path is a regular file that fails to read, abort the whole invocation with a hard error before writing any target, per the "Unreadable existing target aborts before any writes" scenario.
- [ ] 2.4 Skip the `fs::write` (and the `println!`/status line, in normal mode) for any target whose status is `Unchanged`; leave the file's mtime untouched.
- [ ] 2.5 For a target whose status is `Updated`, capture its pre-write content and record it as `Written::Updated { path, original_content }` (instead of the current `written_paths: Vec<PathBuf>`) before writing.
- [ ] 2.6 For a target whose status is `Created`, record it as `Written::Created(path)` before writing, as today.

## 3. Rollback restores instead of deletes

- [ ] 3.1 Update the existing rollback logic (currently `for path in written_paths.iter().rev() { fs::remove_file(path) }`) to iterate `Written` values in reverse: `Created` paths are removed, `Updated` paths are restored via `fs::write(path, original_content)`.
- [ ] 3.2 Confirm (via a test, see section 6) that repeated writes to the same path by multiple targets in one invocation still roll back to the true pre-invocation content (telescoping), matching the adversarial review's finding that this already works correctly.

## 4. Check mode

- [ ] 4.1 Add a `--check` flag to `TransformArgs` (`src/cli/commands/transform.rs`), scoped to `transform` only (not a global flag), per the design's "Open Questions" leaning.
- [ ] 4.2 Gate every filesystem-mutating call in the write loop behind `!check` — `fs::create_dir_all(parent)`, `fs::write`, and (implicitly) rollback, since nothing is written for rollback to undo in check mode.
- [ ] 4.3 In check mode, still run the classification/comparison step (1.1, 2.1) for every target so status can be reported, but never print rendered content to stdout.
- [ ] 4.4 Before running the write loop in check mode, validate that at least one target resolves to a file path (not standard output); if none do, fail with a clear error and do not proceed, per the "Check mode with only stdout targets fails" scenario.
- [ ] 4.5 After the loop, exit non-zero if any target's status was `Created` or `Updated`; exit zero if every target was `Unchanged`.

## 5. Status reporting

- [ ] 5.1 Replace the current uniform `println!("Emitted to {}", ...)` with per-target status output reflecting `Created` / `Updated` / `Unchanged`, for both normal and check mode.
- [ ] 5.2 Ensure `-q/--quiet` (already wired per the prior quiet-flag fix) suppresses these new status lines the same way it suppresses the existing "Emitted to" line, including in combination with `--check`.

## 6. Tests

- [ ] 6.1 Unchanged target is not rewritten and its mtime is untouched (assert file mtime before/after, or assert no write occurs via a read-only permission trick).
- [ ] 6.2 Changed target is rewritten and reported as updated.
- [ ] 6.3 New target is created and reported as created.
- [ ] 6.4 A single invocation with mixed created/updated/unchanged targets reports each independently and only writes the ones that need it.
- [ ] 6.5 Rollback restores an overwritten target's original content when a later target in the same invocation fails (extend the existing atomic-write rollback test from the prior change).
- [ ] 6.6 Rollback still telescopes correctly to true pre-invocation content when multiple targets in one invocation write the same path.
- [ ] 6.7 An existing target path that is a symlink causes a hard error and no writes happen anywhere in the invocation (unix-gated, matching the existing pattern for permission-based tests in this suite).
- [ ] 6.8 An existing target path that is a directory (where a file is expected) causes a hard error and no writes happen anywhere in the invocation.
- [ ] 6.9 An existing target path with non-UTF-8 content causes a hard error and no writes happen anywhere in the invocation.
- [ ] 6.10 `--check` with no drift exits zero and writes nothing.
- [ ] 6.11 `--check` with drift exits non-zero, writes nothing, and does not create parent directories that didn't already exist.
- [ ] 6.12 `--check` with only stdout targets (no `-o`) fails with a clear error.
- [ ] 6.13 `-q --check` together produce no stdout output, signaling only via exit code.

## 7. Documentation

- [ ] 7.1 Regenerate `docs/cli/rulette.md` via `cargo run --bin gen_docs` to reflect the new `--check` flag.
- [ ] 7.2 Update `docs/2026-04-11-prd.md`'s CI/CD GitHub Actions example to use `rulette transform ... --check` in place of the separate `git diff --exit-code` step, and note the new flag under `transform`'s documented options.
- [ ] 7.3 Update `docs/2026-04-11-man-page.md`'s `transform` section to mention `--check`.

## 8. Verification

- [ ] 8.1 Run `mise run check` (fmt, clippy, markdownlint, full test suite) and confirm it passes clean.
- [ ] 8.2 Run `mise run spec-validate` and confirm it passes clean.
- [ ] 8.3 Manually verify the exact CI use case from the proposal: run `transform` once, then `transform ... --check` a second time with no source changes, and confirm it exits zero with no writes.
