## 1. Establish the breaking graph boundary

- [x] 1.1 Add runtime `sha2` and `hex` dependencies in `Cargo.toml` for graph identifiers, and remove the unused runtime `ureq` dependency so the compiler library has no HTTP client dependency.
- [x] 1.2 Create `src/ir/graph.rs` for the public `CompilationGraph` model alongside the legacy document model needed by not-yet-migrated internal callers.
- [x] 1.3 Export graph APIs from `src/lib.rs` without building a graph-to-document compatibility shim, and defer removal of legacy public exports and direct entity interchange until task section 8 after every caller has migrated.
- [x] 1.4 Keep native parsing, graph validation, selection, capability analysis, and lowering in library modules, with `src/cli/**` limited to argument decoding, input-handle setup, diagnostic rendering, and output delegation.
- [x] 1.5 Add compile-time and unit coverage in `src/ir/mod.rs` and `src/lib.rs` proving the graph API does not expose clap types, caller paths, archive handles, or filesystem handles.

## 2. Implement the validated compilation graph

- [x] 2.1 Define `CompilationGraph`, `Package`, `PackageId`, `SemanticIdentity`, `PackageKind`, `SemanticItem`, `Resource`, `ResourcePath`, `ResourceRole`, `SourceProvenance`, `FrontendPayload`, and structured diagnostic types in `src/ir/graph.rs`.
- [x] 2.2 Model a supported package as exactly one rule or skill semantic item with one UTF-8 Markdown primary instruction, and model recognized agents, hooks, MCP servers, permissions, and other unsupported units as non-portable packages with an unsupported-semantic diagnostic.
- [x] 2.3 Implement graph validation for supported graph version `0.1`, stable `BTreeMap` ordering, one primary instruction, package-root containment, unique semantic identities, and unique package identifiers.
- [x] 2.4 Implement canonical `PackageId` construction from graph version, package unit kind, semantic identity, and sorted resource path, executable-bit, and content-digest tuples.
- [x] 2.5 Implement `ResourcePath` validation that rejects empty paths or components, dot components, traversal, backslashes, NUL or control characters, absolute and platform-prefixed paths, and normalized intra-package collisions.
- [x] 2.6 Preserve primary instructions as UTF-8 text and opaque resources as either UTF-8 text or exact bytes, with an executable-bit boolean that defaults to false only when the source cannot report it.
- [x] 2.7 Preserve only content-safe provenance in serialized graph values, including the recognized frontend, normalized relative input label or `stdin`, and archive member path, while hashing absolute input roots into the `input_<sha256>` label.
- [x] 2.8 Keep unknown native frontmatter and configuration in a namespaced frontend payload owned by its package or semantic item, and ensure it cannot alter portable rule or skill meaning.
- [x] 2.9 Add focused unit tests in `src/ir/graph.rs` for package identity stability, identity collisions, resource paths, resource roles, provenance redaction, unsupported packages, package boundaries, and deterministic map ordering.

## 3. Replace entity interchange with strict graph interchange

- [x] 3.1 Add graph JSON and TOML serialization and deserialization in `src/ir/graph.rs` or a focused `src/ir/serialization.rs` module using stable field order, sorted collections, canonical unwrapped RFC 4648 base64, UTF-8 text, and a terminal newline for JSON.
- [x] 3.2 Make graph deserialization require exactly the supported `graph_version` and reject unknown structural fields, invalid enum values, invalid base64, duplicate package identifiers, duplicate semantic identities, and non-canonical resource paths before selection or lowering.
- [x] 3.3 Add graph schema support in `src/cli/commands/schema.rs` and graph documentation coverage in `src/bin/gen_docs.rs`, and defer replacement of legacy CLI schema output until task section 8.
- [x] 3.4 Add canonical graph fixtures and snapshots alongside legacy fixtures until the CLI migration removes the legacy surface in task section 8.
- [x] 3.5 Add graph interchange tests for byte-identical repeated serialization, JSON and TOML re-ingestion, invalid base64, unknown fields, unsupported versions, and rejected legacy `RuletteDocument` documents.

## 4. Build safe raw observation discovery

