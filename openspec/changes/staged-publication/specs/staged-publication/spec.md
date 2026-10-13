## ADDED Requirements

### Requirement: Isolated deterministic staging

The staging mode SHALL isolate compilation from live destinations and publish only a complete stage under the following root-binding and failure guarantees.

#### Scenario: Staging isolates live destinations and binds explicit roots

- **WHEN** any caller compiles selected native targets into a stage
- **THEN** The `transform --stage <directory>` mode SHALL compile selected native targets into an isolated stage root without reading or mutating a live native destination.
- **AND** The invocation SHALL require one explicit `--project-root <root>` for any set of project-scoped targets and one `--user-root <target>=<root>` for every user-scoped target.
- **AND** Those roots SHALL be used only to resolve and bind target mappings, and SHALL NOT authorize a later mutation.

#### Scenario: Staging publishes an exclusively owned complete directory

- **WHEN** any staging invocation creates or publishes its stage
- **THEN** The requested stage directory SHALL not exist before compilation starts.
- **AND** The compiler SHALL create the stage in an exclusively created sibling temporary directory only after parsing, graph validation, selection, capability analysis, and lowering succeed.
- **AND** The compiler SHALL publish the complete stage by an atomic no-replace rename where the platform supports it.
- **AND** On failure, the compiler SHALL remove only its owned temporary directory and SHALL NOT create the requested stage root.

#### Scenario: Staging does not publish a live destination

- **WHEN** a caller stages a project-scoped target with `transform <inputs> --target <format>@project --project-root <root> --stage <directory>`
- **THEN** Rulette SHALL write only below the new stage directory
- **AND** SHALL NOT create, replace, or remove a file below `<root>`.

#### Scenario: An existing stage directory is rejected

- **WHEN** a caller supplies `--stage <directory>` and that directory already exists
- **THEN** Rulette SHALL exit with a non-zero status
- **AND** SHALL NOT alter the existing directory or any live destination.

### Requirement: Reviewable versioned publication plan

Every successful stage SHALL provide a versioned canonical publication plan with the following artifact, privacy, and integrity guarantees.

#### Scenario: Successful stages use a canonical versioned plan

- **WHEN** any staging invocation succeeds
- **THEN** Every successful stage SHALL contain exactly an `artifacts/` directory and a `rulette.plan.json` file at its root.
- **AND** The plan SHALL use the versioned canonical JSON schema defined for `plan_version` `0.1`.
- **AND** Canonical plan JSON SHALL use fixed struct field order, sorted arrays, UTF-8 strings, integer byte lengths, and no floating-point or free-form object values.

#### Scenario: Plans record typed artifacts without authority or live paths

- **WHEN** any successful stage records its publication plan
- **THEN** The plan SHALL record the compiler version, graph version and digest, target mapping-version identifiers, hashed root identities, accepted loss findings, and ordered artifact entries.
- **AND** Each artifact entry SHALL record a stable entry identifier, target, mapping version, logical scope, relative stage artifact path, native artifact class, normalized native artifact path, SHA-256 digest, byte length, executable-bit metadata, source package identity, and capability findings.
- **AND** The plan SHALL contain no absolute root path, home path, environment-expanded path, credential, managed path, or authority token.

#### Scenario: Plan digests cover the exact canonical bytes

- **WHEN** any staging invocation succeeds and reports its plan digest
- **THEN** The compiler SHALL print the SHA-256 digest of the exact canonical plan bytes after staging succeeds.
- **AND** The plan digest SHALL cover the exact bytes that apply reads rather than a reserialized interpretation of the plan.

#### Scenario: A plan identifies its staged artifacts without live paths

- **WHEN** staging succeeds for one or more native targets
- **THEN** `rulette.plan.json` SHALL identify each staged artifact by its typed mapping descriptor and relative stage path
- **AND** SHALL NOT contain the absolute project root or user root used during staging.

#### Scenario: A malformed plan shape is not accepted

- **WHEN** an apply or plan-mode check reads a plan with an unsupported version, duplicate entry, free-form field, or invalid artifact path
- **THEN** Rulette SHALL reject the plan before resolving a destination or mutating the filesystem.

### Requirement: Explicit root authority and verified scope mappings

