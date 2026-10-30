## ADDED Requirements

### Requirement: Typed skill metadata and execution policies

The compilation graph SHALL represent skill metadata and execution policies using typed `SkillMetadata` fields in `SemanticItem::Skill`.
`SkillMetadata` SHALL support `display_name`, `allowed_tools`, `paths`, `disable_model_invocation`, `user_invocable`, `context`, `agent`, `icon`, `color`, and `default_prompt`.
Validation SHALL reject path traversal in `paths` and control characters in `allowed_tools`.
`PackageId` computation SHALL hash the serialized JSON bytes of `SkillMetadata`.

#### Scenario: Parsing and hashing skill metadata

- **WHEN** a skill package contains non-empty `SkillMetadata`
- **THEN** Rulette SHALL construct a validated `SemanticItem::Skill`
- **AND** the package `PackageId` SHALL change when any `SkillMetadata` field is modified.

#### Scenario: Rejecting unsafe paths in skill metadata

- **WHEN** a skill declares `paths` containing `../` or leading `/`
- **THEN** validation SHALL fail immediately before constructing a `CompilationGraph`
- **AND** SHALL NOT emit any partial artifacts.
