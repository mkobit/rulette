# Rulette and Agent Skills

Rulette and Agent Skills address adjacent parts of the agent-guidance ecosystem.

Agent Skills defines a portable skill-package convention centered on `SKILL.md` and its frontmatter.
Rulette is a static compiler that preserves rule and skill packages from five documented harness domains in a deterministic compilation graph.

## Comparison

| Dimension | Agent Skills | Rulette 0.1 |
| --- | --- | --- |
| Primary concern | Skill-package convention and ecosystem | Package-aware compilation and reviewed publication |
| Semantic unit | Skill directory | Rule or skill package |
| Runtime role | Format and distribution tooling | Static local compiler |
| Input acquisition | Tooling-defined | Explicit local files, standard input, directories, and safe tar archives |
| Native semantics | Outside the skill format | Retained as opaque unsupported packages with diagnostics |
| Portability policy | Format-defined | Portable rules and skills only |
| Output authority | Tooling-defined | Deterministic lowering followed by staged review and authorized apply |

## Relationship

Rulette uses the Agent Skills skill-name grammar for portable skill identities.
It does not implement an Agent Skills registry, network retrieval, lockfiles, mutable-reference resolution, or installation workflow in v0.1.

A source harness may expose a native skill directory that Rulette recognizes as a graph skill package.
The compiler retains that package’s primary instruction and opaque resources instead of assuming that all harness-specific fields have portable meaning.

## Boundaries

Rulette is not a substitute for skill discovery, content hosting, or agent execution.
Agent Skills does not define the graph validation, exact package selection, cross-domain loss reporting, or staged-publication contracts that Rulette provides.
