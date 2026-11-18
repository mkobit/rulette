# Upgrade Bun to 1.4.4

* **Author:** Mike Kobit
* **Status:** Proposed
* **Date:** 2026-11-13

--------------------------------------------------------------------------------

## Overview

This design upgrades `bun` from `1.4.3` to `1.4.4` in `rulette`.
It covers local developer hosts, GitHub Actions CI runners, and `.sbx` Docker sandboxes while keeping `mise.toml` and `mise.lock` as the single version authority.

## The problem

`mise.toml` currently pins `bun = "1.4.3"` with `[settings] locked = true`, and `mise.lock` stores checksums for all 11 supported OS and architecture platforms.
Two issues affect this upgrade:

1. **Multi-platform lockfile drift**: Updating `mise.toml` on a single host without regenerating all 11 platform entries in `mise.lock` via `mise lock` breaks `jdx/mise-action` on non-host CI runners (`ubuntu-latest`, `macos-15`, `windows-latest`).
2. **Cached `.sbx` container images**: [The SBX kit spec](../../.sbx/kit/spec.yaml) installs `mise` tools during image build while sharing the host checkout (`clone: false`). Cached `.sbx` environments built with `1.4.3` will fail `mise` shim resolution or keep `1.4.3` after `mise.toml` updates to `1.4.4` unless rebuilt or updated with `mise install`.

In addition, Beads issue `rulette-6gj` originally proposed centralizing CI `bun` version declarations in `package.json`.
Because CI workflows already delegate installation to `./.github/actions/setup-mise`, adding a `bun` version field to `package.json` would create a second version source and risk drift.

## Goals

* Update `bun` from `1.4.3` to `1.4.4` in `mise.toml` and regenerate the `bun` entries in `mise.lock` on the host via `mise lock bun`.
* Invalidate or rebuild cached `.sbx` kit environments so sandboxes provision `bun` `1.4.4` via `mise`.
* Verify compatibility across local host (`mise run check`, `bun --version` equal to `1.4.4`, unchanged `bun.lock`), `.sbx` (`mise run validate-sbx`, `sbx env exec .sbx/sbxenv.yaml -- mise run check`, `sbx env exec .sbx/sbxenv.agy.yaml -- mise run check`), and CI (`lint-markdown`, `check`, `test`, `audit`).

## Non-goals

* Adding `packageManager`, `engines`, or any `bun` version field to `package.json` (superseding the original `rulette-6gj` wording).
* Upgrading JS `devDependencies` (`@fission-ai/openspec`, `markdownlint-cli2`) or `overrides` in `package.json`.
* Adding new `mise.toml` tasks such as `[tasks.audit]` or modifying `.sbx/kit/check-runtime.sh` with custom `mise.toml` shell parsing.
* Modifying CI workflows (`.github/workflows/`) or `./.github/actions/setup-mise` unless a compatibility failure requires it.

## Open questions

None.

## Critical user journeys (CUJs)

### CUJ 1: Developer upgrades or verifies Bun locally

* **Given:** A developer has `mise` installed and pulls the updated `mise.toml` and `mise.lock`.
* **When:** The developer runs `mise install` and `mise run check`.
* **Then:** `bun --version` outputs `1.4.4`, `bun.lock` remains unchanged, and all checks (`install-tools`, `spec-validate`, `lint:md`, `fmt`, `lint`, `test`) pass.

### CUJ 2: CI runner installs Bun across platforms

* **Given:** `.github/workflows/ci.yml` runs on `ubuntu-latest`, `macos-15`, and `windows-latest` using `./.github/actions/setup-mise`.
* **When:** Each runner executes `jdx/mise-action` with `locked = true`.
* **Then:** `mise` resolves the matching platform entry from `mise.lock`, installs `bun` `1.4.4`, and passes `lint-markdown`, `check`, `test`, and `audit`.

### CUJ 3: Sandbox provisions Bun via Mise

* **Given:** [The SBX kit spec](../../.sbx/kit/spec.yaml) builds a `.sbx` container image from the updated `mise.toml` and `mise.lock` (see [the SBX workflow](../2026-10-15-sbx-delegation.md)).
* **When:** An operator runs `mise run validate-sbx` or `sbx env exec .sbx/sbxenv.yaml -- mise run check`.
* **Then:** `mise` inside the sandbox provides `bun` `1.4.4` and all checks pass.

## Technical plan (architecture)

```text
                    +-------------------------------+
                    |   mise.toml (bun = "1.4.4")   |
                    |   mise.lock (11 platforms)    |
                    +---------------+---------------+
                                    |
          +-------------------------+-------------------------+
          |                         |                         |
          v                         v                         v
+-------------------+     +-------------------+     +-------------------+
|   Local host      |     |   GitHub CI       |     |   .sbx sandbox    |
| mise install      |     | setup-mise action |     | image: mise       |
| mise run check    |     | jdx/mise-action   |     | install at build  |
+-------------------+     | locked = true     |     | (or mise activate)|
                          +-------------------+     +-------------------+
```

### Component changes

