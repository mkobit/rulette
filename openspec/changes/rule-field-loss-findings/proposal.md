## Why

Rule lowering currently preserves instruction bodies while silently discarding descriptions or conditional activation.
Strict compilation must reject these losses, and explicitly permitted losses must remain inspectable in staged plans.

## What changes

- Report `rule-description-unrepresentable` and `rule-activation-unrepresentable` as Lossy findings for Codex, Claude, OpenCode, Cursor, and Antigravity.
- Analyze resolved activation with existing renderer keys and branch precedence, aggregating lost fields per package and target.
- Identify these reasons and lost fields in strict rejection diagnostics and retain them through staged-plan parsing.
- Preserve rendering, graph schemas, layouts, override resolution, and AgentPlugin behavior.

## Impact

Backend capability inspection and strict transform compilation become accurate for rule fields.
Migrations that previously depended on silent loss must explicitly request `--allow-lossy`.

## Approval and review

Mike approved reporting lost rule descriptions and activation, rejecting by default, and permitting output with retained findings under `--allow-lossy`.
Design and independent adversarial review completed before implementation bead `rulette-6lj.1` was staged.
The accepted review conditions are recorded in design.md.
