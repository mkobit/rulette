# Journey 2: continuous integration gates

This example demonstrates Journey 2 from the CLI architecture design: enforcing rule synchronization and capability compliance in automated CI pipelines.

Rulette provides two independent quality gates for continuous integration:

- **Drift gate:** verifies that generated tool configuration files in the working tree are up to date with canonical rules.
- **Capability gate:** asserts that all canonical rules and packages can be faithfully represented by target harnesses without unaccepted loss.

## Source layout

```text
examples/journey-2-ci-gates/
├── rules/
│   └── safety.md
├── rulette.transform.jsonc
└── README.md
```

## CI gate execution

### Gate 1: Drift check

The drift gate runs in check mode without writing to disk:

```sh
rulette transform \
  --config rulette.transform.jsonc \
  --from antigravity \
  --check \
  --project-root .
```

Behavior in CI:

- Exits `0` if all target files exist, match canonical output exactly, and have no out-of-band drift.
- Exits non-zero (`1`) and reports `absent`, `outdated`, or `conflicting` for any destination that requires updating.

### Gate 2: Strict capability coverage gate

The capability gate evaluates the compilation graph against core harness capabilities:

```sh
rulette inspect ./rules/ \
  --from antigravity \
  --coverage \
  --strict
```

Behavior in CI:

- Exits `0` if every rule or skill package is classified as `supported` across all evaluated targets.
- Exits non-zero (`1`) if any entity produces a `lossy` or `dropped` finding, preventing accidental degradation of rule instructions.

## Example CI workflow step

In GitHub Actions or another runner, include both gates in your check job:

```yaml
- name: Verify AI rules drift
  run: rulette transform --config rulette.transform.jsonc --from antigravity --check --project-root .

- name: Enforce strict capability coverage
  run: rulette inspect ./rules/ --from antigravity --coverage --strict
```