Staging, checking, and applying SHALL use the following verified scope mappings, explicit root identities, and publication authority boundaries.

#### Scenario: Scope mappings bind explicit root identities

- **WHEN** any staging, checking, or applying invocation runs
- **THEN** The v0.1 publication scope enum SHALL contain only `project` and `user`.
- **AND** Logical scope, target-native mapping, and apply authority SHALL be modeled as distinct values.
- **AND** Every core target SHALL provide one compiled-in project mapping.
- **AND** A user mapping SHALL be available only when its native root and layout are documented by the target vendor and covered by conformance fixtures.
- **AND** The stage plan SHALL bind each mapping to the SHA-256 root identity of the opened explicit root, where the identity encodes the platform canonical root spelling together with its opened volume and file identity using length-prefixed fields.

#### Scenario: Publication requires explicit matching root authority

- **WHEN** any staging, checking, or applying invocation runs
- **THEN** Apply and plan-mode check SHALL require `--allow-project-root <root>` for each project entry and `--allow-user-root <target>=<root>` for each user entry.
- **AND** An authorized root SHALL exactly match the corresponding plan root identity.
- **AND** The plan, a transform configuration, environment variables, target defaults, and prior runs SHALL NOT grant publication authority.
- **AND** Rulette SHALL NOT search for a repository root or read a home-root environment variable when staging, checking, or applying.

#### Scenario: Unsupported scopes and managed locations are rejected

- **WHEN** any staging, checking, or applying invocation runs
- **THEN** Rulette SHALL reject `local`, `enterprise`, `managed`, `system`, and every other non-v0.1 scope before mapping or filesystem access.
- **AND** Rulette SHALL reject any mapping to a known managed or system location even when the caller can write to that location.

#### Scenario: A plan cannot expand publication authority

- **WHEN** a plan requests project publication under a root different from `--allow-project-root <root>`
- **THEN** Rulette SHALL reject apply
- **AND** SHALL NOT write any destination.

#### Scenario: An undocumented user mapping is unavailable

- **WHEN** a caller requests a user-scoped Cursor target
- **THEN** Rulette SHALL report the user mapping as unavailable
- **AND** SHALL NOT infer an output location from user configuration or environment state.

### Requirement: Plan integrity preflight and safe destination resolution

Apply and plan-mode check SHALL enforce the following integrity preflight, typed destination resolution, and handle-relative path-safety guarantees.

#### Scenario: Preflight verifies plan bytes and typed staged artifacts

- **WHEN** any Rulette invocation performs integrity preflight or destination resolution
- **THEN** `transform --apply <plan> --expect-plan-sha256 <digest>` SHALL require the expected digest to match the exact bytes of the plan before parsing or trusting plan contents.
- **AND** Apply and plan-mode check SHALL open the stage root and authorized roots through stable file handles and verify every referenced staged artifact is a regular file whose digest, length, and executable metadata match the plan.
- **AND** Before a destination is read or written, Rulette SHALL validate the artifact's target, mapping version, artifact class, and normalized native artifact path against the compiled-in mapping.
- **AND** The publication library, rather than a plan or backend-provided absolute path, SHALL be the only layer that resolves a typed native artifact into a filesystem destination.

#### Scenario: Destination access rejects unsafe paths and never follows links

- **WHEN** any Rulette invocation validates or accesses a destination
- **THEN** Rulette SHALL reject duplicate destinations, ancestor-descendant destination conflicts, paths outside the authorized root, invalid path components, platform-normalization collisions, reserved platform names, repository-control namespaces, and `.git` paths.
- **AND** Rulette SHALL reject a destination or parent component that is a symlink, junction, reparse point, directory where a regular file is required, or another non-regular file.
- **AND** Destination traversal, temporary-file creation, replacement, and rollback SHALL be relative to pre-opened authorized root handles and SHALL never follow a link.

#### Scenario: Platform traversal enforces root boundaries

- **WHEN** any Rulette invocation traverses filesystem destinations on a supported or unsupported platform
- **THEN** Unix implementations SHALL use component-by-component no-follow descriptor traversal and reject mount-boundary traversal.
- **AND** Windows implementations SHALL use handle-relative traversal and reject reparse points and volume-boundary traversal.
- **AND** Rulette SHALL report publication as unsupported on a platform where these invariants cannot be enforced.

