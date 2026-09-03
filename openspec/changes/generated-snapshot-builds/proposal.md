## Why

Tools such as Beads and OpenSpec can generate skill and workflow files after a user or CI system invokes them.
Rulette needs to compile those existing snapshots without acquiring authority to execute their generators, load their code, or synchronize their state.

Treating generated outputs as a bidirectional synchronization surface would make manual destination edits, generator state, and machine-local configuration implicit inputs.
That violates Rulette's static, deterministic, and stateless build contract.

## What changes

- Define generated skill and workflow snapshots as ordinary explicit source inputs, not executable integrations.
- Select one statically compiled native frontend or one explicitly named graph JSON or graph TOML reader for one homogeneous input set.
- Accept any number of documents in the selected format and aggregate their packages and diagnostics deterministically into one `CompilationGraph`.
- Carry every decoded package in a transient aggregation candidate with a separate content-safe outer explicit-input identity that never changes graph package provenance.
- Reject mixed native frontends, graph encodings, and native-plus-interchange source sets in this change.
- Reject every package-ID and semantic-identity collision before capability analysis, staging, or publication, even when colliding content is byte-identical.
- Preserve existing one-or-more target cardinality and atomic multi-target staging and apply after aggregate graph validation.
- Treat emitted native files as derived build artifacts whose manual edits are detected as drift or conflicts but are never read back, merged, or used to modify source snapshots.
- Require every accepted source format, frontend, backend, and transform to be compiled into the static Rulette binary.

## Scope

- Consume externally produced files, directories, tar archives, gzip-compressed tar archives, and explicitly selected graph JSON or TOML snapshots through already-supported decoders.
- Allow at most one standard-input stream, accepting only an explicitly selected tar, gzip-compressed tar, graph JSON, or graph TOML source.
- Combine multiple explicit inputs into one validated graph in stable order independent of input argument order.
- Produce one or more independent target artifact sets from that graph and use the existing check, staging, and apply safety boundaries for atomic publication.
- Report existing package provenance and separate content-safe outer-input identity in collision diagnostics only.

## Non-goals

- Rulette does not execute `bd`, `openspec`, a generator, a shell command, a script, a hook, or a plugin.
- Rulette does not import a destination tree, infer sources from generated output, merge manual destination edits, or maintain synchronization history.
- Rulette does not add initialization, local state, persistent configuration, runtime plugin discovery, dynamic libraries, network access, or shared-library loading.
- Rulette does not accept plain native standard input until a naming and layout contract exists.
- Rulette does not add JSON5, YAML, TOML, or code-like transform configuration in this change.
- Future declarative configuration may select only transforms compiled into Rulette and remains tracked by `rulette-5n4`.
- Per-input frontend routing remains deferred to `rulette-5n4`.

## Capabilities

### New capabilities

- `generated-snapshot-builds`: Define homogeneous explicit snapshot input handling, deterministic graph aggregation, collision collection, derived artifact drift semantics, and the static execution boundary.

### Modified capabilities

- `transform-pipeline`: Define generated-snapshot compilation as a one-way source-to-target operation that aggregates before backend work, preserves existing multi-target publication, and never reverses destination artifacts into sources.

## Alternatives considered

### Run external generators during transform

Rulette could invoke `bd`, `openspec`, or a configured generator before parsing its output.

This alternative is rejected because execution makes input contents, tool versions, environment variables, network behavior, and filesystem authority runtime dependencies of the Rulette binary.

### Synchronize sources and destinations bidirectionally

Rulette could detect manual output edits and merge them back into source snapshots or generator state.

This alternative is rejected because there is no unambiguous merge authority or source of truth, and a stateless compiler cannot safely retain the history needed to resolve conflicts.

### Route each input through a different frontend

Rulette could infer or select a frontend independently for every explicit input.

This alternative is deferred to `rulette-5n4` because routing policy needs a separate deterministic and user-visible selection contract.

## Security rationale

An external generator is outside Rulette's trust boundary and must run before Rulette receives an immutable input snapshot.

Rulette parses bytes and metadata only, subject to the existing 32 MiB per-resource and 256 MiB total invocation limits through one shared ledger across all inputs plus existing path, archive, graph-schema, and semantic-validation limits.

It does not interpret source content as executable code or grant source, graph, plan, transform configuration, or output artifacts filesystem publication authority.

Staging and apply retain their existing explicit-root authorization, digest verification, no-follow traversal, conflict, and rollback rules.

## Impact

- The graph compiler gains homogeneous multi-input aggregation and deterministic all-group collision reporting.
- The CLI remains a thin adapter that parses syntax before I/O and delegates discovery, aggregation, backend resolution, lowering, checking, staging, and apply behavior to the library.
- Native skill frontends and explicitly selected graph interchange readers remain the only accepted snapshot decoders in this scope.
- Destination artifacts remain disposable build outputs that must be regenerated from explicit sources rather than hand-maintained synchronization peers.

## Validation rationale

- Conformance tests SHALL show that the same homogeneous source set produces byte-identical graph packages, diagnostics, and target artifact sets regardless of input argument order.
- Collision tests SHALL collect and report every package-ID and semantic-identity collision group before a stage directory or live destination is created.
- Collision tests SHALL distinguish unchanged package provenance from transient outer input identity and SHALL not attach the latter to lowering findings.
- Input-boundary tests SHALL reject mixed decoder families, ambiguous `--from auto` resolution, plain native standard input, multiple standard-input streams, zero-package native parses, and source observations lacking a required classification.
- Resource-limit tests SHALL enforce the existing 32 MiB per-resource and 256 MiB total limits through one ledger across the complete source set.
- Target tests SHALL normalize and deduplicate repeated target spellings before requiring one or more unique resolved targets.
- Isolation tests SHALL prove that a source file resembling a generator command, hook, script, plugin, or dynamic library is retained only as permitted opaque content and is never executed or loaded.
- Drift tests SHALL prove that a manual destination edit is reported by check or apply conflict handling and is never merged into a later build.
- Integration tests SHALL prove that a directory generated outside Rulette can be supplied as an explicit input and compiled without invoking its producer.
