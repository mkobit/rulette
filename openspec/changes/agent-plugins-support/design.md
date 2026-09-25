# Agent Plugins v1.0.0 support design

## Context

The Agent Plugins standard ([agent-plugins.org](https://agent-plugins.org) v1.0.0) defines a vendor-neutral package format for distributing AI capabilities.
An Agent Plugin consists of a required root manifest (`plugin.json`), optional MCP server configurations (`mcp.json`), a standardized skills hierarchy (`skills/*/SKILL.md`), and reverse-domain client extension directories (`<reverse-domain>/`).
In Rulette, the compilation architecture transitioned under the compilation graph kernel change to a typed, immutable `CompilationGraph` consisting of content-addressed `Package` units and `Resource` entries.
This design defines the ingestion frontend, compilation graph decomposition, lowering target backend, and publication mapping for bidirectional Agent Plugins v1.0.0 support.

## Goals / Non-goals

**Goals:**

- Implement bidirectional Agent Plugins v1.0.0 support via `InputFormat::AgentPlugin` and `OutputFormat::AgentPlugin`.
- Provide clean, strongly-typed wire and domain models for `plugin.json` and `mcp.json` matching the official v1.0.0 JSON schemas.
- Decompose incoming plugin directories into isolated graph packages: each skill as `PackageKind::Skill`, the manifest as an unsupported manifest package, and each MCP server as an unsupported MCP package.
- Enforce strict capability handling by default: fail compilation when attempting to lower rules to `agent-plugin` under `allow_lossy: false`.
- Provide opt-in rule-to-skill lowering under `allow_lossy: true`, converting rules into standardized `skills/<name>/SKILL.md` packages and reporting `RuleLoweredAsSkill`.
- Support per-target `allow_lossy` configuration in transform config outputs so multi-target pipelines can selectively allow loss on `agent-plugin` without weakening strictness on other targets.
- Enforce path containment rules, rejecting parent directory traversal (`..`) and symlinks.
- Map publication paths safely under `AGENT_PLUGIN_PROJECT` mappings in `src/publication/mapping.rs`.

**Non-goals:**

- Runtime execution, MCP process launching, or variable expansion (`${PLUGIN_ROOT}`, `${PLUGIN_DATA}`).
- Dynamic shared-library plugin loading or arbitrary runtime script execution.
- Remote network registries, lockfiles, or dependency resolution.
- Cross-tool rule instruction rewriting or AST adaptation; rule bodies are preserved directly or lowered to skills only upon explicit opt-in.

## Decisions

### 1. Three-tier domain and wire modeling

To prevent loose JSON dictionaries and maintain clean versioning, data types are structured into three distinct layers:

1. **Serialized-in wire types (`wire::de`)**:
   - `PluginManifestWireV1`: strictly deserializes required fields (`$schema`, `name`) and typed optional fields (`version`, `description`, `author`, `homepage`, `repository`, `license`, `keywords`, `extensions`). Uses `#[serde(flatten)] extra: BTreeMap<String, Value>` to capture unknown top-level fields, which are transformed into non-fatal `GraphDiagnostic` warnings.
   - `McpConfigWireV1`: deserializes `mcpServers` using an internally tagged enum for server transports (`Stdio`, `StreamableHttp`, `Sse`).
   - `SkillFrontmatterWire`: strictly parses YAML frontmatter in `SKILL.md` enforcing Agent Skills grammar and length constraints.
2. **Internal domain types (`model`)**:
   - `PluginManifest`: canonical metadata validated against semantic constraints.
   - `McpConfiguration`: validated map of unique server names to `McpServer`.
   - `McpServer`: typed enum of `Stdio { command, args, env, cwd }`, `StreamableHttp { url, headers }`, and `Sse { url, headers }`.
   - `PluginPath`: validates `./` prefix and safe relative path containment.
3. **Serialized-out wire types (`wire::ser`)**:
   - Structured serializers ensuring emitted manifests and configurations match `https://agent-plugins.org/schemas/1.0.0/*.json` with canonical formatting.

### 2. Graph package decomposition

In accordance with Rulette's compilation graph kernel, each package in the graph represents a single semantic unit:

- **Manifest**: `PackageKind::Unsupported` with `SemanticIdentity("unsupported:agent-plugin-manifest/plugin.json")`, `PackageRoot(".")`, and the manifest stored as an opaque resource and payload.
- **Skills**: each conforming directory `skills/<name>/` maps to `PackageKind::Skill` with `SemanticIdentity("skill:<name>")`, `PackageRoot("skills/<name>")`, and `SKILL.md` as `PrimaryInstruction`.
- **MCP servers**: each entry in `mcp.json` decomposes into an individual package with `SemanticIdentity("unsupported:agent-plugin-mcp-server/<name>")`, holding its isolated server configuration in `FrontendPayload`.
- **Client extensions**: top-level reverse-domain directories map to `PackageKind::Unsupported` with `SemanticIdentity("unsupported:agent-plugin-extension/<dir>")`.

### 3. Lowering to `NativeTarget::AgentPlugin`

The backend lowerer in `src/emitters/lowering.rs` processes the selected graph:

- **Skills**: each `PackageKind::Skill` emits `skills/<name>/SKILL.md` (`NativeArtifactClass::SkillInstruction`) and opaque resources (`NativeArtifactClass::SkillResource`).
- **Manifest**:
  - If exactly one manifest package is selected, its metadata is preserved and emitted as `plugin.json` (`NativeArtifactClass::PluginManifest`).
  - If multiple distinct manifest packages exist, lowering aborts with an aggregation collision error.
  - If zero manifest packages exist (e.g. converting from another harness or partial selection of skills), lowering under `allow_lossy: false` fails with a capability loss finding. Under `allow_lossy: true`, a minimal conforming `plugin.json` is synthesized and recorded with `CapabilityReasonCode::SynthesizedManifest`.
- **MCP servers**:
  - Same-domain packages (`agent-plugin` origin) are re-aggregated into an emitted `mcp.json` (`NativeArtifactClass::McpConfig`).
  - Foreign MCP packages are dropped with `CapabilityReasonCode::OpaqueCrossDomain`.
- **Rules**:
  - In strict mode (`allow_lossy: false`), rules trigger lowering failure (`CapabilitySeverity::Dropped`, `CapabilityReasonCode::UnsupportedSemantic`).
  - In lossy mode (`allow_lossy: true`), rules are lowered as pseudo-skills (`skills/<name>/SKILL.md`) with sanitized names and documented loss of activation triggers (`CapabilityReasonCode::RuleLoweredAsSkill`).

### 4. Per-target `allow_lossy` configuration

Lowering in the compiler kernel executes per-target: `lower(graph, target, LoweringOptions)`.
To allow multi-target fan-out pipelines to permit loss on `agent-plugin` without relaxing strictness on other targets:

- `OutputEntry` in `TransformConfigFile` supports `allow_lossy: Option<bool>`.
- Global CLI `--allow-lossy` acts as the invocation-wide fallback when per-target configuration is absent.

### 5. Publication and filesystem safety

- Add `NativeArtifactClass::PluginManifest`, `NativeArtifactClass::McpConfig`, and `NativeArtifactClass::ClientExtension`.
- In `src/publication/mapping.rs`, `AGENT_PLUGIN_PROJECT` registers exact mappings for `plugin.json` and `mcp.json`, prefix mappings for `skills/`, and validated reverse-domain prefix mappings for extensions.
- All input walks enforce strict symlink rejection and parent directory traversal checks (`..`).

## Adversarial review

**1. Package atomicity and partial selection.**

- *Vulnerability*: If `plugin.json` is an unsupported package, running `--select` on skills leaves the manifest unselected, producing a non-conforming plugin missing `plugin.json`.
- *Mitigation*: In strict mode, missing manifest fails compilation. Under `--allow-lossy`, the backend synthesizes a valid minimal manifest (`$schema` and package name) and reports a loss finding with `CapabilityReasonCode::SynthesizedManifest`.

**2. MCP server decomposition collisions.**

- *Vulnerability*: If multiple MCP server packages each carry the full `mcp.json` file as a resource, re-aggregating them causes artifact path collisions.
- *Mitigation*: Each MCP server package contains only its isolated JSON configuration object in its payload and resources. The lowerer re-aggregates these payloads into a single canonical `mcp.json` artifact.

**3. Rule name and description impedance.**

- *Vulnerability*: Rule logical names allow characters forbidden by the Agent Skills name grammar, and rules may lack descriptions (which skills require).
- *Mitigation*: Under `--allow-lossy`, rule names are sanitized to `^[a-z0-9]+(-[a-z0-9]+)*$`, colliding names are rejected before staging, and missing descriptions fall back to a default descriptive string while reporting `RuleLoweredAsSkill`.

**4. Opaque resource and extension leakage.**

- *Vulnerability*: Malicious or malformed client extensions could attempt to write into protected directories (like `.git` or parent paths).
- *Mitigation*: Client extension directories must strictly match the reverse-domain grammar (`^[a-z0-9_-]+(\.[a-z0-9_-]+)+/`), and publication mappings reject any hidden directory collisions.

**5. Wire schema conformance vs `deny_unknown_fields`.**

- *Vulnerability*: Agent Plugins requires ignoring unknown fields in `plugin.json` with warnings, whereas Rulette enforces strict schema validation.
- *Mitigation*: The wire deserializer captures unknown fields via `#[serde(flatten)] extra` and records non-fatal `GraphDiagnostic` warnings, while internal domain types and graph serialization maintain strict schema validation.

## Risks / Trade-offs

- **[Trade-off] Lossy rule adaptation**: Converting rules into skills drops activation triggers (globs, always-on flags) because Agent Skills have no activation model.
  → Mitigation: Keep strict rejection as the default. Require explicit opt-in via `allow_lossy: true` to perform this conversion.
- **[Risk] Path containment in `mcp.json`**: Relative executable commands (e.g. `./bin/server`) could be crafted to escape the package root.
  → Mitigation: Validate all relative paths in `mcp.json` as safe `ResourcePath` values at parse time.
