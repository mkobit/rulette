## MODIFIED Requirements

### Requirement: Unified typed Intermediate Representation envelope

Rulette SHALL represent source configurations with a versioned `CompilationGraph` rather than a `RuletteDocument` entity envelope.
`CompilationGraph` SHALL contain an exact supported `graph_version`, an ordered map of packages keyed by `PackageId`, and ordered diagnostics.
The graph kernel SHALL not depend on CLI, filesystem, archive, or harness-specific types.
Every serialized graph SHALL use stable field ordering and collections ordered by their documented stable key.
Graph JSON and TOML input SHALL be accepted only when the graph version is exactly supported and the input has no unknown structural fields, invalid enum tokens, invalid base64 content, duplicate package identifiers, or non-canonical resource paths.

#### Scenario: Parsing a supported graph interchange document

- **WHEN** Rulette receives graph JSON or TOML with `graph_version: "0.1"` and canonical package content
- **THEN** it SHALL construct a `CompilationGraph` with the same package identities and resource bytes.

#### Scenario: Rejecting an incompatible graph interchange document

- **WHEN** Rulette receives graph JSON or TOML whose graph version is unsupported or whose structural fields are unknown
- **THEN** it SHALL reject the document before selection or lowering.

#### Scenario: Rule entity representation

- **WHEN** a source rule is parsed into the graph
- **THEN** it SHALL produce a rule package with one rule semantic item and a referenced primary-instruction resource.

#### Scenario: MCP server entity normalization

- **WHEN** an MCP server configuration is recognized by a source frontend
- **THEN** it SHALL produce an unsupported native package and structured unsupported-semantic diagnostic
- **AND** SHALL NOT be normalized to a portable graph entity.

### Requirement: Strict identity uniqueness

The graph validator SHALL enforce unique `PackageId` and `SemanticIdentity` values across one `CompilationGraph`.
`PackageId` SHALL be `pkg_` followed by the SHA-256 hex digest of the graph version, package unit kind, canonical semantic identity, and every resource's normalized path, executable bit, and SHA-256 content digest in sorted path order.
`SemanticIdentity` SHALL be `<kind>:<logical-name>` and SHALL be unique even when colliding package bytes are identical.
A skill logical name SHALL use the supported Agent Skills name grammar.
A rule logical name SHALL be a non-empty UTF-8 frontend-normalized native rule name without control characters.

#### Scenario: Semantic identity collision failure

- **WHEN** two packages have the same `SemanticIdentity`
- **THEN** Rulette SHALL reject the graph before lowering
- **AND** SHALL identify the colliding semantic identity.

#### Scenario: Identity collision failure

- **WHEN** multiple inputs produce packages with the same semantic identity
- **THEN** Rulette SHALL abort before staging or publication
- **AND** SHALL surface the colliding identifier.

## ADDED Requirements

### Requirement: Package-aware graph representation

Each graph package SHALL be the smallest native materialization unit and SHALL contain exactly one portable semantic item or one unsupported native unit.
A standalone rule file SHALL produce one rule package.
A native skill directory SHALL produce one skill package.
A native directory containing multiple rules or skills SHALL produce one package for each semantic item rather than a package with multiple semantic items.
Each supported package SHALL have source provenance, a normalized relative package root, exactly one UTF-8 Markdown primary-instruction resource, and an ordered resource map.
A rule or skill semantic item SHALL reference its primary instruction by resource path instead of duplicating instruction text.

#### Scenario: Separating two rules in one native directory

- **WHEN** a frontend recognizes two native rule files in one directory
- **THEN** it SHALL create two rule packages with distinct package and semantic identities
- **AND** selection of one package SHALL not include the other rule.

#### Scenario: Preserving a native skill package boundary

- **WHEN** a frontend recognizes a skill directory with a primary instruction and auxiliary files
- **THEN** it SHALL create one skill package containing the primary instruction and every retained auxiliary resource.

### Requirement: Resource and provenance safety

Every graph resource SHALL have a non-empty slash-separated `ResourcePath` relative to its package.
Rulette SHALL reject a resource path containing an empty, `.` or `..` component, a backslash, a NUL or control character, an absolute or platform-prefixed spelling, or a normalized collision within its package.
Each resource SHALL preserve exact text or byte content, a `primary-instruction` or `opaque` role, and executable-bit metadata.
Primary instructions SHALL be UTF-8 Markdown and opaque resources MAY contain arbitrary bytes.
Graph JSON and TOML SHALL encode text resources as UTF-8 strings and byte resources as canonical unwrapped RFC 4648 base64.
The graph SHALL record content-safe source provenance containing the recognized frontend, a normalized caller-supplied relative input label or `stdin`, and an archive member path when applicable.
When the caller supplied input root is absolute, serialized provenance SHALL use `input_` followed by the SHA-256 digest of its canonical root instead of an absolute host path.

