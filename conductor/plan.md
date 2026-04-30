# Implementation Plan: Modularity, CLI Alignment, & Jules Workflow

## Objective
Refactor Rulette to support 5-10 daily asynchronous tasks by "Jules" by modularizing the codebase, updating the product definition to deprecate legacy Claude commands, fixing the identified CLI misalignment bugs, and defining daily execution prompts.

## Phase 1: Product Definition & Documentation
- [x] Update `conductor/product.md` to establish the new constraints (Claude Skills over Commands, Modularity).
- [x] Prune the PRD (`docs/2026-04-11-prd.md`) to remove references to the Claude `commands/` directory and explicitly document the shift to [Claude Agent Skills](https://github.com/anthropics/skills).

## Phase 2: Modularity & Refactoring (Technical Rails)
- [x] Step 1: Extract IR definitions from `src/lib.rs` into a new `src/ir/` module.
- [x] Step 2: Move parsing logic (`src/frontend.rs`, `src/claude.rs`, `src/cursor.rs`, etc.) into a dedicated `src/parsers/` module.
- [x] Step 3: Move emitting logic (`src/backend.rs`) into a dedicated `src/emitters/` module.
- [x] Step 4: Extract transformation pipeline logic from `src/cli/commands/transform.rs` into `src/pipeline/`.
- [x] Step 5: Ensure tests are colocated with their bounded contexts to allow localized test execution.

## Phase 3: CLI Misalignment Fixes (Backlog Items)
These will be independent tasks that can be executed concurrently by Jules once the rails are laid:
- [x] Task A: Update `inspect` to use `--to` instead of `--target`.
- [x] Task B: Update `schema` to use `--to` instead of `--format`.
- [x] Task C: Fix the `transform` config override bug where output targets are ignored during emission.
- [x] Task D: Update `ClaudeEmitter` to properly emit `Entity::Skill` as distinct files instead of concatenating them into `CLAUDE.md`.

## Phase 4: Jules Daily Prompts
These prompts are optimized for the new modular architecture. Jules should pick these up one by one.

**Prompt 1: Expand IR & Schema**
> "Your task is to expand the Rulette Intermediate Representation in `src/ir/mod.rs`. Add a new entity kind 'EnvironmentPolicy' with fields for `variables` and `secrets_masking`. Update the JSON schema generation in `src/cli/commands/schema.rs` if needed and add unit tests specifically within the `ir` module."

**Prompt 2: New Parser Implementation**
> "Your task is to add a new parser format for 'GitHub Copilot Rules'. Create `src/parsers/copilot_rules.rs`, register it in `src/parsers/mod.rs`, and implement logic to produce IR `Rule` entities. Ensure this logic is fully contained and includes colocated parser tests without touching the core `lib.rs` or `emitters/`."

**Prompt 3: Emitter Specialization**
> "Your task is to enhance the `WindsurfEmitter` in `src/emitters/windsurf.rs`. Currently, it concatenates rules into a single `.windsurfrules` file. Modify it to support emitting directory-scoped rules if the `rulette:directory-scope` metadata is present, creating multiple `.windsurfrules` files at the specified sub-paths."

**Prompt 4: Pipeline Extension**
> "Your task is to build out a new transformation feature in `src/pipeline/mod.rs`. Add a `TemplateEngine` that allows rule bodies to be interpolated with metadata values (e.g., `{{name}}`). Update the transform command to trigger this if a `--template` flag is provided (you will need to add this flag to `src/cli/commands/transform.rs`)."

## Verification & Rollback
- After the modularization (Phase 2), run `cargo test` to ensure the compilation and existing behaviors remain identical.
- We will track task progress via independent Git branches or tracked issues in `conductor/backlog.md`.
