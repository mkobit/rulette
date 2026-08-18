## Why

Today, verifying that generated output is up to date requires an external step: run `rulette transform`, then `git diff --exit-code` to check for drift (as shown in `docs/2026-04-11-prd.md`'s CI/CD example). This has two costs: every `transform` invocation rewrites every output file even when content is unchanged (touching mtimes, generating noisy diffs, invalidating downstream build caches like Bazel), and CI has no way to check "would this change anything?" without actually writing files and then asking git. Rulette should know natively whether a write is a no-op, and should be able to report drift without touching the filesystem at all.

## What Changes

- Before writing each output file, `transform` compares its rendered content against the existing file on disk (if any) and skips the write when content is byte-identical, leaving the file's mtime untouched.
- Per-file emission status is reported (created / updated / unchanged) instead of the current uniform "Emitted to `<path>`" message for every file regardless of whether it changed.
- A new check mode (flag name TBD in design, e.g. `--check`) computes what would change without writing anything to disk, and exits non-zero if any target would be created or updated. This lets CI use `rulette transform ... --check` in place of `rulette transform ...` followed by a separate `git diff --exit-code` step.
- The existing all-or-nothing multi-target write behavior (atomic rollback on failure, from the current `transform-pipeline` spec) is preserved unchanged; drift-skipping is an additional per-file check that happens before a write is attempted, not a replacement for the atomicity guarantee.
- Explicitly out of scope: anything on the fetch/source/registry side (tracked separately, deferred). This change is about the sink/destination side of `transform` only.

## Capabilities

### New Capabilities

(none — this extends an existing capability rather than introducing a new one)

### Modified Capabilities

- `transform-pipeline`: adds a requirement that `transform` SHALL detect when a target's rendered content matches the existing file on disk and skip that write, SHALL report per-file emission status, and SHALL support a check mode that reports drift without writing. This composes with (does not replace) the existing "All-or-nothing multi-target emission" requirement.

## Impact

- `src/cli/commands/transform.rs`: the write loop (already modified once for atomic rollback) needs a pre-write content comparison against the existing file, per-file status tracking, and a check-mode branch that short-circuits before any `fs::write`.
- `src/cli/commands/transform.rs` (`TransformArgs`): new CLI flag for check mode.
- `docs/2026-04-11-prd.md`: CI/CD example can be simplified to drop the separate `git diff --exit-code` step once check mode exists (documentation follow-up, not required for this change).
- No changes to emitters (`src/emitters/*.rs`) — they already compute full output content; this change only affects what `transform.rs` does with that content before writing.
