# Generated snapshot builds design

## Context

Beads, OpenSpec, and comparable tools can generate files that contain skills, workflows, and instructions.

Those tools own their generation lifecycle, source data, and executable behavior.

Rulette's role begins only after a user, CI system, or another external actor presents the generated result as an existing input snapshot.

The active compilation-graph-kernel change defines safe observations, package-aware graph values, supported native frontends, graph JSON and TOML interchange, identity uniqueness, and deterministic lowering.

The active staged-publication change defines non-mutating check, isolated staging, explicitly authorized apply, and atomic multi-target publication.

This design joins those boundaries into a generated-snapshot build contract without adding a generator integration surface.

## Goals

- Compile any number of homogeneous explicit snapshots into one deterministic graph.
- Accept externally generated content only through an explicitly selected static decoder.
- Collect every graph collision before resolving target backends, capability analysis, lowering, staging, or apply.
- Preserve existing one-or-more target cardinality and atomic multi-target publication.
- Keep generated output derived, one-way, and disposable rather than synchronized state.
- Preserve a fully static binary with no runtime code-loading or generator execution path.

## Non-goals

- This change does not integrate with, invoke, discover, configure, or validate an installation of Beads, OpenSpec, or any other generator.
- This change does not make generator provenance a trust credential or require a generator-specific manifest.
- This change does not define a reverse parser from a destination artifact to a generator source or attempt three-way merges.
- This change does not add initialization, state, history, lockfiles, configuration discovery, plugins, scripts, shared libraries, subprocesses, or network access.
- This change does not add JSON5, YAML, TOML, or code-like transformation configuration.
- This change does not accept plain native standard input until a naming and layout contract exists.
- Per-input frontend routing is deferred to `rulette-5n4`.
- Future declarative configuration is limited to selecting compiled-in transforms and is tracked by `rulette-5n4`.

## Input selection and discovery

CLI syntax parsing may validate the selected source decoder and target arguments before any source I/O.

One invocation selects exactly one statically compiled native frontend through an explicit `--from <frontend>` value or an unambiguous `--from auto` resolution, or it selects one explicitly named Rulette graph reader for JSON or TOML.

`--from auto` is supported only when every discovered native input resolves unambiguously to the same native frontend.

Graph JSON and graph TOML never use `--from auto` and require explicit reader selection.

Every explicit source input in that invocation uses the selected decoder family.

A native invocation accepts existing regular files, directories, tar archives, and gzip-compressed tar archives recognized by the selected frontend.

A graph invocation accepts existing graph JSON documents when `graph-json` is selected or graph TOML documents when `graph-toml` is selected.

Mixed native frontends, graph JSON plus graph TOML, and native plus graph interchange inputs are usage errors in this change.

At most one source may be standard input.

Standard input is accepted only as an explicitly selected tar archive, gzip-compressed tar archive, graph JSON document, or graph TOML document.

Plain native standard input is rejected because a stream has no safe naming or layout contract for native package discovery.

One shared resource ledger spans every explicit input and the sole standard-input stream.

The existing 32 MiB per-resource limit and 256 MiB total invocation limit apply through that one ledger across the complete source set.

The selected frontend must classify every discovered observation as package content, retained unsupported content, an unrecognized-file warning, or fatal malformed input.

An explicitly selected native frontend that produces zero packages after this classification fails with unsupported source syntax.

Native grouping uses the pair of content-safe explicit-input identity and normalized package root.

Resources from distinct snapshots never merge during native grouping, even when they have the same package root spelling.

The graph reader requires explicit JSON or TOML selection and accepts only exactly supported graph versions and strict schema-valid documents.

The compiler represents every decoded package as a transient aggregation candidate containing the validated package unchanged and a separate content-safe outer explicit-input identity.

For graph interchange, the compiler merges candidate packages and diagnostics in stable order without serializing the outer identity into `SourceProvenance` or `CompilationGraph` and without overwriting existing package provenance.

## Aggregate graph and collision policy

The compiler forms candidate packages from every classified native snapshot or decoded graph document before it constructs the aggregate `CompilationGraph`.

It computes each candidate package ID and semantic identity under the graph kernel rules and collects candidates in deterministic indexes by both keys.