#### Scenario: A substituted artifact prevents publication

- **WHEN** a staged artifact is modified, removed, substituted, or has different executable metadata after staging
- **THEN** apply SHALL fail before reading or changing any destination.

#### Scenario: An arbitrary plan path is rejected

- **WHEN** a plan contains a normalized-looking native artifact path that is not producible by its target mapping and artifact class
- **THEN** Rulette SHALL reject the plan before filesystem traversal.

### Requirement: Conflict-aware transactional publication

Apply SHALL enforce the following conflict preflight, transactional mutation, ownership-aware rollback, and process-lifetime guarantees.

#### Scenario: All destinations are classified before mutation

- **WHEN** any Rulette invocation runs, including one that supplies `--replace` outside apply mode
- **THEN** Apply SHALL preflight every destination before creating a directory or mutating a destination.
- **AND** Rulette SHALL classify each verified destination as absent, unchanged, or conflicting by comparing exact bytes and target-representable executable metadata.
- **AND** An unchanged destination SHALL not be rewritten.
- **AND** An absent destination SHALL be eligible for creation.
- **AND** A conflicting destination SHALL abort the whole apply unless `--replace` is explicit.
- **AND** `--replace` SHALL be valid only with `--apply`.

#### Scenario: Changed destinations use atomic replacement and preserve originals

- **WHEN** any apply invocation mutates a destination after successful preflight
- **THEN** After preflight succeeds, apply SHALL write each changed destination through a same-directory temporary regular file and atomic rename.
- **AND** Before replacing a destination, apply SHALL record its exact original bytes, executable metadata, and file identity.

#### Scenario: Publication failure rolls back only owned changes

- **WHEN** any apply invocation runs, including when a mutation or rollback fails
- **THEN** If a later mutation fails, apply SHALL delete files it created, remove empty directories it created, and restore replaced files only when their identity and digest still match the file written by that apply.
- **AND** If rollback encounters a concurrent change or fails, Rulette SHALL report both the rollback failure and the original publication failure and SHALL NOT overwrite a third-party change.
- **AND** The transaction guarantee applies to failures observed by the running process and does not guarantee atomicity across crashes, power loss, stale destination deletion, or non-cooperating concurrent writers.

#### Scenario: A differing destination requires replacement authority

- **WHEN** apply preflight finds an existing regular file with different bytes or executable metadata
- **AND** `--replace` is absent
- **THEN** Rulette SHALL fail the entire apply before changing any destination.

#### Scenario: A late publication failure rolls back owned changes

- **WHEN** apply creates or replaces one or more destinations and a later destination write fails
- **THEN** Rulette SHALL restore only the destinations it can prove it still owns
- **AND** SHALL leave no partial successful publication created by that apply when rollback succeeds.

### Requirement: Non-mutating publication checks

Source-mode and plan-mode checks SHALL enforce the following explicit root, integrity, drift-reporting, and non-mutation guarantees.

#### Scenario: Source-mode checks use staging semantics and explicit roots

- **WHEN** any check invocation uses source inputs
- **THEN** `transform --check` with source inputs SHALL compute the same native lowering and destination classification as staging without creating a stage directory or mutating a destination.
- **AND** Source-mode check SHALL require the same explicit project and user roots as staging.

#### Scenario: Plan-mode checks verify integrity and apply authority

- **WHEN** any check invocation uses an apply plan
- **THEN** `transform --check` with an apply plan SHALL verify plan integrity, root authority, mapping validity, and destination drift without publishing.
- **AND** Plan-mode check SHALL require the same explicit allow-root arguments as apply.

#### Scenario: Checks report drift without temporary filesystem state

- **WHEN** any source-mode or plan-mode check runs, including when parsing or preflight fails
- **THEN** If a check finds an absent or conflicting destination, Rulette SHALL exit with a non-zero status after reporting the classification.
- **AND** Check SHALL not create temporary files or directories.

#### Scenario: A plan-mode check reports drift without mutation

- **WHEN** a valid authorized plan has a destination that is absent or conflicting
- **THEN** `transform --check` SHALL exit non-zero
- **AND** SHALL not write a destination, temporary file, or directory.
