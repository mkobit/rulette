## ADDED Requirements

### Requirement: Antigravity format auto-detection and parsing

Rulette SHALL automatically detect Antigravity rule sources and parse frontmatter trigger modes, globs, and descriptions into the unified IR.
When `rulette:activation` per-target overrides are present, the parser SHALL resolve them appropriately.

#### Scenario: Parsing Antigravity rule with trigger and globs

- **WHEN** an Antigravity rule file containing `trigger: glob` and a list of `globs` is parsed
- **THEN** Rulette SHALL construct a Rule entity with `ActivationMode::Glob` and the specified glob patterns.

#### Scenario: Parsing Antigravity rule with model decision trigger

- **WHEN** an Antigravity rule file containing `trigger: model_decision` and a `description` is parsed
- **THEN** Rulette SHALL construct a Rule entity with `ActivationMode::Model` and the specified description.

### Requirement: Antigravity target emission

Rulette SHALL emit Rule and Skill entities formatted for Antigravity workspaces.
For rules, Rulette SHALL resolve the `antigravity` activation target and emit the corresponding `trigger`, `globs`, and `description` frontmatter.
For skills, Rulette SHALL emit standardized `skills/<name>/SKILL.md` skill files.

#### Scenario: Emitting rule with resolved always-on trigger

- **WHEN** a Rule entity with `ActivationMode::Always` is emitted to the `antigravity` format
- **THEN** Rulette SHALL emit frontmatter with `trigger: always_on`.

#### Scenario: Emitting rule with resolved glob trigger

- **WHEN** a Rule entity with `ActivationMode::Glob` and glob patterns is emitted to the `antigravity` format
- **THEN** Rulette SHALL emit frontmatter with `trigger: glob` and the formatted `globs` list.

#### Scenario: Emitting skill to Antigravity directory

- **WHEN** a Skill entity is emitted to the `antigravity` format
- **THEN** Rulette SHALL emit a `skills/<name>/SKILL.md` file with validated skill frontmatter and body.
