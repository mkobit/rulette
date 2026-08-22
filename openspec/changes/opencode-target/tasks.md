## 1. Format definitions and parser

- [x] 1.1 Add `InputFormat::OpenCode` and `OutputFormat::OpenCode` to `src/cli/formats.rs`.
- [x] 1.2 Define OpenCode data structures in `src/parsers/opencode.rs`.
- [x] 1.3 Implement OpenCode JSON config and agent markdown parsing and auto-detection in `src/parsers/frontend.rs`.

## 2. Emitter implementation

- [x] 2.1 Implement `OpenCodeEmitter` in `src/emitters/opencode.rs` supporting rules, agents, skills, and MCP configurations.
- [x] 2.2 Export `OpenCodeEmitter` in `src/emitters/mod.rs` and `src/lib.rs`.
- [x] 2.3 Add capability parity test for `OpenCodeEmitter` in `src/emitters/mod.rs`.

## 3. CLI integration and scaffolding

- [x] 3.1 Register OpenCode in `src/cli/commands/transform.rs` target parser, `TOOL_PATH_CONVENTIONS`, and emission dispatch.
- [x] 3.2 Register OpenCode in `src/cli/commands/inspect.rs` dry-run dispatch and `coverage_targets`.

## 4. Tests and verification

- [x] 4.1 Add parser and emitter unit tests for OpenCode config, agents, skills, and rules.
- [x] 4.2 Add integration tests verifying end-to-end transformation to OpenCode format and round trips.
- [x] 4.3 Run full test suite and validate all OpenSpec specifications.
