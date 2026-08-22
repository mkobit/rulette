## Why

Rulette currently flattens native harness layouts into a unified document whose broad entity model cannot preserve package boundaries, primary instruction files, opaque resources, or complete source provenance.
That model implies semantic portability for configuration surfaces that the v0.1 compiler cannot translate safely across targets.
Rulette v0.1 instead needs a narrow compilation contract that deterministically translates native projects among Codex, OpenCode, Claude, Cursor, and Antigravity without inventing a portable runtime or silently discarding source material.

## What changes

- Define Rulette v0.1 as a static, deterministic, native-to-native compiler for Codex, OpenCode, Claude, Cursor, and Antigravity.
- Replace the flattened intermediate document contract with a versioned, package-aware compilation graph.
- Preserve source provenance, rule and skill entities, package boundaries, primary instructions, and safe opaque resources in the graph.
- Represent each opaque resource with a normalized relative path, exact bytes, and executable metadata.
- Reject symbolic links, archive hard-link entries, absolute paths, parent traversal, duplicate normalized paths, and any other path that cannot be normalized safely within its package boundary.
- Make each frontend responsible for recognizing and interpreting its harness-domain layout before constructing the target-independent graph.
- Keep target layout decisions in frontends and backends rather than encoding harness-specific directory conventions in the graph kernel.
- Expose selection as the only public transform over the graph for v0.1.
- Treat semantic rewrites such as rename, set, merge, deduplication, and arbitrary graph mutation as outside the v0.1 public transform contract.
- Detect representational loss before emission and fail compilation by default.
- Add `--allow-lossy` as the explicit opt-in for compilation that drops or degrades graph content.
- Keep agents, hooks, MCP servers, permissions, and a plugin ABI outside the v0.1 portability contract.

## Scope

- Compile native inputs from individual files, standard input, directories, and tar archives through one in-memory compilation graph.
- Preserve the identity and provenance needed to diagnose every selected graph item back to its source package and source path.
- Preserve native primary instructions, portable rule and skill semantics, and safe unmodeled files needed to reconstruct package contents.
- Emit deterministic native layouts for the five v0.1 harnesses without network access, initialization, local state, or auto-discovered configuration.
- Validate the complete compilation plan before writing any output so existing all-or-nothing emission guarantees remain intact.
- Reject non-regular archive entries and input collections that exceed the documented resource-count or byte budgets before materializing package content.

## Non-goals

- Rulette v0.1 does not promise semantic translation of agents, hooks, MCP servers, or permissions between harnesses.
- Safe files associated with unsupported features may survive as opaque resources, but their presence does not create a portability guarantee.
- Rulette v0.1 does not provide a public plugin ABI or dynamically load third-party frontends, backends, or transforms.
- Rulette v0.1 does not provide public mutation transforms beyond selection.
- Rulette v0.1 does not define remote registries, dependency resolution, mutable fetches, daemon behavior, initialization, or persisted local configuration.

## Capabilities

### New capabilities

- `compilation-graph-kernel`: Define the versioned package-aware graph, provenance model, portable rule and skill entities, primary instructions, safe opaque resources, path safety rules, and deterministic graph invariants.

### Modified capabilities

- `ir-core`: Replace the unified `RuletteDocument` envelope and broad entity-kind portability promise with the versioned compilation graph and its package-scoped identities.
- `transform-pipeline`: Limit the public transform surface to selection, make loss an error by default, add `--allow-lossy`, and preserve validation-before-write and atomic multi-target behavior.
- `frontends-and-backends`: Limit the supported v0.1 native formats to Codex, OpenCode, Claude, Cursor, and Antigravity, assign native layout ownership to format adapters, and require loss reporting against graph content before emission.

## Impact

- The library boundary changes from a flat document transformation API to a graph compilation API with inward dependencies that do not reference CLI, filesystem, archive, or harness-specific types.
- Frontends translate filesystem and archive observations into validated graph packages, while backends translate selected graph packages into native target layouts.
- The CLI remains a thin wrapper over library compilation and exposes selection plus the explicit `--allow-lossy` policy switch.
- Existing public rename, set, exclude-as-mutation, merge, deduplication, broad entity normalization, and permissive loss behavior require removal or compatibility decisions during design.
- Existing JSON IR and schema surfaces require an explicit compatibility decision because graph schema versioning replaces the current `RuletteDocument` contract.
- Documentation and release notes must replace claims of general configuration portability with the narrower v0.1 rule, skill, primary-instruction, and safe-resource contract.

## Test and release rationale

- Golden fixtures for every supported frontend SHALL verify graph package boundaries, provenance, primary instructions, rule and skill entities, opaque resource bytes, normalized paths, and executable metadata.
- Safety fixtures for directories and tar archives SHALL verify rejection of links, absolute paths, traversal paths, and package-boundary escapes before compilation produces output.
- Cross-target fixtures SHALL verify deterministic output and identify every degradation that strict mode rejects or `--allow-lossy` permits.
- Selection tests SHALL verify that selection is the only public graph transform and cannot detach retained resources from their package or provenance invariants.
- Repeated-run tests SHALL verify byte-identical output for identical inputs, options, and compiler versions.
- Release validation SHALL verify that the distributed binary is fully static and executes without runtime dependencies or network access.
- This contract reset belongs in v0.1 because the project is pre-stable and narrowing the public surface now avoids preserving an unsafe or misleading compatibility promise after release.
- The release SHALL remain blocked until all five native frontends and backends pass the same graph conformance, path safety, deterministic compilation, and strict-loss suites.
