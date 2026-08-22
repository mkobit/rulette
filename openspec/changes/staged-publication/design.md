# Staged publication and scopes design

## Context

The current transform command renders and writes live harness files in one operation.

That combines compilation with filesystem authority and can overwrite a reviewed destination through an untrusted input, checked-in transform configuration, or mistaken output path.

The v0.1 compiler separates deterministic build output from explicit publication while keeping `transform` as the single top-level verb.

## Goals

- Stage native artifacts and a self-contained plan without writing live harness configuration.
- Require an explicit apply operation and per-scope authorization before live publication.
- Guarantee project-scope mapping for every core target.
- Enable only documented, tested user-scope mappings.
- Verify staged bytes, reject conflicting destinations by default, and apply every destination atomically.

## Non-goals

- This change does not add a new top-level publish command, an initialization step, a daemon, a lockfile, a registry, or a persistent authority database.
- This change does not promise portable local-project or enterprise scope publication.
- This change does not write managed or system-owned paths.
- This change does not redefine the compilation graph or target serialization.
- This change does not use a transform configuration, environment variable, stage plan, or source file as implicit write authorization.

## CLI contract

`rulette transform <inputs> --target <format>@<scope> [--project-root <root>] [--user-root <target>=<root>] --stage <directory>` compiles, validates, and writes only a new isolated staging directory.

`--target` is repeatable and `@project` is the default scope when it is omitted.

`--to` and `--out` are removed from native v0.1 publication because a target name and scope replace direct destination paths.

`--stage` is required whenever a native target produces files rather than graph output on stdout.

Staging requires one `--project-root <root>` for any project target and one `--user-root <target>=<root>` for every user target.

Those roots are read only during staging and bind the resulting plan to exactly the roots that may later be authorized for check or apply.

The stage directory must not exist when the command starts.

The compiler creates it only after parsing, graph validation, selection, capability analysis, and lowering have succeeded.

`rulette transform --apply <stage-directory>/rulette.plan.json --expect-plan-sha256 <digest> --allow-project-root <root>` verifies and publishes the exact staged artifact set.

`--apply` is mutually exclusive with source inputs, `--from`, selection flags, `--target`, `--stage`, and transform configuration loading.

`--allow-project-root <root>` is required when a plan contains a project entry and may appear once.

`--allow-user-root <target>=<root>` is repeatable and is required for every target that has a user entry.

These arguments are the only v0.1 publication authority inputs.

`--allow-scope` is removed because authorizing the word `project` or `user` cannot constrain a root selected by the environment or untrusted plan.

Every plan entry's logical scope and resolved root identity must match one explicit apply authority argument.

`--replace` and `--expect-plan-sha256` are valid only with `--apply`.

Without `--replace`, an existing destination with different bytes is a hard conflict.

`--allow-lossy` is valid only during compilation and staging.

When it permits a lowering loss, the exact structured findings are recorded in the plan for review before apply.

`--check` remains non-mutating.

With source inputs, `--check` computes the same lowering plan and compares every resolved destination without creating a stage directory or writing a destination.

With an apply plan, `--check` verifies plan integrity, authority, and destination drift without publishing.

Source-mode checks require the matching `--project-root` and `--user-root` arguments used by staging because destination resolution is an explicit scope operation even when it does not write.

Plan-mode checks require the matching `--allow-project-root` and `--allow-user-root` arguments used by apply.

Graph output to stdout remains available for pipeline composition.

Native layouts are never published to stdout as an alternative to stage or apply.

## Stage layout and plan format

The stage root contains exactly `artifacts/` and `rulette.plan.json`.

The compiler builds the stage in an exclusively created sibling temporary directory, writes all artifacts through descriptor-relative regular-file operations, fsyncs files and the plan where supported, and renames the complete directory to the requested stage path without replacement.

Failure removes the owned temporary directory and never creates the requested stage root.

`artifacts/` holds artifacts under deterministic target entry identifiers rather than final destination paths.

The plan is canonical JSON with stable key ordering and a `plan_version` of `0.1`.

It contains the compiler version, graph version, a graph digest, mapping-version identifiers, hashed canonical root identities, accepted loss findings, and an ordered artifact entry list.

A root identity is the SHA-256 digest of the platform canonical root spelling together with its opened volume and file identity, encoded with length prefixes.

Each artifact entry contains a stable entry identifier, target name, mapping version, logical scope, relative stage artifact path, typed native artifact class, normalized native artifact path, SHA-256 digest, byte length, executable-bit metadata, source package identity, and per-entry capability findings.

