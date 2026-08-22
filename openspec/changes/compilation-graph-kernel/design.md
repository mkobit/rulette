# Compilation graph kernel design

## Context

The current `RuletteDocument` aggregates text-only entities after discovery has already discarded package layout, byte content, and most source identity.

That makes a skill appear to be the same unit in every harness, even when one harness loads a directory package and another loads only a rule document.

The v0.1 compiler must retain enough source structure to lower safely without making a claim that every source feature has portable semantics.

## Goals

- Define one versioned compilation graph that library code can create, transform, inspect, serialize, and lower without depending on CLI, filesystem, archive, or harness types.
- Preserve rules and skill packages as the v0.1 semantic units.
- Preserve source provenance and package resources without inventing semantics for scripts, assets, or references.
- Make deterministic selection the only public graph transform in v0.1.
- Produce target-aware loss diagnostics before staging or publication.

## Non-goals

- The graph does not define a new authoring layout.
- The graph does not provide portable agents, hooks, MCP servers, permissions, a plugin ABI, text reducers, metadata rewrites, merging, or deduplication.
- The graph does not load dynamic code, fetch network resources, retain local state, or discover configuration automatically.
- The graph does not define destination paths or publication authority.

## Graph model

`CompilationGraph` replaces `RuletteDocument` as the public library compilation value.

It contains a fixed `graph_version`, an ordered package map, and ordered diagnostics.

All collections use `BTreeMap` or a vector sorted by their documented stable key before serialization, lowering, capability analysis, or diagnostics rendering.

The first public graph version is `0.1`.

The existing JSON and TOML IR are pre-release implementation details and are not accepted as a compatibility format after this change.

`CompilationGraph` owns `Package` values keyed by `PackageId`.

`PackageId` is `pkg_` followed by the SHA-256 hex digest of the graph version, unit kind, canonical semantic identity, and every resource's normalized path, executable bit, and SHA-256 content digest in sorted path order.

It is unique within one graph and is never inferred from a display name.

Each `Package` is the smallest native materialization unit and contains exactly one semantic item or one unsupported native unit.

A standalone rule file is one rule package, a skill directory is one skill package, and a directory containing many rules or skills produces many packages rather than one multi-item package.

Each supported package has a `SourceProvenance`, a normalized relative root, one primary instruction resource, and an ordered resource map.

`SourceProvenance` records a content-safe source label, the archive member path when applicable, and the harness frontend that recognized the package.

The source label is the normalized caller-supplied relative input label or the literal `stdin` label, never input order.

When the caller supplies an absolute input root, the source label is `input_` followed by the SHA-256 digest of that canonical root rather than the host path itself.

Absolute local paths are retained only for diagnostics during a single invocation and are excluded from serialized graphs, staged plans, and emitted artifacts.

`ResourcePath` is slash-separated, relative to its containing package, non-empty, and normalized before it enters the graph.

It rejects an empty component, `.` or `..` component, backslash, NUL or control character, absolute or platform-prefixed spelling, and a path whose normalized form collides with another resource path in the package.

Each `Resource` contains its path, a role, its exact content, and its executable-bit metadata.

The only v0.1 resource roles are `primary-instruction` and `opaque`.

`primary-instruction` is unique within a package and must be UTF-8 Markdown.

`opaque` has no portable semantic meaning and may contain text or arbitrary bytes.

In the Rust API, resource content is `Text(String)` or `Bytes(Vec<u8>)`.

In graph JSON and TOML, text uses `encoding: "utf-8"` with a string payload and bytes use `encoding: "base64"` with canonical RFC 4648 base64 without line wrapping.

The executable bit is a boolean and is false for every resource whose source does not expose executable metadata.

A backend must report a loss if the selected target cannot preserve executable metadata for a resource whose executable bit is true.

The primary instruction resource is the source of truth for instruction text.

The single `Rule` or `Skill` semantic item in a supported package references that resource by `ResourcePath` rather than duplicate a body string.

`Rule` contains its stable identity, optional description, target-resolvable activation, and only portable metadata already defined by the graph contract.

`Skill` contains its stable identity, required discovery description, standard skill metadata, and its primary instruction reference.

`SemanticIdentity` is globally unique in one graph and is `<kind>:<logical-name>`.

