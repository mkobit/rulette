## ADDED Requirements

### Requirement: Isolated deterministic staging

The `transform --stage <directory>` mode SHALL compile selected native targets into an isolated stage root without reading or mutating a live native destination.
The invocation SHALL require one explicit `--project-root <root>` for any set of project-scoped targets and one `--user-root <target>=<root>` for every user-scoped target.
Those roots SHALL be used only to resolve and bind target mappings, and SHALL NOT authorize a later mutation.
The requested stage directory SHALL not exist before compilation starts.
The compiler SHALL create the stage in an exclusively created sibling temporary directory only after parsing, graph validation, selection, capability analysis, and lowering succeed.
The compiler SHALL publish the complete stage by an atomic no-replace rename where the platform supports it.
On failure, the compiler SHALL remove only its owned temporary directory and SHALL NOT create the requested stage root.

#### Scenario: Staging does not publish a live destination

- **WHEN** a caller stages a project-scoped target with `transform <inputs> --target <format>@project --project-root <root> --stage <directory>`
- **THEN** Rulette SHALL write only below the new stage directory
- **AND** SHALL NOT create, replace, or remove a file below `<root>`.

#### Scenario: An existing stage directory is rejected

- **WHEN** a caller supplies `--stage <directory>` and that directory already exists
- **THEN** Rulette SHALL exit with a non-zero status
- **AND** SHALL NOT alter the existing directory or any live destination.

### Requirement: Reviewable versioned publication plan

Every successful stage SHALL contain exactly an `artifacts/` directory and a `rulette.plan.json` file at its root.
The plan SHALL use the versioned canonical JSON schema defined for `plan_version` `0.1`.
Canonical plan JSON SHALL use fixed struct field order, sorted arrays, UTF-8 strings, integer byte lengths, and no floating-point or free-form object values.
The plan SHALL record the compiler version, graph version and digest, target mapping-version identifiers, hashed root identities, accepted loss findings, and ordered artifact entries.
Each artifact entry SHALL record a stable entry identifier, target, mapping version, logical scope, relative stage artifact path, native artifact class, normalized native artifact path, SHA-256 digest, byte length, executable-bit metadata, source package identity, and capability findings.
The plan SHALL contain no absolute root path, home path, environment-expanded path, credential, managed path, or authority token.
The compiler SHALL print the SHA-256 digest of the exact canonical plan bytes after staging succeeds.
The plan digest SHALL cover the exact bytes that apply reads rather than a reserialized interpretation of the plan.

#### Scenario: A plan identifies its staged artifacts without live paths

- **WHEN** staging succeeds for one or more native targets
- **THEN** `rulette.plan.json` SHALL identify each staged artifact by its typed mapping descriptor and relative stage path
- **AND** SHALL NOT contain the absolute project root or user root used during staging.

#### Scenario: A malformed plan shape is not accepted

- **WHEN** an apply or plan-mode check reads a plan with an unsupported version, duplicate entry, free-form field, or invalid artifact path
- **THEN** Rulette SHALL reject the plan before resolving a destination or mutating the filesystem.

### Requirement: Explicit root authority and verified scope mappings

The v0.1 publication scope enum SHALL contain only `project` and `user`.
Logical scope, target-native mapping, and apply authority SHALL be modeled as distinct values.
Every core target SHALL provide one compiled-in project mapping.
A user mapping SHALL be available only when its native root and layout are documented by the target vendor and covered by conformance fixtures.
The stage plan SHALL bind each mapping to the SHA-256 root identity of the opened explicit root, where the identity encodes the platform canonical root spelling together with its opened volume and file identity using length-prefixed fields.
Apply and plan-mode check SHALL require `--allow-project-root <root>` for each project entry and `--allow-user-root <target>=<root>` for each user entry.
An authorized root SHALL exactly match the corresponding plan root identity.
The plan, a transform configuration, environment variables, target defaults, and prior runs SHALL NOT grant publication authority.
Rulette SHALL NOT search for a repository root or read a home-root environment variable when staging, checking, or applying.
Rulette SHALL reject `local`, `enterprise`, `managed`, `system`, and every other non-v0.1 scope before mapping or filesystem access.
Rulette SHALL reject any mapping to a known managed or system location even when the caller can write to that location.

#### Scenario: A plan cannot expand publication authority

- **WHEN** a plan requests project publication under a root different from `--allow-project-root <root>`
- **THEN** Rulette SHALL reject apply
- **AND** SHALL NOT write any destination.

#### Scenario: An undocumented user mapping is unavailable

