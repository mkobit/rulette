## Context

The library lowers graph packages before publication.
Existing renderers choose native activation using target resolution and mode precedence; capability classification must describe those actual bytes without changing them.

## Decisions

Use two stable Lossy reason codes: `rule-description-unrepresentable` and `rule-activation-unrepresentable`.
Emit at most one finding of each reason per package and target, with package provenance and the actual native artifact path.
Aggregate omitted activation fields in order: mode, globs, pattern, description.
Name distinct discarded modes deterministically; duplicate occurrences of the selected mode do not imply loss.

Plain Codex, Claude, and OpenCode instructions omit nonempty rule descriptions and conditional activation.
Absent activation or Always alone is unconditional and supported.
Only resolved activation matters; unused overrides do not contribute findings.
Cursor resolves `cursor-mdc` with existing `cursor` alias fallback; other targets use their current target keys.

Cursor preserves its emitted description and Always when represented by true alwaysApply.
With false alwaysApply, nonempty emitted globs preserve Glob; missing or empty emitted globs preserve Manual instead, including mixed Glob/Manual activation.
Antigravity preserves the mode selected by always_on, glob, model_decision, manual, then Pattern-to-glob precedence.
Globs are preserved only in branches that actually emit them.
Pattern text is never emitted.
Antigravity model_decision activation description is preserved only when it matches the emitted rule description after whitespace normalization.
Cosmetic whitespace alone is not material loss.

Strict diagnostics append details only for these approved rule-loss codes; unrelated loss diagnostics retain their existing behavior.
Staged-plan parsing accepts both codes without changing the plan schema.
AgentPlugin remains outside scope because its existing finding already reports native activation loss.

## Independent adversarial review record

Review completed before implementation staging; all conditions accepted:

1. Strict rejection identifies stable loss codes and lost fields.
2. Mode preservation derives from the selected renderer branch, not target-wide support.
3. Cursor Manual has a strict-success regression.
4. Antigravity matching model description succeeds strictly; differing description rejects.
5. Overrides are tested in both directions, retaining existing cursor-mdc and cursor alias semantics.
6. At most one aggregated finding exists per reason, package, and target.
7. AgentPlugin remains outside scope.

## Validation

Write failing regressions before implementation for strict diagnostics, permitted losses, staged-plan roundtrip, absent and unconditional metadata, native mode precedence, overrides, deterministic aggregation, and all-or-nothing multi-target staging.
Final review regressions cover missing and empty Cursor globs, strict success for real Glob and Manual, and mixed Glob/Manual precedence based on actual output.
Run focused tests, `mise run check`, and `mise run spec-validate` in the current sandbox; record final review output in `/tmp/rulette-rule-loss-final-check.log`.
No resource, layout, plugin, configuration, renderer, or schema expansion is authorized.
