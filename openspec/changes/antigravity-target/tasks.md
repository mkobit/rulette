## 1. Format definitions and parser

- [x] 1.1 Add `InputFormat::Antigravity` and `OutputFormat::Antigravity` to `src/cli/formats.rs`.
- [x] 1.2 Define `AntigravityRuleFrontmatter` and `AntigravityTrigger` in `src/parsers/antigravity.rs`.
- [x] 1.3 Implement Antigravity rule parsing and auto-detection in `src/parsers/frontend.rs`.

## 2. Emitter implementation

- [x] 2.1 Implement `AntigravityEmitter` in `src/emitters/antigravity.rs` with trigger resolution and skill emission.
- [x] 2.2 Export `AntigravityEmitter` in `src/emitters/mod.rs` and `src/lib.rs`.
- [x] 2.3 Add capability parity test for `AntigravityEmitter` in `src/emitters/mod.rs`.

## 3. CLI integration and scaffolding

- [x] 3.1 Register Antigravity in `src/cli/commands/transform.rs` target parser and `TOOL_PATH_CONVENTIONS`.
- [x] 3.2 Register Antigravity in `src/cli/commands/inspect.rs` dry-run dispatch and `coverage_targets`.

## 4. Tests and verification

- [x] 4.1 Add parser and emitter unit tests for Antigravity trigger modes and overrides.
- [x] 4.2 Add integration tests verifying end-to-end transformation to Antigravity format.
- [x] 4.3 Run full test suite and validate all OpenSpec specifications.