It reports every semantic-identity collision group and every package-ID collision group in stable key order before graph construction proceeds.

Each collision group lists the shared key, all candidate package IDs, the existing package provenance, and the separate outer explicit-input identity for every candidate in deterministic outer-identity order.

Byte-identical duplicate packages remain collisions.

The compiler never chooses a winner by input order, directory depth, timestamp, generator name, or hash order.

When both indexes contain collisions, the diagnostic reports all groups rather than stopping after the first one.

Only a collision-free candidate set becomes one `CompilationGraph` with packages and diagnostics in the graph's stable order.

Duplicate package identifiers, unsupported graph versions, malformed graph interchange, unsafe resource paths, archive safety failures, shared-budget failures, and source observations lacking a classification are hard errors.

No loss policy may relax a collision, source-safety failure, schema violation, static execution boundary, or publication authorization failure.

## Multi-target build lifecycle

After complete aggregation and collision validation, the compiler normalizes and deduplicates target spellings under existing target-resolution behavior and requires one or more unique resolved targets from the compiled-in backend registry.

Backend resolution, capability analysis, and lowering never run against a partial graph.

Each backend receives the fully validated selected graph and deterministically produces an independent typed target artifact set without writing files.

The existing all-or-nothing stage and apply transaction spans every selected target artifact set.

`check` evaluates target artifacts against authorized destinations without mutating a stage or destination.

`stage` writes deterministic artifacts and one immutable digest-bearing plan only to a new isolated stage directory.

`apply` verifies the stage plan, every artifact digest, target mapping, and explicit root authority before publishing artifacts atomically under the staged-publication rules.

The direction is one way from explicit source snapshots through check, stage, and apply to destinations.

A completed stage or apply cannot become a source implicitly, and an output path never becomes an input merely because it lies near a source snapshot.

The stateless design cannot prove that a prior separate check ran, so staging repeats complete validation and does not depend on check history.

Manual changes to a live destination are drift in check mode and conflicts in apply mode unless the caller explicitly requests existing replacement behavior.

Manual changes are never parsed, merged, or propagated to the graph, source snapshot, generator, or a later stage.

## Artifact ownership and reproducibility

Rulette considers native target files produced by this flow build artifacts.

The only source of a later build is the next explicit input set supplied by the caller.

The build retains no output-to-source association or mutable synchronization history.

Identical valid inputs, selected decoder, selected targets, compiler version, and compiled-in mappings produce byte-identical graph and target artifact sets in stable order.

Collision diagnostics render the existing package provenance and separate outer explicit-input identity from their transient candidates.

Capability and lowering findings retain only the existing package `SourceProvenance`.

The outer explicit-input identity is neither serialized into `SourceProvenance` or `CompilationGraph` nor used to overwrite package provenance.

Target artifacts carry only target-native content and metadata defined by their backends.

They do not carry a hidden Rulette control channel, a source-path escape, a root authority token, or an instruction to execute a generator.

## Static security boundary

The Rulette binary contains every accepted parser, frontend, backend, transform, and mapping implementation.

It may read caller-supplied input bytes and perform the existing explicitly authorized stage or apply filesystem operations.

It must not spawn a process, execute shell text, evaluate scripts, invoke an interpreter, load a dynamic or shared library, discover a plugin, access a network service, or read a generator-specific runtime configuration.

Text that looks like a command, hook, plugin declaration, script, library reference, or external tool invocation has no executable effect in a Rulette build.

Supported package resources can be preserved as opaque data only when the existing frontend and backend contracts permit that retention and lowering.

Every source and destination filesystem operation remains constrained by the graph input-safety and staged-publication authorization rules.

## Alternatives considered

### Generator execution adapter

Rulette could offer a `--generate` option that invokes Beads, OpenSpec, or a registered command before compilation.

This would make builds dependent on external executables, their versions, environment state, and side effects, so it is rejected.

### Bidirectional synthetic synchronization

Rulette could scan generated destinations, reconcile edits, and write changes back into source snapshots or generator metadata.

This would require hidden history and an arbitrary conflict-resolution authority, so it is rejected.

