## Dependency map

1 -> 2, 3.

2 + 3 -> 4.

3 -> 5, 6.

4 + 5 + 6 -> 7.

7 -> 8.

8 -> 9.

1-9 -> 10.

## 1. Establish the library-first snapshot aggregation boundary

- [x] 1.1 Create a library-owned aggregation module such as `src/parsers/aggregation.rs` that accepts discovered observations and a decoder-selection value without Clap arguments, caller filesystem paths, archive handles, or publication handles.
- [x] 1.2 Define a transient aggregation-candidate type containing one validated `Package` unchanged and a separate outer explicit-input identity derived from normalized content-safe `ObservationProvenance.input_label`.
- [x] 1.3 Retain archive member provenance separately from outer identity and never derive outer identity from argv position, discovery order, or an unnormalized absolute path.
- [x] 1.4 Keep the outer identity out of `SourceProvenance`, `CompilationGraph`, graph JSON, and graph TOML, and preserve the package's existing provenance unchanged.
- [x] 1.5 Export the aggregation entry point through `src/parsers/mod.rs` and `src/lib.rs` while retaining `src/cli/**` as argument decoding, handle setup, diagnostic rendering, and library delegation only.
- [x] 1.6 Add compile-time and unit tests proving the public aggregation API has no dependency on Clap, caller paths, stdin readers, archive readers, staging roots, or apply authority.

Acceptance evidence: `cargo test parsers::aggregation` proves aggregation candidates preserve package bytes and package provenance while retaining outer identity only in transient state.

## 2. Make input discovery share one invocation ledger

- [x] 2.1 Refactor `src/inputs/mod.rs` so one library-owned observation collector or ledger is created for a complete explicit source set rather than once per path, archive, or stdin reader.
- [x] 2.2 Route directory, regular-file, tar, gzip-compressed tar, and accepted standard-input discovery through that shared ledger.
- [x] 2.3 Enforce the existing 10,000-observation, 32 MiB per-resource, and 256 MiB total invocation limits cumulatively across every explicit path and the one permitted standard-input stream.
- [x] 2.4 Preserve the existing path normalization, symlink rejection, hostile archive-member and path rejection, byte preservation, executable metadata, and content-safe input labels while changing collector ownership.
- [x] 2.5 Extend `src/inputs/tests.rs` with split-path, path-plus-archive, and path-plus-stdin fixtures that individually fit the limits but cumulatively exceed each applicable limit.
- [x] 2.6 Add hostile tar and gzip-tar regressions for symbolic and hard links, duplicate members, traversal, absolute paths, platform-prefixed paths, and unsafe PAX or GNU path overrides while the same shared ledger is active.

Acceptance evidence: focused input tests prove one source set fails before frontend compilation when its aggregate observation count or byte total exceeds the configured ledger limit.

## 3. Implement homogeneous decoder selection and supported stdin modes

- [x] 3.1 Add a library decoder-selection model in `src/parsers/frontend.rs` that distinguishes explicit native frontend selection, explicit `graph-json`, explicit `graph-toml`, and `--from auto` native resolution.
- [x] 3.2 Make `--from auto` inspect the complete discovered native source set, reject any set with more than one recognized native frontend family, and otherwise succeed only when one family is unambiguous.
- [x] 3.3 Make explicit `--from <frontend>` reject every observation recognized as another native frontend before aggregation.
- [x] 3.4 Reject native-plus-interchange inputs and graph JSON plus graph TOML inputs, and send files matching no known frontend through unrecognized-file classification followed by unsupported-source-syntax failure when no package results.
- [x] 3.5 Restrict standard input in `src/inputs/mod.rs` and `src/cli/commands/transform.rs` to at most one explicitly selected tar, gzip-compressed tar, graph JSON, or graph TOML source, and reject plain native stdin without inventing a filename or package root.
- [x] 3.6 Preserve syntax validation before source I/O while deferring resolved backend work until the complete decoder and input set has passed source discovery and aggregation validation.
- [x] 3.7 Add unit tests in `src/parsers/frontend.rs` and CLI integration tests in `tests/cli_tests/transform_tests.rs` for explicit decoder selection, homogeneous auto success, auto multi-family rejection, explicit-frontend foreign-observation rejection, unknown-file classification, explicit-only graph interchange, native stdin rejection, valid tar and gzip-tar stdin, and rejected repeated stdin.