#### Scenario: Rejecting a resource that escapes a package

- **WHEN** a resource path contains `../` or an absolute path spelling
- **THEN** Rulette SHALL reject the input before constructing a graph package.

#### Scenario: Preserving a binary opaque resource

- **WHEN** a recognized package contains a retained opaque binary resource
- **THEN** the graph SHALL preserve its bytes, normalized path, provenance, and executable-bit metadata without assigning portable semantics.

### Requirement: Safe observation discovery

The input layer SHALL provide frontends raw observations containing bytes, normalized source-relative paths, executable metadata when available, source provenance, and an origin kind of filesystem, standard input, tar, or gzip-compressed tar.
One invocation SHALL accept at most 10,000 observations, 32 MiB for one resource, and 256 MiB of total resource bytes.
Directory discovery SHALL reject symlinks before reading content.
Archive discovery SHALL reject every non-regular entry, including symbolic-link and hard-link entries, duplicate normalized member paths, and unsafe GNU or PAX path overrides.
Filesystem hard links SHALL be copied as ordinary bytes and SHALL not preserve link identity.
Malformed input and safety-invariant failures SHALL be hard errors that no loss policy can relax.

#### Scenario: Rejecting an archive symbolic link

- **WHEN** a tar input contains a symbolic-link or hard-link member
- **THEN** Rulette SHALL fail before a frontend receives an observation from that archive.

#### Scenario: Rejecting an input resource budget overflow

- **WHEN** input discovery exceeds the resource count, individual-resource byte, or total-byte budget
- **THEN** Rulette SHALL fail without constructing a graph, lowering plan, stage, or destination output.

### Requirement: Explicit unsupported-semantic representation

Frontends SHALL classify every recognized observation as a supported semantic package, retained opaque package resource, unsupported semantic package, unrecognized-file warning, or fatal malformed input.
Agents, hooks, MCP servers, permissions, and other unsupported native semantic units SHALL be represented as opaque native package content with a structured unsupported-semantic diagnostic.
An unrecognized file outside a recognized package SHALL produce a warning and SHALL not be added to the graph.
No frontend SHALL silently discard an observation after recognizing its enclosing package.

#### Scenario: Unsupported native agent is not made portable

- **WHEN** a core frontend recognizes a native agent definition
- **THEN** it SHALL retain the recognized content as an unsupported native package with a structured diagnostic
- **AND** SHALL NOT construct a portable v0.1 agent entity.

### Requirement: Deterministic capability-aware lowering

Each target backend SHALL lower a selected graph into a deterministic typed lowering plan without writing files.
The backend SHALL classify every selected rule, skill, resource, and unsupported semantic as `supported`, `lossy`, or `dropped` with a stable reason.
The compiler SHALL aggregate the worst classification for each selected graph package and target before staging or publication.
Two selected graph items that produce the same target artifact class and normalized native artifact path SHALL cause a hard collision.
A backend SHALL report loss when it cannot preserve a selected resource's executable bit.
A target without a native skill-package concept MAY lower a skill's primary instruction as a rule only under the explicit lossy policy and SHALL classify every unrepresentable opaque resource as dropped.
Opaque resource emission SHALL be allowed only when the backend explicitly supports the originating frontend's package shape.
Cross-domain opaque resource lowering SHALL always be classified as dropped with source provenance.

#### Scenario: Strict skill lowering to a rule-only target

- **WHEN** a selected skill package has opaque resources and the target has no native skill-package concept
- **THEN** the backend SHALL report the primary-instruction degradation and every dropped opaque resource before publication.

#### Scenario: Native artifact collision

- **WHEN** two selected packages lower to the same target artifact class and normalized native artifact path
- **THEN** Rulette SHALL fail before returning a lowering plan.

### Requirement: Target activation override resolution

Rule activation overrides SHALL resolve in exact target-name, normalized target-family alias, then default order.
Override keys SHALL be normalized by trimmed lower-case comparison.
Duplicate override keys after normalization SHALL be a graph-validation error.
Overrides for targets without a registered v0.1 backend SHALL remain in the graph and SHALL be ignored unless a later backend names that target.

#### Scenario: Resolving a normalized target-family override

- **WHEN** a rule has no exact target override but has a normalized family-alias override for the selected target
- **THEN** the backend SHALL use the family-alias override before falling back to the default.