1. **Version pin and lockfile (`mise.toml`, `mise.lock`)**
    * Update `bun = "1.4.3"` to `bun = "1.4.4"` under `[tools]` in `mise.toml`.
    * Run `mise lock bun` on the host so `mise` regenerates the `version = "1.4.4"` URLs and checksums for all 11 platforms in `mise.lock` without modifying `github:gastownhall/beads` or `rust`.
2. **Sandbox kit version (`.sbx/kit/spec.yaml`)**
    * Bump `version` in `.sbx/kit/spec.yaml` (from `"0.2.0"` to `"0.2.1"`) if needed to invalidate cached `.sbx` kit images, and keep `mise` (`mise install` / `mise activate`) as the sole tool-provisioning mechanism in the sandbox.

### Unchanged components

* `package.json` and `bun.lock`: No `engines`, `packageManager`, or dependency changes.
* `.sbx/kit/check-runtime.sh`: Remains unchanged; `mise` shims and `mise` provisioning handle tool resolution.
* `.github/workflows/ci.yml`, `.github/workflows/openspec.yml`, `.github/workflows/release.yml`, and `./.github/actions/setup-mise`: No changes required; they already consume `mise.toml` and `mise.lock`.

## Alternatives considered

### Alternative A: Declare `bun` version in `package.json` and read it in CI

* **Pros:** Follows JavaScript ecosystem conventions (`engines.bun` or `packageManager`) and surfaces the expected version to tools that inspect `package.json` directly.
* **Cons:** `mise` does not read `package.json` for core tool pinning, and CI already delegates `bun` installation to `./.github/actions/setup-mise`. Adding a `bun` version field to `package.json` creates a second declaration that can drift from `mise.toml` and `mise.lock`.
* **Reason for rejection:** `mise.toml` and `mise.lock` already serve as the single version source across local, CI, and `.sbx` environments; duplicating the pin in `package.json` increases drift risk without simplifying CI.

## Non-functional requirements

* **Security and privacy:** No new network endpoints or credentials are introduced; `mise lock bun` and `mise install` use the existing `mise` and GitHub/Bun release checksums.
* **Performance and latency:** `mise lock bun` runs once on the host during the upgrade; runtime execution has no added overhead.
* **Rollout and rollback:**
  * **Rollout:** Update `mise.toml`, run `mise lock bun` on the host, bump `.sbx/kit/spec.yaml` `version` if cached layers persist, verify locally and in `.sbx`, and recreate any cached `.sbx` environments built on `1.4.3`.
  * **Rollback:** Revert `mise.toml`, `mise.lock`, and `.sbx/kit/spec.yaml` (plus `bun.lock` if modified), run `mise install` on the host, recreate `.sbx` environments built for `1.4.4`, and re-run `mise run check`.

--------------------------------------------------------------------------------

## Detailed implementation plan

### Target files to modify

1. **`mise.toml`**
    * **Type / scope:** Toolchain configuration manifest.
    * **Proposed changes:** Update `bun = "1.4.3"` to `bun = "1.4.4"` under `[tools]`.
2. **`mise.lock`**
    * **Type / scope:** Auto-generated multi-platform tool lockfile.
    * **Proposed changes:** Run `mise lock bun` on the host to regenerate the `[tools.bun]` section for `1.4.4` across all 11 platforms. Verify via `git diff mise.lock` that `github:gastownhall/beads` and `rust` entries are unchanged.
3. **`.sbx/kit/spec.yaml`**
    * **Type / scope:** Sandbox kit specification.
    * **Proposed changes:** Bump `version` (`"0.2.0"` to `"0.2.1"`) so cached `.sbx` kit images rebuild with `bun` `1.4.4` via `mise install` (using `mise activate` where shell activation is needed).

### Operational updates

1. **Host and sandbox refresh:** Run `mise install` on the host. Recreate active `.sbx` environments on the host (for example, `sbx env create --name <name> .sbx/sbxenv.yaml`) or run `mise install` inside an existing sandbox so it has `bun` `1.4.4`.
2. **Beads notes:** Update Beads issue `rulette-6gj` (via `bd update` / `bd comments`) to record that `mise.toml` and `mise.lock` remain the sole `bun` version authority and `package.json` stays dependency-only.

### Verification plan

* **Local runtime:** Run `mise install` and verify `bun --version` outputs `1.4.4`.
* **Local quality gates:** Run `mise run check` (covers `install-tools`, `spec-validate` for OpenSpec specs, `lint:md`, `fmt`, `lint`, and `test`).
* **Lockfile integrity:** Verify `git diff -- bun.lock` is empty and `git diff mise.lock` updates only `[tools.bun]` to `1.4.4`.
* **Sandbox verification:** Run `mise run validate-sbx` (checks `.sbx/sbxenv.yaml` and `.sbx/sbxenv.agy.yaml`) and run `sbx env exec .sbx/sbxenv.yaml -- mise run check` and `sbx env exec .sbx/sbxenv.agy.yaml -- mise run check`.
* **CI matrix:** Confirm `.github/workflows/ci.yml` passes `lint-markdown`, `check`, `test` (`ubuntu-latest`, `macos-15`, `windows-latest`), and `audit`.
