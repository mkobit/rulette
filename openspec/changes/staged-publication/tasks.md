## 1. Publication model and canonical plan

- [x] 1.1 Create `src/publication/mod.rs` and `src/publication/model.rs` as the library-owned boundary for `PublicationScope`, `RootIdentity`, `AuthorizedRoot`, `PlanDigest`, `PlanEntry`, `PublicationPlan`, destination status, and structured loss findings.
- [x] 1.2 Restrict `PublicationScope` deserialization to `project` and `user`, make every plan struct reject unknown fields, and ensure no plan field can hold an absolute root, an environment-expanded path, a credential, or an authority grant.
- [x] 1.3 Create `src/publication/plan.rs` to serialize the versioned `0.1` plan with fixed field order, sorted entries, UTF-8 strings, integer byte lengths, and canonical SHA-256 digests of the exact bytes read by apply.
- [x] 1.4 Implement plan parsing that verifies `--expect-plan-sha256` against raw bytes before JSON deserialization and rejects unsupported versions, duplicate entry identifiers, malformed artifact paths, non-canonical shapes, and unrecognized mapping versions.
- [x] 1.5 Derive a root identity from the caller-opened root's canonical platform spelling plus volume and file identity using length-prefixed fields, and retain only its digest in a plan.
- [x] 1.6 Add unit tests in `src/publication/model.rs` and `src/publication/plan.rs` for stable ordering, digest stability, raw-byte digest rejection, schema rejection, duplicate entries, and root identity changes.

## 2. Target mappings, lowering, and loss findings

- [x] 2.1 Define `NativeArtifact`, `NativeArtifactClass`, `MappingVersion`, `TargetMapping`, and structured per-package capability findings in `src/emitters/mod.rs` or the compilation-graph-owned lowering module, without allowing a backend to return an absolute or unchecked filesystem path.
- [x] 2.2 Extend the core target backends in `src/emitters/codex.rs`, `src/emitters/opencode.rs`, `src/emitters/claude.rs`, `src/emitters/cursor.rs`, and `src/emitters/antigravity.rs` to expose deterministic, versioned project mappings and typed artifact validation for each supported native artifact class.
- [x] 2.3 Add only vendor-documented user mappings to those registries, keep Cursor user rules unavailable, and reject local, enterprise, managed, system, and arbitrary-path mappings before filesystem access.
- [x] 2.4 Update the lowering coordinator in `src/emitters/mod.rs`, `src/lib.rs`, and the compilation-graph integration point to lower selected packages into ordered `NativeArtifact` values and to carry source package identities, executable-bit metadata, and structured loss findings.
- [x] 2.5 Make lowering fail before staging on any lossy or dropped finding unless `--allow-lossy` is explicit, and retain accepted findings in the publication plan.
- [x] 2.6 Update `src/cli/commands/inspect.rs` so coverage reports the same target, package, resource, scope-mapping, severity, finding identifier, and reason-code classifications that staged lowering uses.
- [x] 2.7 Add backend unit and integration tests in the core emitter modules, `tests/cli_tests/coverage_tests.rs`, and `tests/cli_tests/staged_publication_tests.rs` for deterministic artifacts, each project mapping, documented user mappings, unavailable Cursor user mapping, and loss-finding parity.

## 3. Safe root-relative filesystem primitives

- [x] 3.1 Create `src/publication/fs/mod.rs`, `src/publication/fs/unix.rs`, and `src/publication/fs/windows.rs` with a shared handle-oriented interface for opening roots, validating relative components, reading regular files, creating directories, writing temporary files, renaming, and rollback.
- [x] 3.2 Add only the direct platform dependencies required by those modules to `Cargo.toml` and preserve the fully static binary with no runtime service or runtime configuration dependency.
- [x] 3.3 Implement Unix component-by-component no-follow traversal with descriptor-relative operations and mount-boundary rejection in `src/publication/fs/unix.rs`.
- [x] 3.4 Implement Windows handle-relative traversal with reparse-point and volume-boundary rejection in `src/publication/fs/windows.rs`, and return an unsupported-publication error when the platform cannot enforce the contract.
- [x] 3.5 Reject absolute paths, empty and parent components, reserved names, platform-normalization collisions, repository-control namespaces, `.git`, symlinks, junctions, reparse points, and non-regular destination files through the shared interface.
- [x] 3.6 Add focused platform tests in `tests/cli_tests/staged_publication_unix_tests.rs` and `tests/cli_tests/staged_publication_windows_tests.rs` for link traversal, invalid components, reserved names, parent validation, and unsupported-safe-operation failures.

## 4. Isolated staging and plan creation

- [x] 4.1 Create `src/publication/stage.rs` with a library API that accepts validated typed artifacts, explicit staging roots, mapping registries, accepted loss findings, and a requested stage directory.
- [x] 4.2 Validate every requested project and user root during staging, bind its `RootIdentity` to the matching mapping, and use the roots only for mapping resolution rather than destination mutation.
- [x] 4.3 Build artifacts below an exclusively created sibling temporary directory through the safe filesystem interface, store them below deterministic `artifacts/<entry-id>/` paths, and reject any artifact path outside that layout.
- [x] 4.4 Write a complete canonical `rulette.plan.json` containing compiler and graph versions and digests, mapping versions, hashed root identities, accepted loss findings, and every ordered artifact entry.
- [x] 4.5 Fsync staged files and the plan where supported, atomically publish the complete stage directory without replacement, and remove only the owned temporary directory on failure.
- [x] 4.6 Add staging tests in `tests/cli_tests/staged_publication_tests.rs` for deterministic bytes and plan digest, exact stage layout, existing stage rejection, loss-default failure, accepted-loss recording, and proof that staging does not create a live destination.

