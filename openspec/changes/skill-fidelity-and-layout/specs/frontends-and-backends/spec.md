## ADDED Requirements

### Requirement: Skill companion preservation

When lowering a skill package to a skill-directory target (Codex, Claude, OpenCode, Antigravity, AgentPlugin), Rulette SHALL preserve all relative companion resources as Supported `NativeArtifactClass::SkillResource` artifacts. When lowering to a target without skill directory support (such as Cursor project rules), companion resources SHALL be reported as Dropped (`opaque-resource-unrepresentable`).

#### Scenario: Cross-domain skill companion preservation

- **WHEN** a skill containing a companion script or file is lowered to a skill-directory target in a different domain
- **THEN** Rulette SHALL emit the companion resource at `skills/<name>/<path>`
- **AND** report the companion as Supported without generating loss findings.

#### Scenario: Flat rule target companion loss

- **WHEN** a skill is lowered to a flat rule target such as Cursor
- **THEN** Rulette SHALL emit `SkillLoweredAsRule` (`Lossy`)
- **AND** drop all companion resources with `opaque-resource-unrepresentable` (`Dropped`).

### Requirement: Skill executable permission fidelity

When emitting a `NativeArtifactClass::SkillResource` artifact, Rulette SHALL retain the `executable` boolean flag matching the observed resource. The publication engine SHALL verify and set `0755` permissions for executable resources without generating `executable-bit-unrepresentable` findings.

#### Scenario: Executable metadata preservation on skill resources

- **WHEN** a companion resource has executable mode `0755`
- **THEN** Rulette SHALL set `executable: true` on the emitted `NativeArtifact`
- **AND** verify and apply `0755` permissions upon publication without generating `executable-bit-unrepresentable` findings.

#### Scenario: Executable bit on primary markdown instruction

- **WHEN** a primary markdown instruction file has executable mode `0755`
- **THEN** Rulette SHALL set `executable: false` on the emitted artifact
- **AND** report `executable-bit-unrepresentable` (`Lossy`).

### Requirement: Shared skills layout interoperability

Rulette SHALL recognize `.agents/skills/<name>/SKILL.md` and direct `skills/<name>/SKILL.md` paths as a shared universal skill layout across all skill-capable frontends. Observations under `.agents/skills/` SHALL NOT be classified as exclusive to Antigravity and SHALL NOT trigger foreign frontend validation errors when decoded by other native frontends.

#### Scenario: Auto-detection with coexisting rules and shared skills

- **WHEN** input observations contain both `AGENTS.md` and `.agents/skills/<name>/SKILL.md`
- **THEN** Rulette format auto-detection SHALL resolve to `codex`
- **AND** decode both the root rule and the shared skills without multi-frontend ambiguity.

#### Scenario: Standalone shared skills snapshot

- **WHEN** input observations consist solely of `.agents/skills/` or `skills/`
- **THEN** Rulette format auto-detection SHALL resolve to a default skill-capable frontend
- **AND** decode all contained skills deterministically.