Acceptance evidence: `cargo test parsers::frontend` and transform CLI tests demonstrate that only one homogeneous decoder family reaches parsing and plain native stdin never reaches a native frontend.

## 4. Aggregate multiple graph interchange documents deterministically

- [x] 4.1 Replace the single-observation branch in `compile_graph_interchange` in `src/parsers/frontend.rs` with explicit JSON or TOML decoding of every supplied interchange document.
- [x] 4.2 Require every decoded document to pass the existing exact graph-version and strict schema validation before it contributes packages or diagnostics.
- [x] 4.3 Convert each decoded package into an aggregation candidate that retains the package's serialized provenance and records the source document's separate outer input identity.
- [x] 4.4 Merge decoded packages and diagnostics in stable order without serializing or otherwise injecting outer identity into the resulting graph.
- [x] 4.5 Add graph-interchange tests for multiple valid JSON documents, multiple valid TOML documents, unsupported version or schema failure in any document, stable diagnostic merge order, input-order permutations, and rejected JSON-plus-TOML input sets.

Acceptance evidence: graph frontend tests prove equivalent permutations of valid same-encoding documents yield byte-identical canonical graph output and no outer identity field appears in serialized graph data.

## 5. Make native frontend observation classification complete

- [x] 5.1 Extend the native frontend contract in `src/parsers/frontend.rs` and `src/parsers/mod.rs` so each observation is classified as package content, retained unsupported content, unrecognized-file warning, or fatal malformed input.
- [x] 5.2 Update `src/parsers/codex.rs` and `src/parsers/claude.rs` to emit deterministic unrecognized-file warnings for ignored safe files and an unsupported-source-syntax error when the selected frontend produces zero packages.
- [x] 5.3 Update `src/parsers/cursor.rs`, `src/parsers/opencode.rs`, and `src/parsers/antigravity.rs` to return the same classification results instead of silently omitting unmatched observations.
- [x] 5.4 Preserve existing hard failures for malformed recognized content and unsafe observations, and keep retained unsupported semantic content in its owning package under the graph kernel contract.
- [x] 5.5 Add focused parser tests for unknown files, recognized unsupported content, malformed recognized content, and source sets that yield only warnings or retained unsupported content but no packages.

Acceptance evidence: parser and CLI tests prove every discovered observation has one recorded disposition and that selected Codex and Claude invocations with no packages fail deterministically.

## 6. Make native skill grouping input-aware

- [x] 6.1 Replace root-only skill-group keys in `src/parsers/cursor.rs`, `src/parsers/opencode.rs`, and `src/parsers/antigravity.rs` with keys composed of content-safe explicit-input identity and normalized package root.
- [x] 6.2 Audit equivalent grouping in `src/parsers/codex.rs` and `src/parsers/claude.rs` so every native frontend applies the same input-aware grouping invariant.
- [x] 6.3 Ensure resources from different explicit snapshots never join one candidate package before aggregate collision collection, even when their native paths and package roots are identical.
- [x] 6.4 Preserve grouping of all members from one directory or one tar input under the same valid package root.
- [x] 6.5 Add frontend fixtures for Cursor, OpenCode, and Antigravity that use equal skill roots in distinct inputs, plus regression fixtures proving one tar input still groups companion resources into one skill package.

Acceptance evidence: frontend tests show same-root resources from two input labels produce separate candidates and a later collision, while members from one input still form one complete skill package.

## 7. Collect aggregate collisions before graph construction