Skill logical names use the existing lower-case Agent Skills name grammar.

Rule logical names are the frontend's normalized native rule name and must be non-empty UTF-8 without control characters.

Two packages with the same semantic identity are a hard collision even when their bytes are identical.

Unknown native frontmatter and native configuration fields remain in a namespaced frontend payload attached to the owning package or semantic item.

The payload may be preserved for same-domain handling, but it never changes the portable meaning of a rule or skill.

Agents, hooks, MCP servers, permissions, and unrecognized native files are represented as opaque native package content plus a structured unsupported-semantic diagnostic.

They are not silently converted into a portable v0.1 entity.

## Input discovery and safety

The input layer returns raw artifact observations instead of the current text-only `InputFile` values.

An observation contains bytes, a normalized source-relative path, executable metadata when available, source provenance, and an explicit origin kind for filesystem, stdin, tar, or gzip-compressed tar input.

One invocation accepts at most 10,000 observations, 32 MiB for one resource, and 256 MiB of total resource bytes before returning a hard error without lowering or staging output.

Directory walks use `symlink_metadata` and reject symlinks before reading content.

Archive walks reject every non-regular entry, including symbolic-link and hard-link entries, before a frontend sees them.

Archive walks reject duplicate normalized member paths and GNU or PAX path overrides that do not normalize to one safe relative resource path.

Filesystem hard links are copied as ordinary bytes and never preserved as link identity because a portable walker cannot safely distinguish them on every supported platform and their contents are read only from the declared input root.

An input path is rejected when it is absolute, empty, contains a `.` or `..` component, contains a platform prefix, escapes its input root, or cannot be represented as UTF-8 for a diagnostic-safe normalized path.

Frontends may group observations into a package only when all members share a valid package root.

Package discovery is deterministic because observations are ordered by normalized source identity and resource path before a frontend groups them.

Every frontend classifies an observation as a supported semantic package, retained opaque package resource, unsupported semantic package, unrecognized file warning, or fatal malformed input.

An unsupported semantic package is a loss finding that blocks lowering by default and may proceed only through `--allow-lossy`.

An unrecognized file outside a recognized package is reported as a warning and is not added to the graph.

A malformed input or any safety-invariant failure is always fatal.

No frontend may silently discard an observation after it has recognized an enclosing package.

## Frontend and backend boundaries

A harness-domain frontend owns native discovery, package grouping, primary-instruction identification, native frontmatter decoding, and conversion into graph values.

A frontend may recognize only source layouts that its documented harness actually loads.

The graph kernel owns validation, identity uniqueness, selection, ordering, and loss aggregation.

A target backend owns native file layout, native metadata encoding, resource eligibility, and per-item capability classification.

A backend lowers a selected graph into a deterministic `LoweringPlan` of typed native artifacts without writing files.

The staged-publication design defines artifact paths, destination validation, collision handling, and publication transactions.

Two graph items that contribute the same target artifact class and normalized native path are a hard collision before a lowering plan is returned.

Backends must classify every selected rule, skill, resource, and unsupported semantic as `supported`, `lossy`, or `dropped` with a reason.

The compiler aggregates the worst result for each selected graph item and target.

A target without a native skill-package concept may lower a skill's primary instruction as a rule only when `--allow-lossy` is set.

Its opaque resources are then reported as dropped rather than emitted into an invented target layout.

Opaque native resources are eligible for emission only when the backend explicitly supports the originating frontend's package shape.

Cross-domain lowering always classifies opaque native resources as dropped with their source provenance.

## Selection and loss policy

Selection operates over package identities only because each supported package has exactly one semantic item.

The v0.1 CLI exposes repeatable `--select <package-id>` arguments with exact package-ID matching and no expression language.

No `--select` argument selects every package in stable package-ID order.

One or more `--select` arguments select their deterministic union and an unknown package ID is a hard error.

Selection never mutates instruction text, metadata, activation, resources, package roots, or target payloads.

When a selected semantic item is retained, its owning package, primary instruction, and all package resources remain attached unless the target capability analysis explicitly reports a loss.

The existing public `--filter`, `--exclude`, `--rename`, and `--set` mutations are removed from the v0.1 transform contract.