## 5. Apply preflight, check mode, and transaction

- [x] 5.1 Create `src/publication/apply.rs` to open an untrusted plan and stage root through stable handles, validate the plan digest before parsing, and verify every staged artifact's type, digest, byte length, and executable metadata before any destination read.
- [x] 5.2 Revalidate each artifact's target, mapping version, class, and normalized native path against the compiled-in registry before deriving its destination below an explicitly authorized root.
- [x] 5.3 Preflight all entries before mutation by checking exact root identity and scope authority, duplicate and ancestor-descendant destinations, path safety, mapping availability, known managed or system paths, and destination classification.
- [x] 5.4 Implement absent, unchanged, and conflicting classification by comparing exact bytes and target-representable executable metadata, skip unchanged files, and require `--replace` for every conflict.
- [x] 5.5 Create `src/publication/transaction.rs` to write changed destinations through same-directory temporary regular files and atomic rename after complete preflight succeeds.
- [x] 5.6 Record original bytes, executable metadata, and file identity before replacement, and on an observed late failure remove newly created files, remove empty owned directories, and restore only files whose identity and digest still prove apply ownership.
- [x] 5.7 Return both the original failure and any rollback failure without overwriting a concurrent third-party change, and document in code that crash, power-loss, stale-deletion, and non-cooperating-writer atomicity are out of scope.
- [x] 5.8 Implement source-mode and plan-mode `--check` through the same preflight classifier without creating a stage, parent directory, temporary file, or destination.
- [x] 5.9 Add transaction and check tests in `tests/cli_tests/staged_publication_tests.rs` for plan or artifact tampering, root mismatch, absent and unchanged results, conflict with and without replacement, multi-target preflight failure, injected late failure with rollback, plan-mode drift, source-mode drift, and zero filesystem mutation during checks.

## 6. Transform CLI and explicit transform configuration

- [x] 6.1 Replace direct native publication arguments in `src/cli/commands/transform.rs` with repeatable `--target <format>@<scope>`, `--stage <directory>`, `--apply <plan>`, `--expect-plan-sha256 <digest>`, `--project-root <root>`, `--user-root <target>=<root>`, `--allow-project-root <root>`, `--allow-user-root <target>=<root>`, `--replace`, `--allow-lossy`, and `--check`.
- [x] 6.2 Make apply mutually exclusive with sources, `--from`, package-selection flags, `--target`, `--stage`, and `--config`, and make replace and expected-plan-digest valid only with apply.
- [x] 6.3 Require explicit staging roots for source-mode project and user targets, require exact allow roots for plan-mode check and apply, and ensure the CLI only adapts arguments before calling the library publication APIs.
- [x] 6.4 Remove direct native `--to` and `--out` publication behavior, retain graph output on standard output, and reject a native target that is neither staged nor applied.
- [x] 6.5 Move transform strictness to strict-by-default loss handling with `--allow-lossy`, while preserving `inspect --coverage --strict` as an informational failure gate in `src/cli/globals.rs`, `src/main.rs`, and `src/cli/commands/inspect.rs`.
- [x] 6.6 Replace the output-path and per-output strict fields in the explicit transform-config schema in `src/cli/commands/transform.rs` with safe source, target, logical-scope, and package-selection declarations, and reject config fields that could grant roots, replace permission, loss permission, raw destination paths, or authority.
- [x] 6.7 Update `src/cli/formats.rs`, `src/cli/commands/schema.rs`, and generated CLI metadata so only the core native target tokens and supported scope syntax are accepted for this release.
- [x] 6.8 Add command-surface tests in `tests/cli_tests/staged_publication_tests.rs`, `tests/cli_tests/transform_config_tests.rs`, `tests/cli_tests/transform_tests.rs`, and `tests/cli_tests/strict_tests.rs` for flag conflicts, missing root authorization, config non-authority, native-target stage requirement, stdout graph preservation, and removed direct-write behavior.

## 7. Documentation and generated reference

- [x] 7.1 Update `docs/2026-04-11-prd.md` with the v0.1 stage, review, apply, and check journey for project and documented user mappings.
- [x] 7.2 Update `docs/2026-04-11-man-page.md` and `docs/cli/rulette.md` with the new target, root, stage, apply, check, replace, and allow-lossy syntax, including an explicit migration from direct output paths.
- [x] 7.3 Update `docs/2026-08-18-cli-ux-design.md` to remove superseded direct-write, unverified-scope, and transform-strict claims while preserving transform configuration as an explicit non-authoritative input.
- [x] 7.4 Regenerate CLI reference output with `cargo run --bin gen_docs` after the command schema is complete.

## 8. Cross-platform conformance and release validation

- [x] 8.1 Run the focused publication, transform-config, coverage, and target-emitter tests on Unix and Windows, with platform-specific safe-filesystem scenarios enabled on their native platform.
- [x] 8.2 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` after every platform-specific implementation adjustment.
- [x] 8.3 Run `mise run check` and `mise run spec-validate` from the repository root, and resolve every reported failure before marking this change complete.
- [x] 8.4 Verify release packaging still produces the static `rulette` binary without a daemon, hidden state store, automatic configuration discovery, or runtime configuration dependency.
