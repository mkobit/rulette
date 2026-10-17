## Execution blueprint and dependency map

This task breakdown is structured for deterministic, test-driven execution by downstream implementation agents using the `.beads/formulas/permission-precedence-implementation.formula.toml` workflow formula:

- `Layer 1 (Task 1)` $\to$ `Layer 2 (Task 2)`
- `Layer 2 (Task 2)` $\to$ `Layer 3 (Task 3)` and `Layer 4 (Task 4)` (parallelizable across isolated module boundaries)
- `Layer 3 (Task 3)` + `Layer 4 (Task 4)` $\to$ `Layer 5 (Task 5)` $\to$ `Task 6 (Final Verification)`

### Strict module boundary rules for downstream agents

1. **Layer 1 (`src/ir/permissions.rs`, `src/ir/mod.rs`)**: Pure data types and precedence algorithms only. **Forbidden imports**: `crate::cli`, `crate::inputs`, `crate::parsers`, `crate::emitters`, `crate::publication`, `std::fs`, `std::env`, `std::process`, `clap`.
2. **Layer 2 (`src/ir/graph.rs`, `src/lib.rs`)**: Compilation graph integration and cross-package policy merging. Depends only on `src/ir/permissions.rs` and standard library / `serde` / `schemars` / `sha2`.
3. **Layer 3 (`src/parsers/{claude,opencode,cursor,antigravity,codex}.rs`)**: Native frontend decoding into `PackageKind::Permissions`. Must never import `crate::emitters` or `crate::publication`.
4. **Layer 4 (`src/emitters/lowering.rs`, `src/publication/{mapping,plan}.rs`)**: Pure target lowering, capability classification (`capabilities()`), and static destination path mapping. Must never read or write filesystem paths during lowering.
5. **Layer 5 (`src/cli/commands/inspect.rs`, `tests/`)**: Thin CLI presentation and integration tests. All precedence and capability logic must be called from library exports, never duplicated in `src/cli/`.

---

## 1. Layer 1: Pure permission domain and 4-tier precedence engine (`src/ir/permissions.rs`)

- [ ] 1.1 Create `src/ir/permissions.rs` and re-export its public types from `src/ir/mod.rs` and `src/lib.rs`:
  - `PermissionAction` (`Allow`, `Ask`, `Deny`) with `#[serde(rename_all = "lowercase")]`, `restrictiveness() -> u8` (`Allow = 1, Ask = 2, Deny = 3`), and `join_restrictive(self, other: Self) -> Self`.
  - `PermissionScope` (`User`, `Project` [default], `Local`, `Enterprise`) with `#[serde(rename_all = "lowercase")]` and `rank() -> u8` (`User = 1, Project = 2, Local = 3, Enterprise = 4`).
  - `ToolAccessRule` with `#[serde(deny_unknown_fields)]`: required `tool: String`, optional `pattern: Option<String>`, required `action: PermissionAction`, and optional `scope: Option<PermissionScope>`.
  - `PermissionPolicy` (`default_mode: Option<PermissionAction>`, `rules: Vec<ToolAccessRule>`) and `TargetPermissionPolicy` (`Bare(PermissionPolicy)` | `Wrapped(TargetPermissionOverrides)`), implementing `resolve(&self, target: &str) -> &PermissionPolicy` with exact target name $\to$ normalized family prefix $\to$ `default` precedence.
- [ ] 1.2 Implement canonical tool and pattern normalization in `src/ir/permissions.rs`:
  - `CanonicalTool` (`All`, `Bash`, `Read`, `Edit`, `Write`, `WebFetch`, `WebSearch`, `Glob`, `Grep`, `List`, `Agent`, `Skill`, `ExternalDirectory`, `Mcp`, `Custom(String)`).
  - Parse tool-embedded specifiers (`"Bash(git diff *)"` $\to$ `("bash", "git diff *")`, `"WebFetch(domain:example.com)"` $\to$ `("webfetch", "example.com")`, `"mcp__github__get_issue"` / `"github:get_issue"` / `"mcp(github/get_issue)"` $\to$ `("mcp", "github/get_issue")`, `"Shell(git:*)"` $\to$ `("bash", "git *")`, `"command(git status)"` $\to$ `("bash", "git status")`).
  - Reject empty tool names, empty parenthesized specifiers, and any `tool` or `pattern` containing `char::is_control`.