- [x] 4.1 Create a library-owned observation module such as `src/inputs/mod.rs` with `ArtifactObservation` values containing bytes, normalized source-relative path, executable metadata, provenance, and filesystem, stdin, tar, or gzip-tar origin.
- [x] 4.2 Move discovery policy out of the text-only `src/cli/io.rs` reader so the CLI passes caller-selected input handles to the library without filtering files by extension or decoding bytes before frontend classification.
- [x] 4.3 Implement deterministic directory discovery with `symlink_metadata`, reject every symlink before reading it, and copy ordinary filesystem hard-link contents without retaining hard-link identity.
- [x] 4.4 Implement tar and gzip-tar discovery that rejects all non-regular entries, including symbolic links and hard links, duplicate normalized member names, unsafe GNU or PAX path overrides, traversal, absolute paths, platform prefixes, and input-root escapes before any frontend observes content.
- [x] 4.5 Enforce the 10,000-observation, 32 MiB-per-resource, and 256 MiB-total-resource limits during discovery and fail before graph construction, lowering, staging, or publication.
- [x] 4.6 Add unit tests in `src/inputs/mod.rs` and integration fixtures under `tests/fixtures/` for byte preservation, executable metadata, symlink rejection, archive links, duplicate archive members, PAX and GNU path safety, malformed paths, and each resource budget.

## 5. Migrate the five native frontends

- [x] 5.1 Replace the string-only `parse` contract in `src/parsers/frontend.rs` with a frontend interface that consumes ordered observations and returns a validated `CompilationGraph` plus structured diagnostics.
- [x] 5.2 Migrate Codex discovery in `src/parsers/codex.rs` to recognize documented project files and construct rule or skill packages with package roots, primary instructions, provenance, and safe opaque resources.
- [x] 5.3 Migrate Claude discovery in `src/parsers/claude.rs` to construct graph packages and classify settings, agents, hooks, MCP servers, permissions, and other non-portable semantics as unsupported rather than portable entities.
- [x] 5.4 Migrate Cursor discovery in `src/parsers/cursor.rs` to construct rule and skill packages from documented native layouts and preserve only package-owned safe resources.
- [x] 5.5 Migrate OpenCode discovery in `src/parsers/opencode.rs` to construct graph packages from documented configuration, rules, skills, and agent layouts, classifying non-portable agent and MCP semantics as unsupported packages.
- [x] 5.6 Migrate Antigravity discovery in `src/parsers/antigravity.rs` to construct graph rule and skill packages with target-resolvable activation and safe package resources.
- [x] 5.7 Make `InputFormat::Auto` and `--from` in `src/cli/formats.rs` and `src/parsers/frontend.rs` accept the five core harness frontends plus graph interchange only, and remove legacy non-core parser formats and auto-detection branches.
- [x] 5.8 Add one golden fixture tree per core frontend under `tests/fixtures/v0_1/` and graph assertions in `tests/cli_tests/parse_tests.rs` for package roots, primary instructions, identities, provenance, opaque bytes, executable metadata, unsupported semantics, and deterministic auto-detection.

## 6. Introduce pure capability-aware backend lowering

- [x] 6.1 Replace the write-capable `Emitter` contract in `src/emitters/mod.rs` with a backend contract that accepts a selected graph and returns a deterministic typed `LoweringPlan` plus per-package and per-resource capability findings without reading or writing caller paths.
- [x] 6.2 Define stable `Supported`, `Lossy`, and `Dropped` capability findings with a machine-stable reason, package identity, resource identity when applicable, and source provenance, and aggregate the worst finding per selected package and target.
- [x] 6.3 Migrate `src/emitters/codex.rs`, `src/emitters/claude.rs`, `src/emitters/cursor.rs`, `src/emitters/opencode.rs`, and `src/emitters/antigravity.rs` to lower only the graph packages their target can represent.
- [x] 6.4 Require each core backend to return a hard collision for duplicate target artifact class and normalized native path before returning its lowering plan.
- [x] 6.5 Require each core backend to report loss when it cannot preserve an executable resource bit, and preserve opaque resources only for an explicitly supported same-domain package shape.
- [x] 6.6 Permit a rule-only target to lower a skill primary instruction as a rule only through the explicit lossy policy, and report every omitted opaque resource as dropped rather than inventing a target layout.
- [x] 6.7 Preserve target activation override resolution in exact target-name, normalized family-alias, then default order, and reject duplicate keys after trimmed lower-case normalization.
- [x] 6.8 Remove non-core emitter registration and old `RuletteDocument` capability-parity tests from `src/emitters/mod.rs`, while retaining shared Agent Skills name validation only if the graph skill validator needs it.
- [x] 6.9 Add backend unit tests in each core emitter module and shared parity tests in `src/emitters/mod.rs` for deterministic artifacts, supported and lossy findings, artifact collisions, same-domain opaque resources, cross-domain drops, executable metadata, skill-to-rule lowering, and activation precedence.