An explicit transform configuration expresses the same selection as a sorted `select` array of exact package IDs and rejects every previous pipeline form.

Compilation fails before staging when any selected item is `lossy` or `dropped`.

`--allow-lossy` permits staging, retains all loss diagnostics in the compilation result, and records them in the staged publication plan.

`transform` no longer accepts `--strict` because strict loss handling is its default behavior.

`inspect --coverage --strict` remains the explicit non-zero coverage gate for informational inspection.

Transform configuration no longer accepts a global or per-output `strict` field, and it rejects mutation pipeline steps rather than silently translating them.

Graph output to stdout is lossless graph serialization and therefore rejects `--allow-lossy` as inapplicable.

Unsafe paths, malformed metadata, identity collisions, unsupported input syntax, and authorization failures are hard errors that `--allow-lossy` cannot relax.

## Library and CLI migration

Library parsing returns `CompilationGraph` and lowering consumes a selected `CompilationGraph`.

The library exports graph validation, selection, capability analysis, and lowering-plan APIs without exposing clap, paths supplied by the caller, or raw filesystem handles.

The CLI remains a thin adapter that reads observations, invokes a named frontend, applies selection, renders diagnostics, and passes a lowering plan to staging or stdout rendering.

`ir-json`, `ir-toml`, and `schema` are updated to expose the graph contract and graph schema.

Graph JSON and TOML are re-ingestable interchange forms only when their `graph_version` is exactly supported.

Both forms reject unknown structural fields, invalid enum tokens, invalid base64, duplicate package IDs, and non-canonical resource paths.

JSON serialization uses stable struct-field order, sorted maps, UTF-8, and a terminal newline.

TOML serialization uses the same sorted graph order and canonical base64 payloads.

Target activation overrides preserve the current exact-format, then normalized family-alias, then default precedence.

Override keys are normalized by trimmed lower-case comparison and duplicate keys after normalization are a hard graph-validation error.

Overrides for non-core targets remain preserved in the graph and are ignored unless a later backend explicitly names that target.

Old `RuletteDocument` JSON, TOML, and direct entity APIs are removed in this breaking pre-release change.

Explicit transform configuration remains a permitted input artifact, but it cannot bypass graph validation or introduce write authority.

## Test strategy

- Unit-test graph ordering, identity uniqueness, resource encoding, executable metadata, and graph-schema serialization.
- Unit-test path normalization and rejection for empty, absolute, traversal, symlink, archive-link, and package-boundary escape inputs.
- Add frontend fixtures for all five core harnesses that assert package roots, primary instructions, provenance, portable rule or skill items, and retained opaque resources.
- Add backend fixtures that assert deterministic layouts and a capability result for every selected item and resource.
- Verify a native skill package with text, binary, and executable resources round-trips through each target that supports that package layout.
- Verify lowering a package to a rule-only target fails by default and succeeds only with `--allow-lossy` while reporting every dropped resource.
- Verify selection cannot emit a resource without its owning package or lose provenance for a retained item.
- Verify byte-identical graph serialization and lowering across repeated invocations with the same inputs and options.
- Verify package-ID and semantic-identity collisions, target artifact collisions, unsupported-semantic outcomes, selection union and no-match behavior, graph-version rejection, and capability-to-lowering-plan parity for every backend.
- Verify archive duplicate members and each resource budget fails before any graph, stage, or destination output is produced.
- Verify that targets which do not preserve executable metadata reject an executable resource by default and report the same loss under `--allow-lossy`.

## Adversarial review resolution

The review found that a package with many semantic items would detach real instruction bodies during selection.

The design now defines one semantic unit per package and makes native directories collections of packages.

The review found an archive-link contradiction.

The design now rejects archive links and copies filesystem hard-link contents without preserving link identity.

The review found that the old strict and transform-configuration policy would be ambiguous after strict-by-default conversion.

The design now removes transform strictness and mutation configuration, retains inspection strictness, and defines `--allow-lossy` boundaries.

The review found that opaque resources could cross harness boundaries without a capability decision.

The design now permits opaque-resource emission only for an explicitly supported same-domain package shape.

The review found that resource paths and executable metadata needed target-independent validation before lowering.

The design now defines a portable resource-path grammar and makes unpreserved executable metadata an explicit loss.
