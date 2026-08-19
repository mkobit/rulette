## ADDED Requirements

### Requirement: Per-target activation overrides

The IR rule metadata SHALL support a typed `rulette:activation` model that accepts either a bare `Activation` configuration or a wrapped target-override container with `default` and `overrides` mappings.
When resolving activation settings for a specific target `T`, Rulette SHALL use `overrides[T]` if defined, and otherwise fall back to `default`.
Override resolution SHALL perform full replacement of the activation configuration for the given target, rather than deep merging fields.

#### Scenario: Resolving target-specific override

- **WHEN** a rule defines `rulette:activation` with a `default` mode and an override for target `cursor`
- **AND** an emitter transforms the rule for target `cursor`
- **THEN** Rulette SHALL resolve the activation settings defined in `overrides["cursor"]`.

#### Scenario: Fallback to default activation

- **WHEN** a rule defines `rulette:activation` with a `default` mode and overrides for other targets, but not `claude`
- **AND** an emitter transforms the rule for target `claude`
- **THEN** Rulette SHALL resolve the activation settings defined in `default`.

#### Scenario: Backwards compatibility with bare activation object

- **WHEN** a rule defines `rulette:activation` as a bare `Activation` object without `default` or `overrides` wrappers
- **THEN** Rulette SHALL parse the object as the `default` activation configuration with empty overrides.
