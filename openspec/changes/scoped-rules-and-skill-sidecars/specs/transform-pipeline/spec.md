## ADDED Requirements

### Requirement: Scoped rule and sidecar publication mapping

The publication mapping engine SHALL authorize `.claude/rules/*.md`, subdirectory `*/AGENTS.md`, and skill sidecar `skills/*/agents/openai.yaml` artifact paths under authorized project roots.

#### Scenario: Mapping valid Claude scoped rule publication path

- **WHEN** a `NativeArtifactClass::Rule` artifact at `rules/test.md` is published to `claude@project`
- **THEN** target mapping SHALL resolve the path to `.claude/rules/test.md`.

#### Scenario: Mapping valid Codex subdirectory instruction publication path

- **WHEN** a `NativeArtifactClass::Instruction` artifact at `src/api/AGENTS.md` is published to `codex@project`
- **THEN** target mapping SHALL resolve the path to `src/api/AGENTS.md`.

### Requirement: Fail-closed verification for unrepresentable skill tools

Strict transform compilation (`allow_lossy: false`) SHALL reject any skill lowering where security tool restrictions (`allowed_tools`) are dropped.
The transformation SHALL proceed only when `--allow-lossy` is specified.

#### Scenario: Strict transform rejects dropped skill tool restrictions

- **WHEN** a skill declaring `allowed_tools` is transformed to `antigravity` without `--allow-lossy`
- **THEN** compilation SHALL fail immediately with a non-zero exit code
- **AND** Rulette SHALL NOT write any target files to disk.