- [x] 7.1 Implement deterministic pre-construction indexes in `src/parsers/aggregation.rs` keyed independently by `SemanticIdentity` and `PackageId`.
- [x] 7.2 Collect every collision group from both indexes before constructing a `CompilationGraph`, rather than returning after the first duplicate.
- [x] 7.3 Render each group in stable key order with the shared key, every candidate package ID, each candidate's unchanged package provenance, and its separate outer explicit-input identity in deterministic outer-identity order.
- [x] 7.4 Keep byte-identical duplicate packages as collisions and prohibit winner selection by input order, timestamp, generator name, directory depth, or hash order.
- [x] 7.5 Ensure collision failure occurs before backend registry resolution, capability analysis, lowering, stage creation, destination checking, or apply.
- [x] 7.6 Add unit and CLI tests for multiple semantic and package-ID collision groups, byte-identical duplicates, deterministic diagnostics across input permutations, same-root distinct snapshots, and absence of outer identity from capability and lowering findings.

Acceptance evidence: aggregation tests report all stable collision groups in one failure and prove no backend or publication test double is called after a collision.

## 8. Preserve ordered backend resolution and atomic multi-target publication

- [x] 8.1 Update the library compilation coordinator and `src/cli/commands/transform.rs` so backend registry resolution, capability analysis, and lowering begin only after discovery, decoder selection, aggregation, and collision validation complete.
- [x] 8.2 Normalize and deduplicate repeated target spellings with the existing target-resolution rules, then require one or more unique resolved targets.
- [x] 8.3 Lower each unique target into an independent typed artifact set and pass the full set unchanged to the existing staged-publication transaction boundary.
- [x] 8.4 Keep `check` non-mutating and retain existing all-or-nothing stage and apply behavior across every unique target artifact set.
- [x] 8.5 Add transform and staged-publication integration tests for repeated target spellings, multiple unique targets, pre-lowering collision failure, check drift without destination import, multi-target stage preflight failure, and apply rollback after an injected later-target mutation failure.

Acceptance evidence: integration tests prove duplicate target spellings lower once and failures before or during publication leave no partial stage or live multi-target publication set.

The later-target apply failure test uses a deliberately crate-private, test-only fault injector rather than adding a production failure-injection API; it exercises transaction rollback without expanding the public publication surface.

## 9. Document the explicit snapshot-build boundary

- [ ] 9.1 Update `docs/2026-04-11-prd.md`, `docs/2026-04-11-man-page.md`, and `docs/cli/rulette.md` to describe explicit homogeneous snapshot inputs, constrained auto detection, accepted stdin forms, shared limits, one-or-more unique targets, and check-stage-apply direction.
- [ ] 9.2 Update `src/bin/gen_docs.rs` and regenerate only tracked project CLI reference files when their source-of-truth command descriptions require new `--from`, stdin, or target-cardinality text, without invoking Beads, OpenSpec, or another external generator.
- [ ] 9.3 Document that Beads, OpenSpec, and other generators run outside Rulette, that graph JSON and TOML are explicit interchange inputs only, and that outputs are derived artifacts never imported or merged.
- [ ] 9.4 Document collision diagnostics as displaying package provenance plus transient outer input identity, while capability and lowering findings display only package provenance.
- [ ] 9.5 Add documentation assertions or CLI help snapshots covering rejected plain native stdin, ambiguous auto detection, and deduplicated targets.

Acceptance evidence: generated CLI documentation and user-facing references agree with the library behavior and contain no command, plugin, runtime-loading, reverse-sync, or initialization workflow.

## 10. Validate the generated snapshot build change

- [ ] 10.1 Run focused input, parser, aggregation, graph-interchange, transform CLI, and staged-publication tests while each implementation task lands.
- [ ] 10.2 Run `mise run fmt`, `mise run lint`, `mise run test`, and `mise run spec-validate` after the complete change is implemented.
- [ ] 10.3 Run `mise run build` and perform static-link binary and dependency inspection, without release packaging or publication, to confirm the implementation adds no runtime-loaded code, generator execution, network access, local state, configuration discovery, or direct destination publication path.
- [ ] 10.4 Record any unimplemented cross-cutting requirement as a follow-up Bead before closing this implementation change.

Acceptance evidence: the full validation suite passes and focused tests cover every scenario in `generated-snapshot-builds` and its `transform-pipeline` delta.