- **WHEN** a caller requests a user-scoped Cursor target
- **THEN** Rulette SHALL report the user mapping as unavailable
- **AND** SHALL NOT infer an output location from user configuration or environment state.

### Requirement: Plan integrity preflight and safe destination resolution

`transform --apply <plan> --expect-plan-sha256 <digest>` SHALL require the expected digest to match the exact bytes of the plan before parsing or trusting plan contents.
Apply and plan-mode check SHALL open the stage root and authorized roots through stable file handles and verify every referenced staged artifact is a regular file whose digest, length, and executable metadata match the plan.
Before a destination is read or written, Rulette SHALL validate the artifact's target, mapping version, artifact class, and normalized native artifact path against the compiled-in mapping.
The publication library, rather than a plan or backend-provided absolute path, SHALL be the only layer that resolves a typed native artifact into a filesystem destination.
Rulette SHALL reject duplicate destinations, ancestor-descendant destination conflicts, paths outside the authorized root, invalid path components, platform-normalization collisions, reserved platform names, repository-control namespaces, and `.git` paths.
Rulette SHALL reject a destination or parent component that is a symlink, junction, reparse point, directory where a regular file is required, or another non-regular file.
Destination traversal, temporary-file creation, replacement, and rollback SHALL be relative to pre-opened authorized root handles and SHALL never follow a link.
Unix implementations SHALL use component-by-component no-follow descriptor traversal and reject mount-boundary traversal.
Windows implementations SHALL use handle-relative traversal and reject reparse points and volume-boundary traversal.
Rulette SHALL report publication as unsupported on a platform where these invariants cannot be enforced.

#### Scenario: A substituted artifact prevents publication

- **WHEN** a staged artifact is modified, removed, substituted, or has different executable metadata after staging
- **THEN** apply SHALL fail before reading or changing any destination.

#### Scenario: An arbitrary plan path is rejected

- **WHEN** a plan contains a normalized-looking native artifact path that is not producible by its target mapping and artifact class
- **THEN** Rulette SHALL reject the plan before filesystem traversal.

### Requirement: Conflict-aware transactional publication

Apply SHALL preflight every destination before creating a directory or mutating a destination.
Rulette SHALL classify each verified destination as absent, unchanged, or conflicting by comparing exact bytes and target-representable executable metadata.
An unchanged destination SHALL not be rewritten.
An absent destination SHALL be eligible for creation.
A conflicting destination SHALL abort the whole apply unless `--replace` is explicit.
`--replace` SHALL be valid only with `--apply`.
After preflight succeeds, apply SHALL write each changed destination through a same-directory temporary regular file and atomic rename.
Before replacing a destination, apply SHALL record its exact original bytes, executable metadata, and file identity.
If a later mutation fails, apply SHALL delete files it created, remove empty directories it created, and restore replaced files only when their identity and digest still match the file written by that apply.
If rollback encounters a concurrent change or fails, Rulette SHALL report both the rollback failure and the original publication failure and SHALL NOT overwrite a third-party change.
The transaction guarantee applies to failures observed by the running process and does not guarantee atomicity across crashes, power loss, stale destination deletion, or non-cooperating concurrent writers.

#### Scenario: A differing destination requires replacement authority

- **WHEN** apply preflight finds an existing regular file with different bytes or executable metadata
- **AND** `--replace` is absent
- **THEN** Rulette SHALL fail the entire apply before changing any destination.

#### Scenario: A late publication failure rolls back owned changes

- **WHEN** apply creates or replaces one or more destinations and a later destination write fails
- **THEN** Rulette SHALL restore only the destinations it can prove it still owns
- **AND** SHALL leave no partial successful publication created by that apply when rollback succeeds.

### Requirement: Non-mutating publication checks

`transform --check` with source inputs SHALL compute the same native lowering and destination classification as staging without creating a stage directory or mutating a destination.
Source-mode check SHALL require the same explicit project and user roots as staging.
`transform --check` with an apply plan SHALL verify plan integrity, root authority, mapping validity, and destination drift without publishing.
Plan-mode check SHALL require the same explicit allow-root arguments as apply.
If a check finds an absent or conflicting destination, Rulette SHALL exit with a non-zero status after reporting the classification.
Check SHALL not create temporary files or directories.

#### Scenario: A plan-mode check reports drift without mutation

- **WHEN** a valid authorized plan has a destination that is absent or conflicting
- **THEN** `transform --check` SHALL exit non-zero
- **AND** SHALL not write a destination, temporary file, or directory.
