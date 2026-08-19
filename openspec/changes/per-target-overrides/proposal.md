## Why

Rule files often require tool-specific activation semantics across different editors and agents.
For example, Cursor uses `alwaysApply: true` or glob patterns, while Antigravity uses explicit trigger modes (`always_on`, `glob`, `manual`, `model_decision`) and matching descriptions.
Currently, `rulette:activation` only supports a single flat `Activation` configuration, forcing per-target divergence into untyped `extra` maps where cross-target awareness and validation are lost.
`docs/2026-08-18-cli-ux-design.md` §3 specifies a typed per-target override wrapper for `rulette:activation` that resolves predictably per target emitter.
This change implements the typed `{ default: T, overrides: map<target, T> }` model for activation metadata, tracked in `rulette-5bk.8`.

## What Changes

- Add a parametric `TargetOverrides<T>` container supporting both a bare value `T` and a wrapped `{ default: T, overrides: map<target, T> }` form.
- Update `rulette:activation` in the IR (`RuleMetadata`) to deserialize from either the flat `Activation` shape or the wrapped `{ default: Activation, overrides: ... }` shape, maintaining backwards compatibility.
- Implement target-specific resolution semantics: an emitter for target `T` resolves `overrides[T]` if present, otherwise falling back to `default`.
- Resolution uses full replacement rather than deep merging: an override completely replaces default activation settings for that target.
- Target emitters query resolved activation settings for their target identifier instead of reading raw flat fields.
- Update schema generation for `rulette:activation` to expose the new wrapped and flat schema definitions.

## Capabilities

### Modified Capabilities

- `ir-core`: The `rulette:activation` metadata envelope supports per-target override blocks with full replacement fallback semantics.
- `frontends-and-backends`: Emitters resolve target-specific activation overrides when emitting target configuration files.

## Impact

- `src/ir/mod.rs`: Introduce `TargetOverrides<T>` and update `RuleMetadata::activation`.
- `src/emitters/cursor.rs` and other emitters: Resolve activation settings for the specific emitter target.
- `src/parsers/frontend.rs`: Parse both flat and wrapped `rulette:activation` blocks from source rule frontmatter.
- `src/cli/commands/schema.rs`: Ensure schema generation for `rulette:activation` reflects the updated type definition.
- `openspec/specs/ir-core/spec.md`: Update specification requirements for target overrides on activation metadata.
- Tracking: bead `rulette-5bk.8` (parent `rulette-5bk`).
