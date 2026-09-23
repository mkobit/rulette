## Why

The Agent Plugins specification (agent-plugins.org v1.0.0) is an open, vendor-neutral standard for packaging Agent Skills (`skills/*/SKILL.md`) and Model Context Protocol servers (`mcp.json`) alongside a root manifest (`plugin.json`) and client extension directories.
Modern AI developer tools, including Cursor, Codex, Copilot, and Claude, are increasingly supporting or interoperating with this distribution format.
Currently, Rulette cannot ingest or emit Agent Plugins v1.0.0 packages, preventing teams from standardizing and distributing skill packages and MCP configs across harnesses using this open standard.
This change introduces bidirectional Agent Plugins v1.0.0 support in Rulette as both an input format (`InputFormat::AgentPlugin`) and output target (`OutputFormat::AgentPlugin`), tracked in `rulette-vvl`.

## What changes

- Add `agent-plugin` as a supported input format (`InputFormat::AgentPlugin`) and output format (`OutputFormat::AgentPlugin`).
- Implement the Agent Plugins frontend parser in `src/parsers/agent_plugin/` supporting `plugin.json` root manifests, `skills/*/SKILL.md` skill discovery, `mcp.json` MCP configurations, and reverse-domain client extension directories.
- Model `plugin.json` (v1.0.0) wire types separately from domain types, collecting unknown top-level fields into non-fatal `GraphDiagnostic` warnings while strictly enforcing schema versioning.
- Decompose an Agent Plugin package into isolated graph package units: each skill as `PackageKind::Skill`, the manifest as an unsupported manifest package, and each MCP server as an unsupported MCP package.
- Implement the Agent Plugins lowering target in `src/emitters/lowering.rs` supporting direct skill emission, manifest rendering, and same-domain MCP server re-aggregation.
- Enforce strict capability handling by default: attempting to lower `PackageKind::Rule` to `agent-plugin` fails under strict mode (`allow_lossy: false`) because the specification defines no rule format.
- Support opt-in rule-to-skill lowering under `allow_lossy: true`, converting rules to standardized skills (`skills/<name>/SKILL.md`) and emitting `CapabilityReasonCode::RuleLoweredAsSkill` loss findings.
- Introduce per-target `allow_lossy` configuration in transform outputs so multi-target pipelines can permit loss on `agent-plugin` without relaxing strictness for lossless targets.
- Integrate Agent Plugins into CLI commands (`transform`, `inspect`, format auto-detection, and publication path mapping).

## Scope

- Ingest compliant Agent Plugins v1.0.0 directory structures and tar/tar.gz archives.
- Validate root manifest `$schema` for `https://agent-plugins.org/schemas/1.0.0/plugin.schema.json`.
- Enforce package path containment, rejecting symlinks and parent directory traversal (`..`).
- Emit compliant Agent Plugins v1.0.0 directory layouts from the compilation graph.
- Record non-fatal warnings for non-conforming skills or invalid server entries without invalidating conforming siblings.
- Report deterministic capability loss findings before publication staging.

## Non-goals

- Rulette does not execute plugins, launch MCP subprocesses, or expand `${PLUGIN_ROOT}` / `${PLUGIN_DATA}` environment variables at compile time.
- Rulette does not provide dynamic shared-library plugin loading or arbitrary runtime script execution.
- Rulette does not define remote registry fetching or dependency resolution in this change.
- Rulette does not invent semantic rule translation across foreign harnesses; rule bodies are preserved as instructions or lowered to skills only when explicitly permitted.

## Capabilities

### Modified capabilities

- `frontends-and-backends`: Support auto-detecting, parsing, inspecting, and emitting Agent Plugins v1.0.0 packages, with strict rule rejection by default and opt-in rule-to-skill adaptation under `allow_lossy`.

## Impact

- `src/cli/formats.rs`: Add `InputFormat::AgentPlugin` and `OutputFormat::AgentPlugin`.
- `src/parsers/agent_plugin/`: Add parser module with decoupled wire deserialization, domain modeling, and graph compilation.
- `src/parsers/frontend.rs`: Register `NativeFrontend::AgentPlugin` and auto-detection rules based on `plugin.json`.
- `src/emitters/lowering.rs`: Register `NativeTarget::AgentPlugin`, new artifact classes, capability reason codes (`RuleLoweredAsSkill`, `SynthesizedManifest`), and rule/skill lowering rules.
- `src/publication/mapping.rs`: Add `AGENT_PLUGIN_PROJECT` publication candidate mappings for `plugin.json`, `mcp.json`, and `skills/`.
- `src/publication/model.rs` & `src/publication/stage.rs`: Support per-target `allow_lossy` configuration.
- `openspec/specs/frontends-and-backends/spec.md`: Add requirements for Agent Plugins ingestion and emission.
- Tracking bead: `rulette-vvl`.
