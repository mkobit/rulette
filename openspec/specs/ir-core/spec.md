# ir-core Specification

## Purpose

Define the typed Intermediate Representation (IR) data model that captures AI tool configuration across rules, skills, hooks, MCP servers, sub-agents, and permissions.

## Requirements

### Requirement: Unified typed Intermediate Representation envelope

The IR SHALL represent all input source configurations in a single unified JSON document structure (`RuletteDocument`).
Each entity within the document SHALL be assigned one of the supported entity kinds: `rule`, `skill`, `hook`, `mcp-server`, `agent`, or `permissions`.
Format-specific metadata SHALL be preserved in an extensible `extra` map using `rulette:` well-known extension keys.

#### Scenario: Rule entity representation

- **WHEN** a source rule file is parsed into the IR
- **THEN** it SHALL produce an entity with `kind: "rule"` and a normalized metadata envelope
- **AND** any format-specific fields SHALL be placed in `extra`.

#### Scenario: MCP server entity normalization

- **WHEN** an MCP server configuration is parsed from any tool source
- **THEN** it SHALL produce an entity with `kind: "mcp-server"`
- **AND** the server command, arguments, and environment variables SHALL be stored in a standardized configuration payload.

### Requirement: Strict identity uniqueness

The IR parser SHALL enforce unique identity keys across all entities loaded in a single transformation context.
If two entities share identical names/identifiers across input files, the system SHALL fail execution immediately with a collision error.

#### Scenario: Identity collision failure

- **WHEN** multiple input files contain entities with identical names
- **THEN** Rulette SHALL abort transformation without producing partial output
- **AND** it SHALL surface the colliding entity identifier to the user.
