# Rulette 0.1 compiler model

Rulette is moving from a text-and-entity converter to a package-aware compiler for local agent guidance.

The v0.1 core recognizes Codex, Claude, Cursor, OpenCode, and Antigravity layouts.
It represents their supported rules and skills as deterministic graph packages.
Each retained package keeps its source provenance, normalized root, primary instruction, opaque resources, and executable metadata.

This model makes a skill directory a real compilation unit instead of reducing it to one body string.
It also makes nonportable semantics explicit.
Agents, hooks, MCP settings, permissions, and native configuration are retained as opaque unsupported packages with diagnostics instead of being silently flattened into portable instructions.

The compiler has one safe path.
It compiles explicit local inputs, selects exact package identifiers, lowers target-relative artifacts, reports loss, and stages the result for review.
It does not fetch content, execute plugins, discover configuration, retain state, or publish directly to arbitrary native paths.

Rulette v0.1.0 ships `rulette-v0.1.0-x86_64-unknown-linux-musl.tar.gz` as a fully static Linux binary with no runtime dependencies.
Its `.sha256` sidecar supports download verification before extraction.
The narrow portability promise covers rules, skills, primary instructions, and safe package resources across the five core harnesses.
It deliberately excludes agents, hooks, MCP settings, permissions, plugins, registries, and fetch behavior.

Native direct-output migration becomes a stage, review, and explicitly authorized apply workflow.
Lowering remains strict by default, while `--allow-lossy` records a maintainer’s deliberate acceptance of representational loss before staging.
The command reference defines the stage, digest, and root-authorization arguments.

The public library centers on `CompilationGraph`, exact selection, capability analysis, lowering plans, and staged publication.
The CLI remains an adapter over those contracts.

This release boundary deliberately excludes text rewriting, metadata mutation, agent portability, hook portability, MCP portability, package registries, and runtime execution.
Those behaviors need explicit semantics and validation before they can join the compiler.