The destination descriptor is derived only by revalidating the typed native artifact class and normalized native artifact path against the compiled-in mapping version.

The plan never authorizes an arbitrary project-relative or user-relative destination path.

The plan contains no absolute root path, user-home path, managed path, environment-expanded path, credentials, or implicit authority token.

Each artifact digest covers its exact staged bytes, and the plan digest covers the exact canonical plan bytes.

Canonical plan JSON permits only the versioned typed schema, sorted arrays, fixed struct field order, UTF-8 strings, integer byte lengths, and no floating-point or free-form object values.

The operator-visible plan digest is printed by stage.

Apply requires the supplied `--expect-plan-sha256` value to equal the bytes it reads before it parses or trusts the plan.

The entry identifier is the SHA-256 digest of its target, scope, destination descriptor, mode, and content digest joined with unambiguous length prefixes.

The plan records `allow_lossy: true` only when compilation was explicitly invoked with `--allow-lossy` and at least one entry has a loss finding.

Every loss finding has a stable identifier, graph package identity, target, artifact entry identifier when one exists, severity, reason code, and human-readable reason.

The stage writer rejects any artifact path that is not a normalized relative path below `artifacts/`.

The stage writer creates regular files only and never creates links.

## Scope mapping and authority

Logical scope answers where an artifact is intended to live.

Target mapping answers the harness-specific native location that corresponds to that scope.

Apply authority answers whether this invocation may mutate that logical scope.

The three concerns are independent and must remain distinct in code and schema.

The project root is supplied by the caller through `--project-root <path>` during staging and the matching `--allow-project-root <path>` during plan check or apply.

The compiler canonicalizes the opened project root, stores only its SHA-256 identity in the plan, and requires an exact identity match at check or apply.

Apply never searches upward for a Git root because build and sandbox environments may not contain Git metadata.

Each core target has one documented project mapping owned by its target backend.

Codex project output resolves below the project root using its native `AGENTS.md` and `.codex` conventions.

OpenCode project output resolves below the project root using `opencode.json`, `.opencode`, and native instruction conventions.

Claude project output resolves below the project root using `CLAUDE.md` and `.claude` conventions.

Cursor project output resolves below the project root using `.cursor` conventions.

Antigravity project output resolves below the project root using `.agents` conventions.

User mappings are an allow-list compiled into target backends and are not inferred from a requested output path or environment variable.

Staging binds a user mapping to an explicit `--user-root <target>=<root>` argument, and plan check or apply requires the corresponding `--allow-user-root <target>=<root>` argument.

The v0.1 user mapping table contains only mappings backed by primary vendor documentation and conformance fixtures.

Claude user mappings require an explicit root that is the caller-approved `~/.claude` directory.

Codex user mappings require an explicit caller-approved root and never read `CODEX_HOME` during staging, checking, or apply.

OpenCode user mappings require an explicit root that is the caller-approved `~/.config/opencode` directory.

Antigravity user mappings require an explicit root that is the caller-approved `~/.gemini` directory.

Cursor user rule output is not a v0.1 mapping because its primary documentation does not define a stable filesystem destination for User Rules.

No backend may synthesize a local-project mapping in v0.1.

No backend may map `enterprise`, `managed`, `system`, or an absolute path supplied by an artifact or transform configuration.

Apply uses positive containment beneath an explicitly opened authorized root rather than a blacklist as its primary safety control.

It also rejects known managed and system paths before any destination read or write, including documented vendor managed roots even when the invoking process has permission to modify them.

The v0.1 scope enum contains only `project` and `user` and rejects every other serialized token before mapping or filesystem access.

## Apply preflight and transaction

Apply opens the plan and stage root as untrusted input through stable file handles.

It rejects an unsupported plan version, unexpected plan digest, unsupported mapping version, duplicate entry identifier, duplicate or ancestor-descendant resolved destination, invalid JSON shape, root-identity mismatch, unauthorized scope, unsupported target mapping, invalid path component, or plan entry outside the stage root.

It verifies that every referenced staged file is a regular file and that its digest, length, and executable bit match the plan before reading any destination.

It resolves every destination only after mapping and root authorization pass.

Backends return typed `NativeArtifact` values containing only target, mapping version, artifact class, normalized native artifact path, bytes, executable metadata, and capability findings.

The publication library validates those typed values and is the only layer that turns them into filesystem paths.

