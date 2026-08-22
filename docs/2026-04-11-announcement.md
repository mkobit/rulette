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

The public library centers on `CompilationGraph`, exact selection, capability analysis, lowering plans, and staged publication.
The CLI remains an adapter over those contracts.

This release boundary deliberately excludes text rewriting, metadata mutation, agent portability, hook portability, MCP portability, package registries, and runtime execution.
Those behaviors need explicit semantics and validation before they can join the compiler.
