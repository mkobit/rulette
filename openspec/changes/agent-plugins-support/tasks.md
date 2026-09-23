## 1. Domain types, wire schemas, and format definitions

- [ ] 1.1 Add `InputFormat::AgentPlugin` and `OutputFormat::AgentPlugin` to `src/cli/formats.rs`.
- [ ] 1.2 Implement Agent Plugins v1.0.0 wire types and domain models in `src/parsers/agent_plugin/` (`PluginManifestWireV1`, `McpConfigWireV1`, `PluginManifest`, `McpConfiguration`).
- [ ] 1.3 Add `NativeFrontend::AgentPlugin` and `NativeTarget::AgentPlugin` enums.

## 2. Ingestion frontend and package decomposition

- [ ] 2.1 Implement `compile_native` in `src/parsers/agent_plugin/` to decompose plugin directories into skill packages, unsupported manifest packages, and per-server MCP packages.
- [ ] 2.2 Implement auto-detection and format candidate matching for `plugin.json` in `src/parsers/frontend.rs`.
- [ ] 2.3 Implement non-fatal diagnostic warning emission for unknown manifest fields and invalid server entries.
- [ ] 2.4 Enforce path containment rules for relative paths in `mcp.json` and reject symlinks.

## 3. Lowering backend target and rule handling

- [ ] 3.1 Add `NativeArtifactClass::PluginManifest`, `NativeArtifactClass::McpConfig`, and `NativeArtifactClass::ClientExtension` to `src/emitters/lowering.rs`.
- [ ] 3.2 Add `CapabilityReasonCode::RuleLoweredAsSkill` and `CapabilityReasonCode::SynthesizedManifest`.
- [ ] 3.3 Implement `lower_agent_plugin` in `src/emitters/lowering.rs` supporting direct skill emission, manifest preservation/synthesis, and same-domain MCP re-aggregation.
- [ ] 3.4 Enforce strict rule rejection under `allow_lossy: false` and opt-in rule-to-skill lowering under `allow_lossy: true`.

## 4. Publication mapping and per-target configuration

- [ ] 4.1 Register `AGENT_PLUGIN_PROJECT` candidate mappings in `src/publication/mapping.rs`.
- [ ] 4.2 Add `allow_lossy: Option<bool>` to `OutputEntry` in transform configuration models.
- [ ] 4.3 Support per-target `allow_lossy` resolution during lowering and publication staging.

## 5. CLI integration and diagnostics

- [ ] 5.1 Register Agent Plugins in `src/cli/commands/transform.rs` target dispatch and scaffold conventions.
- [ ] 5.2 Register Agent Plugins in `src/cli/commands/inspect.rs` coverage and loss diagnostics.

## 6. Testing and verification

- [ ] 6.1 Add unit tests for manifest wire parsing, unknown field warnings, and schema validation.
- [ ] 6.2 Add unit tests for MCP server decomposition and same-domain re-aggregation.
- [ ] 6.3 Add integration tests verifying strict rule rejection and opt-in rule-to-skill conversion under `--allow-lossy`.
- [ ] 6.4 Add round-trip tests for Agent Plugins v1.0.0 packages.
- [ ] 6.5 Run `cargo test`, `cargo clippy`, and `mise run spec-validate`.