It rejects a destination that is a symlink, junction, reparse point, directory, other non-regular file, unreadable existing file, reserved platform name, platform-normalization collision, repository-control namespace, or path outside its authorized root.

It rejects `.git` and every target path that is not producible by the selected backend artifact class.

It classifies every verified destination as absent, unchanged, or conflicting by comparing both bytes and target-representable executable-bit metadata.

An unchanged destination is omitted from mutation.

An absent destination is eligible for creation.

A conflicting destination aborts the whole apply unless `--replace` was supplied.

All destination parent directories are validated before mutation and are created only after every preflight check passes.

All destination traversal, temporary-file creation, replacement, and rollback are relative to pre-opened authorized root handles.

Unix implementations use component-by-component no-follow descriptor traversal and reject mount-boundary traversal.

Windows implementations use handle-relative traversal, reject reparse points, and reject volume-boundary traversal.

A platform that cannot provide these invariants reports publication as unsupported rather than falling back to path-based operations.

Apply writes each changed destination through a same-directory temporary regular file and atomic rename.

It records the exact original bytes, executable metadata, and file identity of every replaced destination before the first mutation.

If a later mutation fails, apply deletes newly created files, removes empty directories it created, and restores every replaced file's original bytes and mode only when the destination still has the identity and digest written by this apply.

Rollback failure or concurrent destination modification is reported together with the original publication failure and never overwrites a third-party change.

Apply is failure-atomic for errors observed by the running process and does not claim crash, power-loss, stale-deletion, or non-cooperating concurrent-writer atomicity.

Apply never follows a symlink while validating, reading, creating, replacing, or rolling back a destination.

## Configuration and capability interaction

An explicit transform configuration may declare source inputs, targets, logical scopes, and selection intent.

It may not declare stage roots, apply roots, user-home locations, replace permission, loss permission, or scope authorization.

Target backends classify loss before stage writing.

The compiler fails a staged build on any `lossy` or `dropped` result unless `--allow-lossy` is explicit.

`inspect --coverage` remains the preflight explanation path and reports the same findings the stage command uses.

Project or user scope capability is reported separately from entity and resource capability so a caller can distinguish an unrepresentable artifact from an unauthorized or unavailable destination.

## Migration

The current direct `--out` publication path is removed before the first public release.

Users replace it with a stage command followed by a separately reviewed apply command.

Existing explicit transform configuration remains an input artifact after its output declarations are migrated from raw write paths to repeated target and logical-scope requests.

`transform --strict` is removed because staged compilation is strict by default, while `inspect --coverage --strict` remains available as an informational gate.

The release notes include a before-and-after migration example and explain that current IR and direct-write behavior were pre-release interfaces.

## Test strategy

- Test canonical plan serialization, stable ordering, digest calculation, and rejection of malformed or duplicate entries.
- Test that stage creation fails safely when the requested stage directory already exists and that every successful stage contains only the documented layout.
- Test each core target's project mapping and each allow-listed user mapping against the vendor-documented paths.
- Test Cursor user-rule scope as unavailable rather than guessed.
- Test project-root containment, invalid path components, absolute destinations, symlink destinations, managed roots, and unsupported scopes as preflight failures.
- Test tampered, missing, substituted, or mode-changed staged artifacts and assert that no destination changes.
- Test absent, unchanged, and conflicting destinations with and without `--replace`.
- Test multi-target preflight failure and injected late-write failure with exact rollback of created and replaced files.
- Test `--check` with source compilation and with a stage plan and assert it creates no directories or files.
- Test that transform configuration and plan contents cannot authorize project or user writes without matching root-specific apply authority.
- Test strict-by-default loss failure, `--allow-lossy` plan recording, and the matching `inspect --coverage` findings.

## Adversarial review resolution

The review found that a scope-only permission could be redirected through the current directory or environment.

The design now binds staging and apply to caller-supplied roots whose canonical path and filesystem identity are hashed into the plan.

The review found that a relative path in a plan could become an arbitrary write capability.

The design now accepts only backend-typed artifact descriptors and revalidates them against the compiled mapping version before deriving a destination.

The review found time-of-check and plan-tampering gaps.

The design now requires a plan-byte digest, stage artifact digests, descriptor-relative no-follow traversal, and platform refusal where those guarantees cannot be implemented.

The review found that apply atomicity and the command migration were overstated.

The design now limits its atomicity claim to observed in-process failures, makes stale deletion a non-goal, defines root-specific authority arguments, removes direct `--to` and `--out` publication, and preserves explicit transform configuration only as safe compilation input.
