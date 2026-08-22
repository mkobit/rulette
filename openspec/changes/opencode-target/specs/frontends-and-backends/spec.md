## ADDED Requirements

### Requirement: OpenCode format auto-detection and parsing

Rulette SHALL automatically detect OpenCode configuration and agent files and parse them into the unified IR.
Rulette SHALL support parsing `opencode.json` / `opencode.jsonc` files containing MCP servers and agent definitions into `Entity::McpServer` and `Entity::Agent`.
Rulette SHALL support parsing `.opencode/agents/*.md` markdown agent files into `Entity::Agent`.

#### Scenario: Parsing OpenCode JSON configuration with MCP servers

- **WHEN** an `opencode.json` file containing an `mcp` server mapping is parsed
- **THEN** Rulette SHALL construct `Entity::McpServer` entities with matching command, args, and env configurations.

#### Scenario: Parsing OpenCode agent markdown file

- **WHEN** an OpenCode agent markdown file containing `description:`, `mode:`, and `model:` frontmatter is parsed
- **THEN** Rulette SHALL construct an `Entity::Agent` entity with the specified metadata and body prompt.

### Requirement: OpenCode target emission

Rulette SHALL emit Rule, Agent, Skill, and McpServer entities formatted for OpenCode workspaces.
For agents, Rulette SHALL emit standardized `agents/<name>.md` files with YAML frontmatter.
For skills, Rulette SHALL emit standardized `skills/<name>/SKILL.md` skill files.
For MCP servers, Rulette SHALL emit `opencode.json` with formatted `mcp` configurations.
For rules, Rulette SHALL emit markdown rule files.

#### Scenario: Emitting agent to OpenCode directory

- **WHEN** an `Entity::Agent` entity is emitted to the `opencode` format
- **THEN** Rulette SHALL emit an `agents/<name>.md` file with validated frontmatter and system prompt body.

#### Scenario: Emitting MCP server to OpenCode directory

- **WHEN** an `Entity::McpServer` entity is emitted to the `opencode` format
- **THEN** Rulette SHALL emit an `opencode.json` configuration file containing the MCP server definition.

#### Scenario: Emitting skill to OpenCode directory

- **WHEN** an `Entity::Skill` entity is emitted to the `opencode` format
- **THEN** Rulette SHALL emit a `skills/<name>/SKILL.md` file with validated skill frontmatter and body.
