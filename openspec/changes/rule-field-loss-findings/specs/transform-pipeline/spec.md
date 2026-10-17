## ADDED Requirements

### Requirement: Strict rule field loss reporting

Strict transform loss policy SHALL reject rule description or activation losses before any stage or destination writes. Rejection SHALL identify the stable rule-loss reason codes and omitted field names. `--allow-lossy` SHALL permit output while retaining both new reason codes and their findings through staged-plan serialization and parsing.

#### Scenario: Multi-target rule loss rejects atomically

- **WHEN** one requested target preserves a rule and another loses its description or activation without `--allow-lossy`
- **THEN** compilation SHALL identify those losses and SHALL NOT write any target's staging artifacts or destinations.

#### Scenario: Explicitly permitted rule loss roundtrip

- **WHEN** rule field loss is accepted with `--allow-lossy`
- **THEN** staged-plan serialization and parsing SHALL retain each aggregated finding, reason, package identity, and artifact association.