- [ ] 1.3 Implement `SpecificityClass` (`UniversalWildcard = 1`, `UnanchoredGlob = 2`, `AnchoredPrefix = 3`, `Exact = 4`), the 6-tuple `SpecificityKey`, and structural pattern helpers on `NormalizedPattern`:
  - `subsumes(&self, narrower: &Self) -> bool`: returns `true` when `self` covers all strings matched by `narrower` (`Universal` covers all; `AnchoredPrefix(p1)` with single trailing `*` covers `Exact(s)` and `AnchoredPrefix(p2)` when `s` or `p2` starts with `p1`'s literal prefix; equal patterns cover each other).
  - `overlaps(&self, other: &Self) -> bool`: returns `true` when `self.subsumes(other) || other.subsumes(self)` or when an `AnchoredPrefix` and `UnanchoredGlob` can match a common string (conservative intersection check: only disjoint literal prefixes or incompatible exact strings return `false`).
  - `matches_invocation(&self, candidate: &str) -> bool`: deterministic glob matcher (`*` and `?`) used by the reference evaluator in tests.
- [ ] 1.4 Implement `resolve_permission_policy(policy: &PermissionPolicy, source_identity: &str, source_package_id: &str) -> Result<ResolvedPermissionPolicy>`:
  - Deduplicate exact `(CanonicalTool, NormalizedPattern)` collisions using Fail-Closed `Deny` dominance (`Deny` at any scope wins), then `PermissionScope` rank (`Enterprise > Local > Project > User`), then `PermissionAction::join_restrictive` (`Deny > Ask > Allow`), tie-breaking provenance by `(source_identity, source_package_id, rule_index)`.
  - Assign each unique rule to its canonical tier:
    - `PrecedenceTier::UniversalBaseline` (Tier 1): `pattern == "*"` and `action in {Allow, Ask}`
    - `PrecedenceTier::SpecificAllow` (Tier 2): `pattern != "*"` and `action == Allow`
    - `PrecedenceTier::SpecificAsk` (Tier 3): `pattern != "*"` and `action == Ask`
    - `PrecedenceTier::HardDeny` (Tier 4): `action == Deny`
  - Detect shadowed rules: any rule $R_{\text{low}}$ in Tier 1–3 that is subsumed (`subsumes`) by a rule $R_{\text{high}}$ in Tier 3–4 with $\text{tier}(R_{\text{high}}) > \text{tier}(R_{\text{low}})$ is removed from `active_rules` and pushed to `shadowed_rules: Vec<ShadowedPermissionRule>`.
  - Sort `active_rules` in strict ascending order of `(tier, SpecificityKey, canonical_tool, normalized_pattern, source_identity, source_package_id)`.
  - Implement `ResolvedPermissionPolicy::evaluate(&self, tool: &str, arg: Option<&str>) -> PermissionAction` (evaluates highest matching tier `Tier 4 > Tier 3 > Tier 2 > Tier 1`, falling back to `default_mode.unwrap_or(PermissionAction::Ask)`).
- [ ] 1.5 Write TDD unit tests in `src/ir/permissions.rs`:
  - **Fixture 1 (Deserialization fail-closed)**: JSON `{"tool": "bash"}` (missing `action`) and `{"tool": "bash", "action": "permit"}` (unknown `action`) must return `Err`.
  - **Fixture 2 (SpecificityKey total order)**: Verify `"git status"` (`Exact`) $>$ `"git push --force *"` (`AnchoredPrefix`, 17) $>$ `"git push *"` (`AnchoredPrefix`, 9) $>$ `"git *"` (`AnchoredPrefix`, 4) $>$ `"* --version"` (`UnanchoredGlob`) $>$ `"*"` (`UniversalWildcard`).
  - **Fixture 3 (Exact collision fail-closed & permutation invariance)**: Permute `[allow("bash", "git push *"), ask("bash", "git push *"), deny("bash", "git push *")]` across all 6 orderings and assert `resolve_permission_policy` always yields a single `Deny` rule.
  - **Fixture 4 (Shadowing detection)**: Input `[deny("bash", "git *"), allow("bash", "git status")]` produces `active_rules = [deny("bash", "git *")]`, `shadowed_rules = [allow("bash", "git status") shadowed by deny("bash", "git *")]`, and `evaluate("bash", Some("git status")) == PermissionAction::Deny`.
  - **Fixture 5 (Partial glob overlap fail-closed)**: Input `[allow("bash", "* --help"), deny("bash", "rm *")]` places `deny("bash", "rm *")` in Tier 4 after `allow("bash", "* --help")` in Tier 2, and `evaluate("bash", Some("rm -rf / --help")) == PermissionAction::Deny`.
  - **Fixture 6 (Sequential vs. Set-based equivalence)**: Verify that iterating `active_rules` in order and taking the *last* matching rule (OpenCode semantics) returns the exact same `PermissionAction` as `evaluate()` (Claude/Codex `Deny > Ask > Allow` set semantics) across a 20-invocation test matrix.

**Acceptance criteria & validation command:**
`cargo test ir::permissions` passes with 0 warnings and verifies all 6 TDD fixtures.

---

## 2. Layer 2: Compilation graph & schema integration (`src/ir/graph.rs`, `src/cli/commands/schema.rs`)

- [ ] 2.1 Extend `src/ir/graph.rs` to support `Permissions` packages:
  - Add `PackageKind::Permissions` (`#[serde(rename = "permissions")]` via `kebab-case`) and map `PackageKind::Permissions.semantic_kind()` to `Some("permissions")`.
  - Update `SemanticIdentity::parse` to accept `"permissions"` in `matches!(kind, "rule" | "skill" | "permissions" | "unsupported")`.
  - Add `SemanticItem::Permissions { policy: TargetPermissionPolicy, #[serde(skip_serializing_if = "Option::is_none")] frontend_payload: Option<FrontendPayload> }`.
  - Validate `policy` in `SemanticItem::validate()` (ensuring duplicate normalized override keys are rejected and all `ToolAccessRule` items in `default` and `overrides` pass validation).
  - Update `Package::validate()` so `PackageKind::Permissions` packages require at least one opaque source resource (the raw config observation) and zero `PrimaryInstruction` resources, and update `package_id()` to hash the canonical JSON bytes of `policy` for `SemanticItem::Permissions`.
  - Add constructor `Package::permissions(semantic_identity, provenance, package_root, policy, resources, frontend_payload) -> Result<Self>`.
- [ ] 2.2 Implement cross-package permission merging `merge_permission_packages(packages: &[&Package], target: &str) -> Result<ResolvedPermissionPolicy>` in `src/ir/permissions.rs` or `src/ir/graph.rs`:
  - Resolve each package's `TargetPermissionPolicy` for `target`, combine `default_mode` via `PermissionAction::join_restrictive`, and merge all rules through the deterministic precedence engine so package order never affects the merged `ResolvedPermissionPolicy`.
- [ ] 2.3 Write TDD unit tests in `src/ir/graph.rs`:
  - **Fixture 1 (`PackageId` sensitivity to policy changes)**: Changing a single `ToolAccessRule` action from `Ask` to `Deny` in `SemanticItem::Permissions` changes the computed `PackageId`.
  - **Fixture 2 (Graph JSON and TOML round-trip)**: A `CompilationGraph` containing a `PackageKind::Permissions` package round-trips byte-identically through `to_canonical_json()` / `from_json()` and `to_canonical_toml()` / `from_toml()`.
  - **Fixture 3 (Cross-package merge commutativity)**: Merging two `PackageKind::Permissions` packages $(P_A, P_B)$ vs $(P_B, P_A)$ produces identical `ResolvedPermissionPolicy` values.

**Acceptance criteria & validation command:**
`cargo test ir::graph` passes and confirms schema generation in `src/cli/commands/schema.rs`.

---

## 3. Layer 3: Frontend permission parsers (`src/parsers/{claude,opencode,cursor,antigravity,codex}.rs`)

- [ ] 3.1 Update `src/parsers/claude.rs`:
  - In `settings_packages()`, replace the `unsupported_package(observation, "permissions", ...)` call with `permissions_package(observation, permissions)?` returning a `PackageKind::Permissions` package with `SemanticIdentity("permissions:.claude/settings.json")`.
  - Parse `permissions.allow` (`PermissionAction::Allow`), `permissions.ask` (`PermissionAction::Ask`), `permissions.deny` (`PermissionAction::Deny`), and `permissions.defaultMode` (`"default"` / `"manual"` / `"plan"` $\to$ `Some(PermissionAction::Ask)`; `"dontAsk"` $\to$ `Some(PermissionAction::Deny)`; unknown or non-string `defaultMode` values preserved in `frontend_payload` or rejected if malformed).
  - Fail closed with `Err` if `allow`, `ask`, or `deny` is not an array of non-empty strings.
- [ ] 3.2 Update `src/parsers/opencode.rs`:
  - In `compile_config()`, when `object.get("permission")` is present, parse it into a `PackageKind::Permissions` package (`SemanticIdentity("permissions:opencode.json")`):
    - String shorthand `"allow" | "ask" | "deny"` $\to$ `default_mode: Some(action)` + universal rule `("*", "*", action)`.
    - Object map `{ "<tool>": "allow"|"ask"|"deny" | { "<pattern>": "allow"|"ask"|"deny" } }` $\to$ `ToolAccessRule` entries.
    - Reject any unrecognized action value (e.g. `"permit"` or boolean `true`) with a hard parse error.
- [ ] 3.3 Update `src/parsers/cursor.rs`:
  - Recognize `.cursor/cli.json` (`permissions.allow` and `permissions.deny` token strings `Shell(...)`, `Read(...)`, `Write(...)`, `WebFetch(...)`) and `.cursor/permissions.json` (`terminalAllowlist` $\to$ `allow("bash", "<prefix>*")`, `mcpAllowlist` $\to$ `allow("mcp", "<server>/<tool>")`) as `PackageKind::Permissions` packages.
- [ ] 3.4 Update `src/parsers/antigravity.rs`:
  - Recognize `.agents/settings.json` / `.antigravity/settings.json` containing a `"permissions"` block (`allow`, `ask`, `deny` arrays of `command(...)`, `read_file(...)`, `write_file(...)`, `mcp(...)`, `read_url(...)`) as a `PackageKind::Permissions` package.
- [ ] 3.5 Update `src/parsers/codex.rs`:
  - Recognize `.codex/rules/*.rules` files containing Starlark `prefix_rule(pattern = [...], decision = "allow" | "prompt" | "forbidden")` declarations and parse them into `PackageKind::Permissions` packages (`"allow"` $\to$ `Allow`, `"prompt"` $\to$ `Ask`, `"forbidden"` $\to$ `Deny`).
- [ ] 3.6 Write TDD parser unit tests in each frontend module:
  - Test valid native permission parsing and assert the resulting `PackageKind::Permissions` rules.
  - Test malformed / unknown action strings in each format and assert hard parse failure (`Err`).

**Acceptance criteria & validation command:**
`cargo test parsers::` passes all new and updated frontend unit tests.

---

## 4. Layer 4: Fail-safe backend lowering, Asymmetric Approximation Law, and capability reporting (`src/emitters/lowering.rs`, `src/publication/{mapping,plan}.rs`)

- [ ] 4.1 Extend `NativeArtifactClass` and `CapabilityReasonCode` in `src/emitters/lowering.rs` and wire serialization in `src/publication/plan.rs`:
  - Add `NativeArtifactClass::PermissionConfig` (`"permission-config"`).
  - Add `CapabilityReasonCode` variants:
    - `PermissionTargetUnsupported` (`"permission-target-unsupported"`)
    - `PermissionScopeUnrepresentable` (`"permission-scope-unrepresentable"`)
    - `PermissionToolUnrepresentable` (`"permission-tool-unrepresentable"`)
    - `PermissionActionUnrepresentable` (`"permission-action-unrepresentable"`)
    - `PermissionAskPromotedToDeny` (`"permission-ask-promoted-to-deny"`)
    - `PermissionPatternUnrepresentable` (`"permission-pattern-unrepresentable"`)
    - `PermissionPrecedenceUnrepresentable` (`"permission-precedence-unrepresentable"`)
  - Expose `pub fn capabilities(graph: &CompilationGraph, target: NativeTarget) -> Result<Vec<CapabilityFinding>>` in `src/emitters/lowering.rs` and re-export it from `src/lib.rs`.
- [ ] 4.2 Register `NativeArtifactClass::PermissionConfig` in `src/publication/mapping.rs`:
  - `CLAUDE_PROJECT_RULES`: `Exact { class: PermissionConfig, native: "settings.json", mapped: ".claude/settings.json" }`; `CLAUDE_USER_RULES`: `mapped: "settings.json"`.
  - `OPENCODE_PROJECT_RULES` & `OPENCODE_USER_RULES`: `Exact { class: PermissionConfig, native: "opencode.json", mapped: "opencode.json" }`.
  - `CURSOR_PROJECT_RULES`: `Exact { class: PermissionConfig, native: "cli.json", mapped: ".cursor/cli.json" }`.
  - `ANTIGRAVITY_PROJECT_RULES`: `Exact { class: PermissionConfig, native: "settings.json", mapped: ".agents/settings.json" }`; `ANTIGRAVITY_USER_RULES`: `mapped: "settings.json"`.
  - `CODEX_PROJECT_RULES`: `Exact { class: PermissionConfig, native: "rules/rulette.rules", mapped: ".codex/rules/rulette.rules" }`; `CODEX_USER_RULES`: `mapped: "rules/rulette.rules"`.
- [ ] 4.3 Implement target-specific fail-closed permission lowering in `src/emitters/lowering.rs`:
  - **Claude (`settings.json`)**:
    - Emit `permissions: { defaultMode, allow, ask, deny }`.
    - Withhold `Allow` rules with mid-command wildcards (`Bash(* rm)`) or MCP argument patterns (`PermissionPatternUnrepresentable`).
    - Report shadowed rules via `PermissionPrecedenceUnrepresentable`.
  - **OpenCode (`opencode.json`)**:
    - Serialize `"permission"` preserving the **4-Tier Ascending Order** (`Tier 1: UniversalBaseline` $\to$ `Tier 2: SpecificAllow` $\to$ `Tier 3: SpecificAsk` $\to$ `Tier 4: HardDeny`) using insertion-ordered map serialization so `Deny` keys are always emitted last.
    - Merge `"edit"` and `"write"` into `"edit"` using `PermissionAction::join_restrictive`, reporting `PermissionPrecedenceUnrepresentable` if their actions differed on the same pattern.
  - **Cursor (`cli.json`)**:
    - Emit `{"version": 1, "permissions": {"allow": [...], "deny": [...]}}` for `bash` (`Shell(...)`), `read` (`Read(...)`), `edit`/`write` (`Write(...)`), and `webfetch` (`WebFetch(...)`).
    - For `Ask` rules: if the `Ask` pattern overlaps (`overlaps`) any representable `Allow` rule on the same tool, promote the `Ask` rule to `permissions.deny` and report `PermissionAskPromotedToDeny` (`Lossy`); otherwise omit and report `PermissionActionUnrepresentable` (`Lossy`).
    - For unrepresentable tools (e.g. `mcp`, `websearch`): report `PermissionToolUnrepresentable` and withhold any overlapping `Allow` if a `Deny` was unrepresentable.
  - **Antigravity (`settings.json`)**:
    - Emit `{"permissions": {"allow": [...], "ask": [...], "deny": [...]}}` using `command(...)`, `read_file(...)`, `write_file(...)`, `mcp(...)`, `read_url(...)`.
    - Reject any pattern containing `*` or `?` wildcards (`PermissionPatternUnrepresentable`): withhold wildcard `Allow` rules (never widen!) and withhold any `Allow` rule overlapped by an unrepresentable wildcard `Deny` rule (`PermissionPrecedenceUnrepresentable`).
  - **Codex (`rules/rulette.rules`)**:
    - Emit deterministic Starlark `prefix_rule(pattern = [...], decision = "allow" | "prompt" | "forbidden")` entries for `bash` rules with exact or trailing-`" *"` command prefixes.
    - Never emit permission prose into `AGENTS.md`.
    - Report non-`bash` tools with `PermissionToolUnrepresentable` and non-prefix shell globs with `PermissionPatternUnrepresentable` (withholding overlapping `Allow` rules if a `Deny` cannot be represented).
  - **AgentPlugin**:
    - Classify `PackageKind::Permissions` as `CapabilitySeverity::Dropped` with `CapabilityReasonCode::PermissionTargetUnsupported`.
- [ ] 4.4 Include permission loss reason codes in strict rejection error details in `lower()` (alongside `RuleDescriptionUnrepresentable` and `RuleActivationUnrepresentable`).
- [ ] 4.5 Write TDD lowering unit tests in `src/emitters/lowering.rs` and `src/publication/plan.rs`:
  - **Fixture 1 (OpenCode 4-Tier key ordering)**: Lower a policy with `ask("bash", "*")`, `allow("bash", "git *")`, `ask("bash", "git push *")`, and `deny("bash", "git push --force *")` to `NativeTarget::OpenCode` and verify the serialized JSON string has keys in that exact sequential order (`"*"` before `"git *"` before `"git push *"` before `"git push --force *"`).
  - **Fixture 2 (Cursor `Ask` carve-out promoted to `Deny`)**: Lower `[allow("bash", "git *"), ask("bash", "git push *")]` to `NativeTarget::Cursor`: assert `LoweringOptions::strict()` fails with `permission-action-unrepresentable` or `permission-ask-promoted-to-deny`, and `LoweringOptions::allow_lossy()` emits `"allow": ["Shell(git:*)"]` and `"deny": ["Shell(git:push*)"]` with a `PermissionAskPromotedToDeny` finding.
  - **Fixture 3 (Antigravity wildcard `Allow` withheld, never widened)**: Lower `[allow("bash", "git *"), allow("bash", "cargo test")]` to `NativeTarget::Antigravity` with `allow_lossy()`: assert `"command(cargo test)"` is emitted, `"command(git)"` is NOT emitted, and `PermissionPatternUnrepresentable` is reported.
  - **Fixture 4 (Antigravity unrepresentable `Deny` revokes overlapping `Allow`)**: Lower `[allow("bash", "git status"), deny("bash", "*status")]` to `NativeTarget::Antigravity` with `allow_lossy()`: assert `"command(git status)"` is withheld so the target cannot fail open.
  - **Fixture 5 (Codex never writes permissions to `AGENTS.md`)**: Lower a graph with a `Rule` package and a `Permissions` package to `NativeTarget::Codex`: assert `AGENTS.md` contains only the rule instruction text and `.codex/rules/rulette.rules` contains only the Starlark `prefix_rule` definitions.
  - **Fixture 6 (Staged plan round-trip)**: Verify `PublicationPlan` with each new permission `CapabilityReasonCode` serializes to canonical JSON and parses back via `parse_plan_with_expected_digest`.

**Acceptance criteria & validation command:**
`cargo test emitters::lowering && cargo test publication::` passes all unit tests.

---

## 5. Layer 5: CLI diagnostics, coverage matrix, and Journey 6 integration tests (`src/cli/commands/inspect.rs`, `tests/`)

- [ ] 5.1 Update `src/cli/commands/inspect.rs`:
  - Map `PackageKind::Permissions` to `"permissions"` in `package_kind_name()`.
  - Ensure `rulette inspect <input> --to <target>` and `rulette inspect <input> --coverage [--json] [--strict]` display `permissions` rows and all seven permission `CapabilityReasonCode` tokens.
- [ ] 5.2 Add end-to-end CLI integration tests in `tests/`:
  - **Journey 6 multi-target governance fixture**: Ingest a canonical governance source containing rules, skills, and permissions and run `rulette inspect --coverage --json` and `rulette transform` across `claude`, `opencode`, `cursor`, `antigravity`, and `codex`.
  - Verify strict multi-target atomic rejection when any target encounters an unaccepted permission loss without `--allow-lossy`.
  - Verify `--allow-lossy` stages all representable artifacts, records all permission loss findings in `rulette.plan.json`, and never fails open on any target.

**Acceptance criteria & validation command:**
`cargo test --test '*'` passes all CLI integration tests.

---

## 6. Final quality gate verification

- [ ] 6.1 Run `mise run spec-validate` to validate all OpenSpec specifications strictly.
- [ ] 6.2 Run `mise run check` (`spec-validate`, `markdownlint-cli2`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`).
- [ ] 6.3 Run `mise run bd-check` to verify Beads issue tracker health.
