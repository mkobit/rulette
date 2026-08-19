## Context

Rule files often require target-specific activation behaviors across different tools.
For example, Cursor MDC rules support `alwaysApply: true` or file glob patterns, while tools like Antigravity support trigger modes (`always_on`, `glob`, `manual`, `model_decision`) with natural language descriptions.
Previously, `rulette:activation` only supported a flat `Activation` struct, forcing target-specific nuances into untyped `extra` passthrough maps or dropping them during cross-format conversion.
`docs/2026-08-18-cli-ux-design.md` §3 defines a typed per-target override wrapper for `rulette:activation` using `{ default: T, overrides: map<target, T> }`.
This design specifies the data structures, serde deserialization, target resolution semantics, schema generation, and emitter integration for per-target overrides in Rulette.

## Goals / Non-goals

**Goals:**

- Provide a typed, generic `TargetOverrides<T>` wrapper that supports both bare `T` values and wrapped `{ default: T, overrides: map<target, T> }` representations.
- Integrate `TargetOverrides<Activation>` into `RuleMetadata` for `rulette:activation`.
- Support seamless deserialization from existing bare `Activation` YAML/JSON frontmatter and documents without breaking backwards compatibility.
- Implement unambiguous target resolution with full replacement semantics (overrides completely replace defaults for a given target, with no deep merging).
- Support target resolution with exact output format names and tool family aliases (e.g., `cursor-mdc` exact match taking precedence over `cursor` alias, then falling back to `default`).
- Update `rulette schema rulette:activation` to document the new wrapped and flat schema forms.

**Non-goals:**

- Deep merging of individual fields between `default` and target `overrides` (full replacement is an explicit design decision for predictability).
- Applying `TargetOverrides` to other extension keys like `rulette:tool-access` or `rulette:hook-event` in this change (scoped strictly to `rulette:activation` per YAGNI; wrapper is generic so other keys can adopt it later without breaking changes).
- Altering the non-activation contents of `extra` passthrough maps.

## Decisions

**1. Parametric `TargetOverrides<T>` container with untagged deserialization.**

```rust
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TargetOverrides<T> {
    Wrapped {
        default: T,
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        overrides: std::collections::BTreeMap<String, T>,
    },
    Bare(T),
}
```

The container enum is untagged so serde can automatically deserialize either the wrapped `{ default: ..., overrides: ... }` map or the bare `T` struct.
When serializing, if `overrides` is empty or if constructed as `Bare(T)`, it emits cleanly.
A helper method `resolve(&self, target: &str) -> &T` handles resolution for downstream consumers.

**2. Full replacement resolution semantics.**

For target `T`, resolution checks:

1. Exact match in `overrides` for the target format name (e.g. `cursor-mdc`).
2. Prefix or tool family alias in `overrides` (e.g. `cursor` when target is `cursor-mdc` or `cursor-mcp`).
3. Fallback to `default` (or the bare `T` value).

There is no field-by-field merge between `default` and `overrides[T]`.
If a target override specifies `mode: [always]`, it does not inherit `globs` from `default`.
This ensures readers of rule frontmatter can see the exact target behavior without mental map reconciliation.

**3. Integration into `RuleMetadata`.**

In `src/ir/mod.rs`, `RuleMetadata` updates its `activation` field:

```rust
pub struct RuleMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "rulette:activation", skip_serializing_if = "Option::is_none")]
    pub activation: Option<TargetOverrides<Activation>>,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}
```

Emitters query `rule.metadata.activation.as_ref().map(|a| a.resolve("cursor-mdc"))` (or their respective target key) to obtain a resolved `&Activation`.

**4. Scoped to `rulette:activation` only for now.**

While `TargetOverrides<T>` is generic, only `RuleMetadata::activation` uses it in this change.
This satisfies YAGNI while ensuring future keys like `rulette:tool-access` can adopt `TargetOverrides<ToolAccessRule>` without schema divergence or breaking changes.

**5. Schema generation for `rulette:activation`.**

`schemars` automatically generates a JSON Schema representing the `anyOf` / untagged union of `Wrapped` and `Bare(Activation)` for `rulette:activation`.
Running `rulette schema rulette:activation` outputs this schema, documenting both bare and override-wrapped syntax for users and IDE tooling.

## Adversarial review

**1. Untagged serde deserialization ambiguity and validation failure modes.**

- *Issue*: If a user writes an invalid key under `rulette:activation`, serde's untagged deserialization could attempt `Wrapped`, fail, attempt `Bare(Activation)`, fail, and report a generic error message.
- *Mitigation*: In `TargetOverrides`, `Wrapped` requires a `default` mapping, which is disjoint from the valid field set of `Activation` (`mode`, `globs`, `pattern`, `description`). Comprehensive unit tests will verify that malformed YAML produces meaningful errors across both bare and wrapped shapes.

**2. Unknown or future target keys in `overrides`.**

- *Issue*: A user may author rules with overrides for tools not yet supported by Rulette (e.g. `opencode` or custom tools).
- *Mitigation*: The `overrides` map stores `BTreeMap<String, T>` without an enum restriction on target key names. Unknown target keys are preserved losslessly in the IR and ignored during resolution for existing targets, ensuring forward compatibility.

**3. Target key matching and casing conventions.**

- *Issue*: Inconsistent casing (e.g. `Cursor` vs `cursor` vs `cursor-mdc`) could cause override lookup misses.
- *Mitigation*: Target matching normalizes keys by lowercasing and trimming before lookup. `resolve` evaluates exact format names first (e.g. `cursor-mdc`), then tool family prefixes (e.g. `cursor`), before falling back to `default`.

**4. Emitter capability inspection parity.**

- *Issue*: Capability inspection (`inspect --coverage` and `inspect --to`) must check the resolved target activation rather than the un-resolved wrapper.
- *Mitigation*: Emitter `capabilities()` implementations will consume the target-resolved `Activation` directly, ensuring coverage reports reflect the exact configuration emitted for each target.

## Risks / Trade-offs

- **[Risk]** Untagged serde deserialization ambiguity if `T` and `Wrapped` share overlapping keys.
  → Mitigation: `Wrapped` requires a `default` key, whereas `Activation` has `mode`, `globs`, `pattern`, and `description`.
- **[Trade-off]** Full replacement means repeated fields when an override only tweaks one property of a complex default.
  → Mitigation: Activation configurations are small (typically 1–3 fields), so predictability and transparency outweigh the minor duplication.

## Open Questions

Resolved during design and review:

- Key matching order: exact target name (`cursor-mdc`) precedes family alias (`cursor`), falling back to `default`.
- Forward compatibility: `overrides` keys are dynamic strings, allowing future targets without schema breaks.
