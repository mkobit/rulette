## ADDED Requirements

### Requirement: Agent Plugins format auto-detection and parsing

Rulette SHALL automatically detect Agent Plugins v1.0.0 packages and parse plugin manifests, skills, MCP configurations, and client extensions into the compilation graph.
Rulette SHALL validate that the root manifest `$schema` conforms to the v1.0.0 specification URI.
Unknown top-level fields in `plugin.json` SHALL be recorded as non-fatal warning diagnostics without failing package compilation.
Non-conforming skill directories SHALL be skipped with a warning diagnostic without invalidating conforming sibling skills.

#### Scenario: Parsing valid Agent Plugin package

- **WHEN** an Agent Plugin directory containing `plugin.json`, `mcp.json`, and `skills/` is parsed
- **THEN** Rulette SHALL construct a `CompilationGraph` containing `PackageKind::Skill` packages for conforming skills
- **AND** unsupported packages for the root manifest, client extensions, and MCP server configurations.

#### Scenario: Parsing Agent Plugin with unknown manifest fields

- **WHEN** a `plugin.json` file contains unrecognized top-level fields
- **THEN** Rulette SHALL parse the manifest successfully
- **AND** emit a non-fatal warning diagnostic for each unknown field.

### Requirement: Agent Plugins target emission

Rulette SHALL emit compliant Agent Plugins v1.0.0 package structures formatted for plugin distribution.
For skills, Rulette SHALL emit standardized `skills/<name>/SKILL.md` skill files and associated opaque resources.
For MCP servers, Rulette SHALL emit an aggregated `mcp.json` configuration file when lowering same-domain MCP packages.
For the plugin manifest, Rulette SHALL emit `plugin.json` preserving original metadata or synthesizing a conforming manifest under `--allow-lossy`.

#### Scenario: Emitting skills to Agent Plugin directory

- **WHEN** `PackageKind::Skill` packages are emitted to the `agent-plugin` format
- **THEN** Rulette SHALL emit `skills/<name>/SKILL.md` files with validated frontmatter and instructions.

#### Scenario: Rejecting rules under strict mode

- **WHEN** a graph containing `PackageKind::Rule` packages is emitted to `agent-plugin` with `allow_lossy: false`
- **THEN** Rulette SHALL fail lowering with a capability loss error.

#### Scenario: Lowering rules as skills under allow-lossy

- **WHEN** a graph containing `PackageKind::Rule` packages is emitted to `agent-plugin` with `allow_lossy: true`
- **THEN** Rulette SHALL convert rules into standardized skills under `skills/<name>/SKILL.md`
- **AND** report a loss finding with reason code `RuleLoweredAsSkill`.
