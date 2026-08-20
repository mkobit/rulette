## Why

Antigravity is one of the core tools in the modern AI development ecosystem and replaces Gemini CLI in this project's roadmap.
Previously, Rulette lacked an Antigravity parser frontend and emitter backend, preventing users from translating rules and configurations to and from Antigravity.
This change introduces full Antigravity format support, including trigger-mode activation translation and skill emission, tracked in `rulette-5bk.10`.

## What Changes

- Add `antigravity` as a supported input format (`InputFormat::Antigravity`) and output format (`OutputFormat::Antigravity`).
- Implement the Antigravity parser frontend capable of extracting rule metadata, frontmatter trigger modes (`always_on`, `glob`, `manual`, `model_decision`), globs, and descriptions.
- Implement the Antigravity emitter backend supporting rule emission to `.antigravity/` (or specified destination) with resolved activation triggers and skill emission to `skills/<name>/SKILL.md`.
- Integrate Antigravity into CLI commands (`transform`, `inspect`, format auto-detection, and scaffold conventions).
- Implement capability reporting and strict parity validation for the Antigravity emitter.

## Capabilities

### Modified Capabilities

- `frontends-and-backends`: Support auto-detecting, parsing, inspecting, and emitting Antigravity rules and configuration files.

## Impact

- `src/cli/formats.rs`: Add `InputFormat::Antigravity` and `OutputFormat::Antigravity`.
- `src/parsers/antigravity.rs`: Create Antigravity parser metadata structures.
- `src/parsers/frontend.rs`: Integrate Antigravity parser and auto-detection.
- `src/emitters/antigravity.rs`: Create `AntigravityEmitter` implementing `Emitter`.
- `src/emitters/mod.rs`: Export `AntigravityEmitter` and add capability parity tests.
- `src/cli/commands/transform.rs`: Register Antigravity in output targets and scaffold conventions.
- `src/cli/commands/inspect.rs`: Register Antigravity in dry-run emissions and coverage targets.
- `openspec/specs/frontends-and-backends/spec.md`: Add requirements for Antigravity parsing and emission.
- Tracking: bead `rulette-5bk.10` (parent `rulette-5bk`).
