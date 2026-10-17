## ADDED Requirements

### Requirement: Rule field capability findings

Deterministic backend conformance and core-target capability inspection SHALL classify omitted rule descriptions and activation for Codex, Claude, OpenCode, Cursor, and Antigravity as Lossy, using `rule-description-unrepresentable` and `rule-activation-unrepresentable`. Findings SHALL retain package provenance and the actual artifact path. Each package and target SHALL have at most one finding per reason, aggregating lost activation fields in stable mode, globs, pattern, description order.

#### Scenario: Plain instructions omit metadata

- **WHEN** a rule with a nonempty description or conditional activation is lowered to Codex, Claude, or OpenCode
- **THEN** the backend SHALL retain its body and report the omitted fields without changing rendering.
- **AND** absent metadata and activation consisting only of Always SHALL NOT produce loss findings.

#### Scenario: Actual resolved native activation

- **WHEN** Cursor or Antigravity renders rule activation
- **THEN** capability findings SHALL use the same override resolution keys and mode precedence as the renderer.
- **AND** findings SHALL report distinct discarded modes, omitted globs, and omitted pattern text, but not duplicate selected modes or unused overrides.
- **AND** Cursor Manual represented by alwaysApply false without globs SHALL be supported.

#### Scenario: Antigravity model description

- **WHEN** Antigravity selects model_decision
- **THEN** activation description matching the emitted rule description after cosmetic whitespace normalization SHALL be supported.
- **AND** an independent differing activation description SHALL be reported as lost.

#### Scenario: AgentPlugin unchanged

- **WHEN** AgentPlugin lowers a rule
- **THEN** its existing native activation loss reporting SHALL remain unchanged by these new findings.
