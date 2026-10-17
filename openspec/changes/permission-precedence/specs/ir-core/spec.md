## ADDED Requirements

### Requirement: Typed tool access rules and permission policies

The compilation graph SHALL represent portable tool permissions using `PackageKind::Permissions` and `SemanticItem::Permissions` containing a `TargetPermissionPolicy`.
Each `ToolAccessRule` SHALL require a non-empty `tool`, an optional `pattern`, a required `PermissionAction` (`allow`, `ask`, or `deny`), and an optional `PermissionScope` (`user`, `project`, `local`, or `enterprise`).
Validation SHALL fail closed by rejecting any missing, null, or unknown action or scope token.

#### Scenario: Parsing a valid permission policy with per-target overrides

- **WHEN** a `Permissions` package defines a `TargetPermissionPolicy` with a `default` policy and `overrides` for `cursor` and `antigravity`
- **THEN** Rulette SHALL construct a validated `PackageKind::Permissions` package whose `PackageId` digest covers the canonical permission policy
- **AND** target resolution SHALL return `overrides[target]` (or normalized family alias) with full replacement before falling back to `default`.

#### Scenario: Rejecting a tool access rule with missing or invalid action

- **WHEN** a source document or graph interchange payload contains a `ToolAccessRule` with an omitted, empty, or unknown `action` value
- **THEN** Rulette SHALL fail validation immediately with an error before constructing a `CompilationGraph`
- **AND** SHALL NOT default the rule action to `allow` or `ask`.

### Requirement: Deterministic permission conflict-resolution and precedence ordering

Rulette SHALL resolve conflicts among `ToolAccessRule` entries deterministically across four canonical tiers (`UniversalBaseline < SpecificAllow < SpecificAsk < HardDeny`).
`PermissionAction::Deny` SHALL dominate `Ask` and `Allow` across all scopes and overlapping patterns.
Within a tier, rules SHALL be ordered by `PermissionScope` (`enterprise > local > project > user`), pattern `SpecificityKey`, restrictive action (`deny > ask > allow`), and lexicographical provenance tie-breaking.

#### Scenario: Exact tool and pattern collision resolves fail-closed

- **WHEN** two `ToolAccessRule` entries in the same compilation unit or across merged `Permissions` packages normalize to the same `(canonical_tool, normalized_pattern)` with actions `allow` and `deny`
- **THEN** the precedence engine SHALL resolve the rule action to `deny` regardless of input order
- **AND** permuting the input rule or package order SHALL produce byte-identical resolved policies.

#### Scenario: Specificity ordering across exact, prefix, unanchored, and universal patterns

- **WHEN** a `PermissionPolicy` contains rules for `bash` with patterns `"*"` (`UniversalWildcard`), `"* --version"` (`UnanchoredGlob`), `"git *"` (`AnchoredPrefix`, length 4), `"git push *"` (`AnchoredPrefix`, length 9), and `"git status"` (`Exact`)
- **THEN** the precedence engine SHALL score their `SpecificityKey` values strictly by `(tool_is_exact, SpecificityClass, leading_literal_len, literal_segment_count, total_literal_len, Reverse(wildcard_count))`
- **AND** SHALL order them as `"git status" > "git push *" > "git *" > "* --version" > "*"`.

#### Scenario: Partial glob overlap between allow and deny resolves to deny

- **WHEN** a policy contains `allow("bash", "* --help")` and `deny("bash", "rm *")`
- **THEN** the precedence engine SHALL place `deny("bash", "rm *")` in `Tier 4 (HardDeny)` above `allow("bash", "* --help")` in `Tier 2 (SpecificAllow)`
- **AND** any invocation matching both patterns (such as `rm -rf / --help`) SHALL evaluate to `deny`.

### Requirement: Shadowed permissive rule detection

The permission precedence engine SHALL detect when a lower-tier permissive rule (`Allow` or baseline `Ask`) is structurally subsumed by a broader higher-tier restrictive rule (`Deny` or specific `Ask`) on the same tool.
When a rule is subsumed by a higher-tier rule, the engine SHALL prune the shadowed permissive rule from active emission and record a shadowed-rule diagnostic for capability reporting.

#### Scenario: Narrow allow rule shadowed by broad deny rule

- **WHEN** a policy defines `deny("bash", "git *")` and `allow("bash", "git status")`
- **THEN** the precedence engine SHALL retain `deny("bash", "git *")` as the effective rule
- **AND** SHALL prune `allow("bash", "git status")` from active emission while recording it as shadowed by `deny("bash", "git *")`.