### Per-input runtime routing

Rulette could infer or select a frontend independently for every explicit input.

This is deferred to `rulette-5n4` because routing policy needs a separate deterministic and user-visible selection contract.

### Generated transform configuration

Rulette could consume a JSON5, YAML, TOML, or code-like file that declares arbitrary generator or transform behavior.

This is out of scope and any future declarative configuration may only choose transforms compiled into Rulette, as tracked by `rulette-5n4`.

## Testing strategy

- Permute a homogeneous native or graph input set and assert identical graph ordering, collision diagnostics, target artifacts, and stage-plan digests.
- Supply two snapshots defining the same semantic identity or package ID and assert every collision group is reported before backend work, stage creation, or destination mutation.
- Supply native snapshots with equal package roots and assert their resources remain in separate groupings until graph aggregation.
- Supply mixed frontend families, graph encodings, and native-plus-interchange inputs and assert each is rejected before discovery or lowering as applicable.
- Supply zero-package native content, unclassified observations, plain native standard input, and multiple standard-input arguments and assert deterministic source errors.
- Supply tar, gzip-compressed tar, graph JSON, and graph TOML through explicitly selected standard input and assert they share one 32 MiB per-resource and 256 MiB total ledger across the complete source set.
- Supply native inputs that resolve to different frontends under `--from auto` and assert an ambiguity error, then assert graph JSON and graph TOML require explicit reader selection.
- Supply repeated spelling variants of one target and assert deduplication before the one-or-more unique resolved target check.
- Supply a generated-looking script, hook, dynamic-library name, or plugin manifest and assert no subprocess, code loader, or network behavior occurs.
- Modify a destination after staging and assert that check reports drift and apply rejects it without `--replace`, then assert that no merge path changes the source graph.
- Stage several target artifact sets and assert apply preflight and rollback remain atomic across the full target set.

## Adversarial review resolution

- Finding: The previous one-target rule contradicted the active staged-publication contract; disposition: this change preserves one-or-more target selection and atomic multi-target stage and apply.
- Finding: Per-input autodetection could create nondeterministic mixed-format aggregation; disposition: one invocation selects one homogeneous native frontend or explicit graph JSON or TOML reader, and per-input routing is deferred to `rulette-5n4`.
- Finding: Equal package roots from distinct snapshots could merge resources before collision validation; disposition: native grouping keys include content-safe explicit-input identity and normalized package root.
- Finding: Fail-fast collision handling could conceal related collisions; disposition: deterministic pre-construction indexes collect and report all semantic-identity and package-ID collision groups in stable order.
- Finding: Per-input limits could permit aggregate resource exhaustion; disposition: one shared ledger enforces the existing 32 MiB per-resource and 256 MiB total invocation limits across every explicit input and the sole standard-input stream.
- Finding: Plain native standard input lacks a naming and layout contract; disposition: at most one standard-input source is accepted and it must be an explicitly selected tar, gzip-compressed tar, graph JSON, or graph TOML document.
- Finding: A selected frontend could silently accept irrelevant input; disposition: every observation requires a classification and a selected native frontend with zero packages fails as unsupported source syntax.
- Finding: The prior order implied backend work could occur before complete validation; disposition: syntax parsing may precede I/O, but backend resolution, capability analysis, and lowering begin only after aggregation and collision validation complete.
- Finding: Outer graph-input provenance might be mistaken for a serialized schema guarantee or could overwrite package provenance; disposition: each transient aggregation candidate carries the validated package unchanged plus a separate outer identity that collision diagnostics render alongside existing package provenance in deterministic outer-identity order, while lowering findings retain only package `SourceProvenance`.
- Finding: Auto-detected native input families could be inconsistent and graph encodings could be silently inferred; disposition: `--from auto` succeeds only when every discovered native input resolves unambiguously to one frontend, while graph JSON and graph TOML require explicit reader selection.
- Finding: Duplicate target spellings could violate target cardinality or repeat output work; disposition: existing normalization deduplicates spellings before one-or-more unique resolved targets are required.
- Finding: Prior delivery commitments exceeded this Bead's scope; disposition: the proposal now uses validation rationale only and contains no delivery work.
