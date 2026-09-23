# Rulette and RuleSync

Rulette and RuleSync both facilitate synchronizing AI assistant guidance across multiple tools.
However, they take fundamentally different architectural approaches to configuration, execution, and publication safety.

RuleSync is a generator that auto-discovers configuration and directly writes generated files into repository locations.
Rulette is a stateless static compiler that transforms explicit input artifacts into an intermediate representation and publishes through a reviewed two-phase staging workflow.

## Comparison

| Dimension | RuleSync | Rulette 0.1 |
| --- | --- | --- |
| Execution model | Dynamic generator and file syncer | Pure compiler kernel with two-phase publication |
| Configuration | Implicit discovery of `.rulesync/rulesync.jsonc` | Explicit `--config <path>` artifact; never auto-discovered |
| State and init | `rulesync init` scaffolds stateful project directory | Stateless; no initialization step, no runtime dependencies |
| Target write authority | Direct in-place mutation of live files | Two-phase publication (`--stage` plan review then `--apply`) |
| Verification | Overwrites destinations without pre-commit audit | Cryptographic SHA-256 digest checks and `--check` drift validation |
| Representation loss | Silent dropping or best-effort translation | Strict failure by default on capability loss; `--allow-lossy` required |
| Target overrides | Inline untyped blocks (`cursor`, `antigravity`) | Typed `rulette:activation` default and per-target overrides |
| Unsupported features | Unrecognized keys passed through or dropped | Retained as opaque unsupported packages with provenance |

## Architecture

### 1. Stateless compilation versus stateful synchronization

RuleSync relies on discovering `.rulesync/` in the current working directory and directly updating target rule files.
This introduces hidden state and makes build steps dependent on ambient directory context.

Rulette rejects ambient configuration files and initialization phases.
It operates purely on explicit inputs (files, directories, standard input, or tar archives).
When declarative manifests are needed, they are passed explicitly via `--config <path>` as input artifacts.

### 2. Reviewed staging versus direct file mutation

Running `rulesync generate` mutates project directories in place, risking silent overwrites of manual edits.

Rulette separates plan generation from file mutation:

1. `rulette transform --stage <dir>` compiles the graph, verifies capability mappings, and writes a self-contained publication plan (`rulette.plan.json`).
2. The user or CI audits the plan and verifies its SHA-256 digest.
3. `rulette transform --apply <plan> --expect-plan-sha256 <digest>` publishes artifacts only with explicit root authorization.

### 3. Capability reporting versus silent loss

When converting complex rules to harnesses that do not support certain features, RuleSync drops unmapped properties silently.

Rulette enforces strict capability checking:

- Every package and resource is evaluated as Supported, Lossy, or Dropped across all target harnesses.
- Any representation loss immediately halts staging unless `--allow-lossy` is explicitly supplied.
- Running `rulette inspect <input> --coverage` prints a full capability matrix across all core targets.

## Migration from RuleSync

Migrating a repository from RuleSync to Rulette involves three steps.

### 1. Translate the configuration manifest

Convert `.rulesync/rulesync.jsonc` into a declarative Rulette transform manifest (`rulette.transform.jsonc`):

```jsonc
{
  // Explicit inputs to compile
  "inputs": ["./rules/"],

  // Target harnesses and publication scope
  "targets": [
    { "target": "cursor", "scope": "project" },
    { "target": "claude", "scope": "project" },
    { "target": "codex", "scope": "project" },
    { "target": "opencode", "scope": "project" },
    { "target": "antigravity", "scope": "project" }
  ]
}
```

### 2. Map per-tool blocks to typed activation overrides

RuleSync embed per-tool options as unvalidated sub-blocks:

```yaml
---
description: TypeScript conventions
globs: ["**/*.ts", "**/*.tsx"]
cursor:
  alwaysApply: true
antigravity:
  trigger: model_decision
  description: Apply when editing or reviewing TypeScript files
---
```

In Rulette, translate these into the typed `rulette:activation` schema:

```yaml
---
description: TypeScript conventions
rulette:activation:
  default:
    mode: [glob]
    globs: ["**/*.ts", "**/*.tsx"]
  overrides:
    cursor:
      mode: [always]
    antigravity:
      mode: [model]
      description: Apply when editing or reviewing TypeScript files
---
```

The compiler resolves each target from its explicit override, falling back to `default` for targets without overrides.

### 3. Stage and apply the migration

Validate the migrated rules using Rulette's staged publication:

```sh
# 1. Stage the publication plan
rulette transform \
  --config rulette.transform.jsonc \
  --stage /tmp/migration-stage \
  --project-root .

# 2. Check for drift against existing files
rulette transform \
  --apply /tmp/migration-stage/rulette.plan.json \
  --expect-plan-sha256 <DIGEST> \
  --allow-project-root . \
  --check

# 3. Apply the verified plan
rulette transform \
  --apply /tmp/migration-stage/rulette.plan.json \
  --expect-plan-sha256 <DIGEST> \
  --allow-project-root .
```
