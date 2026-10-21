## ADDED Requirements

### Requirement: Deterministic rule bundling for single-file instruction targets

When lowering multiple `PackageKind::Rule` packages to `codex` (`AGENTS.md`) or `claude` (`CLAUDE.md`), `lower` in `src/emitters/lowering.rs` SHALL bundle all selected rules into one `NativeArtifactClass::Instruction` artifact instead of failing with `artifact-collision`.
Multi-file targets (`cursor`, `opencode`, and `antigravity`) SHALL emit one `NativeArtifactClass::Rule` file per rule package.
`rulette` SHALL evaluate capability loss findings per bundled rule package.

#### Scenario: Bundling multiple rules into `codex` `AGENTS.md` and `claude` `CLAUDE.md`

- **WHEN** a `CompilationGraph` containing two or more `PackageKind::Rule` packages is lowered to `codex@project`, `codex@user`, `claude@project`, or `claude@user`
- **THEN** `rulette` SHALL emit one deterministic `AGENTS.md` or `CLAUDE.md` artifact containing all selected rules ordered by `(SemanticIdentity, PackageId)`
- **AND** SHALL NOT fail with an `artifact-collision` error.

#### Scenario: Preserving one-file-per-rule emission on multi-file targets

- **WHEN** a `CompilationGraph` containing multiple `PackageKind::Rule` packages is lowered to `cursor@project` (`.cursor/rules/<name>.mdc`), `opencode@project` (`.opencode/rules/<name>.md`), or `antigravity@project` (`.agents/rules/<name>.md`)
- **THEN** `rulette` SHALL emit each `PackageKind::Rule` package as a separate rule file under the target rules directory
- **AND** SHALL NOT bundle the rules into a single instruction file.

#### Scenario: Reporting per-package capability loss in bundled instruction artifacts

- **WHEN** a bundled `PackageKind::Rule` package contains a non-empty `description` or conditional `activation` metadata and `allow_lossy` is `false`
- **THEN** `rulette` SHALL record `rule-description-unrepresentable` or `rule-activation-unrepresentable` for that `PackageId` and abort lowering
- **AND** when `allow_lossy` is `true`, `rulette` SHALL emit the bundled `AGENTS.md` or `CLAUDE.md` artifact and retain each per-package `CapabilityFinding`.

### Requirement: Rule frontmatter conversion and clean delimiter format

When bundling multiple `PackageKind::Rule` packages into `AGENTS.md` or `CLAUDE.md`, `rulette` SHALL strip raw YAML `---` frontmatter blocks and render each rule section with a deterministic Markdown header or HTML comment (`<!-- rulette:rule <logical-name> -->`).
`rulette` SHALL join adjacent bundled rule sections with a `\n\n---\n\n` delimiter and end the file with a single `\n`.
A single `PackageKind::Rule` package SHALL emit without inter-rule delimiters by default.

#### Scenario: Converting rule frontmatter to section headers and clean delimiters

- **WHEN** multiple `PackageKind::Rule` packages from `.cursor/rules/*.mdc`, `.claude/rules/*.md`, `.opencode/rules/*.md`, or `.agents/rules/*.md` are bundled into `AGENTS.md` or `CLAUDE.md`
- **THEN** `rulette` SHALL convert each rule preamble into a deterministic Markdown header or HTML provenance comment instead of emitting raw YAML frontmatter blocks
- **AND** SHALL join consecutive rule sections with `\n\n---\n\n` and end the file with a single `\n`.

#### Scenario: Single rule emission preserves unbundled content

- **WHEN** a `CompilationGraph` with exactly one `PackageKind::Rule` package is lowered to `AGENTS.md` or `CLAUDE.md` under default bundling options
- **THEN** `rulette` SHALL emit the primary instruction text without adding inter-rule `\n\n---\n\n` delimiters
- **AND** SHALL preserve byte-identical output for single-rule inputs.

### Requirement: Local marketplace catalog parsing and emission with path containment

The `agent-plugin` and `claude` adapters SHALL parse and emit `.claude-plugin/marketplace.json`, `.claude-plugin/plugin.json`, and `plugin.json` offline without network access.
`rulette` SHALL validate every plugin source path in `.claude-plugin/marketplace.json` as a normalized relative `ResourcePath` and SHALL reject `..`, leading `/` or `\`, drive prefixes, or symlinks.
Project mappings SHALL permit `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json`.

#### Scenario: Parsing and emitting a local marketplace catalog while preserving canonical plugin names

- **WHEN** an input snapshot contains `.claude-plugin/marketplace.json` mapping canonical `<plugin>` names to relative local plugin directories
- **THEN** `rulette` SHALL parse the catalog and referenced plugin packages into the `CompilationGraph`
- **AND** SHALL emit `.claude-plugin/marketplace.json` with canonical `<plugin>` names intact so `/<plugin>:<skill>` invocations stay valid.

#### Scenario: Rejecting parent directory traversal in a marketplace catalog

- **WHEN** an entry in `.claude-plugin/marketplace.json` contains `..` or an absolute path
- **THEN** `rulette` SHALL fail parsing immediately with a path-containment error
- **AND** SHALL NOT construct a `CompilationGraph` or stage any artifacts.

#### Scenario: Rejecting symbolic links in marketplace plugin paths

- **WHEN** a plugin directory referenced by `.claude-plugin/marketplace.json` contains a symbolic link in the filesystem or archive input
- **THEN** `rulette` SHALL reject the input during observation discovery before frontend decoding
- **AND** SHALL NOT read through or follow the symbolic link.