## 7. Replace public mutation transforms with selection and strict loss handling

- [x] 7.1 Replace `src/pipeline/mod.rs` metadata mutation helpers with graph selection by exact `PackageId`, preserving selected package resources and provenance without modifying graph content.
- [x] 7.2 Update `src/cli/commands/transform.rs` to accept repeatable `--select <package-id>` and select all packages in stable package-ID order when the flag is absent.
- [x] 7.3 Remove `--filter`, `--exclude`, `--rename`, `--set`, and metadata overrides from `src/cli/commands/transform.rs`, move `--strict` from `src/cli/globals.rs` to `InspectArgs` only, and delete obsolete transform configuration and mutation tests.
- [x] 7.4 Add `--allow-lossy` to `transform`, make loss a pre-write compilation error by default, and retain every structured loss finding in the compilation result when the escape hatch is present.
- [x] 7.5 Keep `inspect --coverage --strict` as the informational non-zero coverage gate, but migrate `src/cli/commands/inspect.rs` to query the same graph capability data and package/resource provenance that lowering uses.
- [x] 7.6 Replace transform configuration in `src/cli/commands/transform.rs` with a deny-unknown-fields input schema limited to source inputs, target and logical-scope requests, and a sorted exact `select` array, rejecting mutation pipelines and global or per-output strictness fields.
- [x] 7.7 Ensure transform configuration is read only when explicitly passed with `--config` and cannot introduce loss permission, write authority, stage-root selection, destination paths, local state, automatic discovery, plugins, or network behavior.
- [x] 7.8 Add CLI tests in `tests/cli_tests/transform_tests.rs`, `tests/cli_tests/strict_tests.rs`, `tests/cli_tests/coverage_tests.rs`, and `tests/cli_tests/transform_config_tests.rs` for all-package selection, deterministic selection union, no-match failure, removed flag usage errors, strict-by-default loss failure, `--allow-lossy` diagnostics, graph stdout rejecting `--allow-lossy`, and restricted configuration rejection.

## 8. Complete the graph migration without dual representations

- [x] 8.1 Update `src/cli/commands/transform.rs`, `src/cli/commands/inspect.rs`, `src/cli/commands/schema.rs`, `src/lib.rs`, and all core frontend and backend call sites to use `CompilationGraph` exclusively.
- [x] 8.2 Remove `RuletteDocument`, `Entity`, legacy exports, old entity-specific parser and emitter pathways, the mutable `pipeline` API, and unsupported format registrations in the same change so no runtime branch can silently fall back to the old portability contract.
- [x] 8.3 Keep the public target set limited to Codex, OpenCode, Claude, Cursor, and Antigravity, with graph JSON, graph TOML, and schema available only as debugging and interchange surfaces rather than native harness targets.
- [x] 8.4 Update `docs/2026-04-11-prd.md`, `docs/2026-04-11-man-page.md`, `docs/2026-04-11-announcement.md`, `docs/2026-04-11-landscape.md`, and generated CLI documentation to describe the narrowed graph, package, selection, strict-loss, and five-target contract.
- [x] 8.5 Add migration regression tests that prove legacy IR, removed formats, and removed mutation flags fail with clear usage or compatibility errors instead of producing partial output.

## 9. Validate the compiler kernel

- [x] 9.1 Run focused graph, input, frontend, backend, selection, coverage, and configuration tests while each migration unit lands, including every new scenario in `tests/cli_tests/` and module tests.
- [x] 9.2 Run the complete Rust suite with `mise run test`, then run `mise run fmt`, `mise run lint`, and `mise run spec-validate` after the OpenSpec change is complete.
- [x] 9.3 Build the release binary with `mise run build` and verify graph parsing and lowering do not make network requests, dynamically load code, auto-discover configuration, retain local state, or write native outputs before the staged-publication layer is invoked.
- [x] 9.4 Hand the validated graph and lowering APIs to the staged-publication change for destination mapping and apply transactions, and do not add direct filesystem publication back into a frontend or backend.
