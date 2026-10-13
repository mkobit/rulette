## ADDED Requirements

### Requirement: Typed native artifact lowering

Every backend SHALL lower native targets through the following typed artifact contract, mapping validation, and determinism guarantees.

#### Scenario: Native lowering returns only typed artifact data

- **WHEN** any backend lowers native targets requested by a transform invocation
- **THEN** For every native target requested by `transform --target <format>@<scope>`, the backend SHALL lower selected graph packages into typed `NativeArtifact` values before any stage or destination write.
- **AND** A `NativeArtifact` SHALL contain only the target identifier, mapping-version identifier, artifact class, normalized native artifact path, bytes, executable-bit metadata, and structured capability findings.
- **AND** A backend SHALL NOT return an absolute destination path, a root path, an environment-expanded path, a write permission, or a generic unchecked filesystem path.

#### Scenario: Typed artifacts are validated and deterministically generated

- **WHEN** any backend generates an artifact or the publication library derives its destination
- **THEN** The publication library SHALL validate each artifact against the backend's compiled-in mapping version before it derives a destination.
- **AND** The backend SHALL produce deterministic artifact bytes and paths for the same graph, target, mapping version, and scope inputs.

#### Scenario: A backend cannot select an arbitrary destination

- **WHEN** a backend lowers a selected package for a project or user target
- **THEN** it SHALL return a typed native artifact path within its declared mapping grammar
- **AND** SHALL NOT return an absolute filesystem destination.

#### Scenario: Lowering is deterministic

- **WHEN** the same graph and target mapping are lowered twice with identical selection and scope inputs
- **THEN** the backend SHALL produce native artifacts with identical paths, bytes, executable metadata, and capability findings.

### Requirement: Verified target scope mapping registry

Target backends SHALL expose only the following versioned and verified native artifact and logical-scope mappings.

#### Scenario: Registry mappings are versioned verified and restricted

- **WHEN** any Rulette invocation runs with a target backend
- **THEN** Each target backend SHALL expose a versioned registry of supported native artifact classes and logical-scope mappings.
- **AND** Every core target backend SHALL expose a documented project mapping.
- **AND** A user mapping SHALL be exposed only when its root and layout are verified against primary vendor documentation and covered by conformance fixtures.
- **AND** Backends SHALL expose no v0.1 mapping for local-project, enterprise, managed, system, or arbitrary absolute scopes.
- **AND** The publication layer SHALL reject a serialized mapping version that is not supported by the running backend.

#### Scenario: Project mapping exists for a core target

- **WHEN** a caller requests `--target <core-format>@project`
- **THEN** the backend SHALL provide a versioned project mapping suitable for typed artifact validation.

#### Scenario: Unsupported user mapping is not guessed

- **WHEN** a caller requests a user mapping that is not in the backend registry
- **THEN** Rulette SHALL report the mapping as unavailable
- **AND** SHALL NOT derive a destination from the current user's environment or home directory.

### Requirement: Authoritative loss findings for native lowering

Native lowering and coverage inspection SHALL use the following structured capability findings and explicit accepted-loss rules.

#### Scenario: Native lowering reports structured package and artifact findings

- **WHEN** any backend lowers a selected graph package
- **THEN** Backends SHALL report structured capability findings for every selected graph package during native lowering.
- **AND** Each finding SHALL include a stable finding identifier, package identity, target, severity, reason code, and human-readable reason.
- **AND** When a finding corresponds to a native artifact, it SHALL also include that artifact entry identifier.

#### Scenario: Staging and coverage inspection enforce authoritative loss findings

- **WHEN** any staged compilation or coverage inspection evaluates target capability findings
- **THEN** Staged compilation SHALL fail on `lossy` or `dropped` findings unless the caller explicitly supplies `--allow-lossy`.
- **AND** When `--allow-lossy` permits staging, the resulting plan SHALL record each accepted finding.
- **AND** `inspect --coverage` SHALL report the same backend capability classifications without creating a stage or publishing a destination.

#### Scenario: Loss blocks staging by default

- **WHEN** target lowering produces a `lossy` or `dropped` finding
- **AND** `--allow-lossy` is absent
- **THEN** Rulette SHALL fail staging before creating the requested stage directory.

#### Scenario: Accepted loss is reviewable

- **WHEN** target lowering produces a `lossy` or `dropped` finding
- **AND** the caller explicitly supplies `--allow-lossy`
- **THEN** Rulette SHALL record the structured finding in `rulette.plan.json`
- **AND** SHALL not treat the plan as implicit authorization to apply it.

## MODIFIED Requirements

### Requirement: Format auto-detection and explicit format overrides

Rulette SHALL automatically infer source formats based on file paths, file extensions, and content structure.
Users SHALL be able to override source auto-detection with `--from`.
Source-compilation transforms SHALL select native destinations with repeatable `--target <format>@<scope>` rather than direct `--to` and output-path flags.
The `inspect --to <format>` capability-analysis interface SHALL remain available and SHALL not grant publication authority.

#### Scenario: Auto-detecting Cursor MDC files

- **WHEN** a `.cursor/rules/*.mdc` file is passed as input without `--from`
- **THEN** Rulette SHALL detect the format as Cursor MDC
- **AND** SHALL parse frontmatter metadata and body rules accordingly.

#### Scenario: Native target selection has no destination path

- **WHEN** a caller passes `--target opencode@project`
- **THEN** Rulette SHALL select the OpenCode project mapping
- **AND** SHALL require explicit staging or apply behavior before any native file is written.

### Requirement: Lossy conversion analysis via inspect verb

The `inspect` command SHALL print human-readable graph details and analyze target capability loss without staging or publishing files.
When running `inspect --to <format>` or `inspect --coverage`, Rulette SHALL report structured backend findings for graph packages and resources that cannot be represented in the destination format.
The analysis SHALL distinguish supported, lossy, dropped, unavailable-mapping, and unsupported outcomes where applicable.
The same finding identifiers and reason codes SHALL be used by staged compilation and recorded in accepted-loss plans.

#### Scenario: Inspecting capability loss

- **WHEN** `rulette inspect <input> --to <target>` is executed
- **THEN** Rulette SHALL list surviving graph content and structured reasons for unmapped, lossy, or dropped content
- **AND** SHALL not create a stage or write a live destination.
