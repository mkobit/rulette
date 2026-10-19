# Rulette v0.1.0 design research: decisions D10 and D11 (`rulette-6lj`)

## Executive summary

Rulette provides a deterministic, offline, strict-by-default compiler for AI rules and skills with explicit staged publication.
Two current behaviors in the codebase create friction and surprise developers who migrate or share skills across harnesses:

1. **D10 (Skill companions and executable metadata):**
   [`lower_skill_package`](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L322-L406) gates companion resource emission on matching frontends.
   It sets `executable: false` on lowered artifacts and reports `ExecutableBitUnrepresentable`.
   However, the staging and transaction layers in [`src/publication/stage.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/publication/stage.rs#L216) and [`src/publication/transaction.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/publication/transaction.rs#L143) already verify and set `0755` permissions.
2. **D11 (Skills-layout interoperability):**
   The ecosystem has converged on `.agents/skills/<name>/SKILL.md` as a shared project skill layout.
   [`native_frontend_candidates`](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/frontend.rs#L370-L398) maps `.agents` exclusively to `NativeFrontend::Antigravity`.
   [`src/parsers/codex.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/codex.rs#L266-L270) only searches `.codex/skills/`.
   Passing `.agents/skills` to `--from codex` fails preflight validation.
   Passing a repository with both `AGENTS.md` and `.agents/skills` under `--from auto` fails with an ambiguous multi-frontend error.

Below is the architectural analysis, recommended decisions, and the v0.1.0 acceptance matrix.

---

## Decision D10: skill companions and executable metadata

### D10 current implementation audit

- **Graph and input layer:**
  [`inputs/mod.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/inputs/mod.rs#L490-L500) reads the filesystem executable bit into `ArtifactObservation::executable`.
  Tar archives in [`observe_archive`](file:///home/mkobit/workspace/mkobit/rulette/src/inputs/mod.rs#L335-L342) hardcode `false` and should read `(entry.header().mode()? & 0o111) != 0`.
  `CompilationGraph` in [`src/ir/graph.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/ir/graph.rs) hashes package identity over ordered resource paths, content digests, and the `executable` boolean flag.
- **Lowering layer:**
  In [`lower_skill_package`](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L370-L404), companion files are only emitted when `package.provenance.frontend == target.as_str()`.
  Across domains, companion files drop with `CapabilityReasonCode::OpaqueCrossDomain` (`Severity::Dropped`).
  Even for same-domain lowering, `lower_skill_package` sets `executable: false` on `NativeArtifact`.
  Lines 241–273 emit `CapabilityReasonCode::ExecutableBitUnrepresentable` (`Severity::Lossy`) for every executable resource.
  For `NativeTarget::Cursor`, [`lower`](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L177-L212) lowers a `Skill` to `.cursor/rules/<name>.mdc` with `SkillLoweredAsRule` (`Lossy`) and drops all companions with `OpaqueResourceUnrepresentable` (`Dropped`).
- **Publication and mapping layer:**
  `Codex`, `OpenCode`, `Claude`, `Antigravity`, and `AgentPlugin` declare explicit `NativeArtifactClass::SkillInstruction` and `NativeArtifactClass::SkillResource` prefix mappings in [`src/publication/mapping.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/publication/mapping.rs).
  [`PlanEntry`](file:///home/mkobit/workspace/mkobit/rulette/src/publication/stage.rs#L216) and [`write_same_directory_temporary`](file:///home/mkobit/workspace/mkobit/rulette/src/publication/transaction.rs#L140-L144) already verify and set `0o755` or `0o644` permissions on disk.

### D10 lens analysis

1. **UX and simplicity:**
   A developer authors a skill in `.claude/skills/release/` with `SKILL.md`, `scripts/check.sh` (`0755`), and `templates/notes.md`.
   Running `rulette transform --target codex@project` fails by default today because `check.sh` and `templates/notes.md` are marked cross-domain opaque resources.
   Both Claude and Codex skills use the same directory structure.
   Dropping companion files across domains is unexpected.
2. **Ease of understanding:**
   The mental model is simple: a skill is a self-contained directory tree rooted at `<skill-name>/`.
   It contains a portable `SKILL.md` entry point and relative companion files with preserved executable permissions.
   When a target supports native skill directories (`codex`, `claude`, `opencode`, `antigravity`, `agent-plugin`), the companion tree maps directly.
   When a target does not support skill directories (such as flat rules in `cursor`), companions drop with explicit `Dropped` or `Lossy` findings.
3. **Configuration on top:**
   Strict mode (`allow_lossy = false`) remains the default.
   Standard skill migrations succeed cleanly under strict mode across skill-directory targets.
   When lowering to a flat rule target or when frontmatter keys are dropped, strict mode blocks staging unless `--allow-lossy` or `allow_lossy = true` in `--config` is supplied.
4. **Determinism and strict safety:**
   All companion paths are validated as normalized relative paths.
   Preserving `executable: resource.executable` on `SkillResource` artifacts makes filesystem and tar snapshots deterministic.

### D10 recommendation

Preserve relative skill companion files and `0755` executable metadata across all skill-directory-capable targets as `Supported` (`Representable`).
Reserve `Dropped` (`OpaqueResourceUnrepresentable`) and `Lossy` (`ExecutableBitUnrepresentable` / `SkillLoweredAsRule`) findings for targets that cannot represent skill directories or executable bits.
Emit a `Lossy` finding (`SkillFrontmatterUnrepresentable`) when unmapped `FrontendPayload` keys in `SKILL.md` frontmatter drop during cross-domain lowering.

#### D10 implementation mechanics

1. In [`lower_skill_package`](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L370-L404), remove the `package.provenance.frontend == target.as_str()` restriction for `SkillResource` artifacts.
   Emit non-primary resources as `NativeArtifactClass::SkillResource` at `skills/{name}/{resource.path}` with `CapabilitySeverity::Supported`.
2. For `NativeArtifactClass::SkillResource`, set `executable: resource.executable` on the lowered `NativeArtifact`.
   Do not emit `ExecutableBitUnrepresentable` when the target preserves the resource with `executable: true`.
   For markdown files (`AGENTS.md`, `CLAUDE.md`, `*.mdc`, `SKILL.md`), keep `executable: false`.
   If a markdown instruction file has `+x` set, emit `ExecutableBitUnrepresentable` (`Lossy`).
   In [`observe_archive`](file:///home/mkobit/workspace/mkobit/rulette/src/inputs/mod.rs#L335-L342), extract `(entry.header().mode()? & 0o111) != 0` instead of hardcoding `false`.
3. Preserve unmapped frontmatter when lowering to the same frontend, and report `Lossy` if dropped across domains.

#### D10 rationale

- Matches developer expectations and the Agent Skills standard (`SKILL.md` + relative companion files).
- Eliminates false-positive loss findings on executable bits.
- Keeps strict safety intact without traversal outside `skills/<name>/`.

#### D10 alternatives considered

- **Drop companions across domains:**
  Forces `--allow-lossy` on every cross-harness skill compilation and strips executable permissions from scripts.
- **Allowlist specific subdirectories:**
  Fails on custom skill layouts such as `templates/` or top-level schemas.

---

## Decision D11: skills-layout interoperability

### D11 current implementation audit

1. **Auto-detection in [`src/parsers/frontend.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/frontend.rs#L370-L398):**
   `has_path_component(path, ".agents")` adds `NativeFrontend::Antigravity` to the candidate set.
   When a repository has both `AGENTS.md` and `.agents/skills/my-skill/SKILL.md`, running `rulette transform . --from auto` fails with multiple frontend candidates.
2. **Explicit selection:**
   Running `rulette transform . --from codex` causes [`validate_native_observations`](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/frontend.rs#L250-L264) to fail because `.agents/skills/...` is marked as `antigravity`.
3. **Parser path matching:**
   [`codex::skill_root`](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/codex.rs#L266-L270) only matches `.codex/skills/`.

### D11 lens analysis

1. **UX and simplicity:**
   In the wider ecosystem, `.agents/skills/<name>/SKILL.md` is the standard shared project directory for skills.
   Requiring developers to copy `.agents/skills` to `.codex/skills` adds unnecessary friction.
2. **Ease of understanding:**
   The mental model treats `.agents/skills/<name>/SKILL.md` as a shared skill layout recognized by all skill-capable frontends.
   `.antigravity/`, `.agent/`, and `.agents/rules/` remain exclusive to `antigravity`.
3. **Configuration on top:**
   `--from auto` resolves to `codex` when `AGENTS.md` and `.agents/skills/...` coexist.
   `--from auto` resolves to `cursor-mdc` when `.cursor/rules/...` and `.agents/skills/...` coexist.
   Passing `--from codex` explicitly compiles `.agents/skills/...` cleanly.
4. **Determinism and strict safety:**
   No ambient filesystem discovery is added; Rulette only reads explicit input paths.
   Colliding skills across `.codex/skills/foo` and `.agents/skills/foo` fail deterministically in aggregation.

### D11 recommendation

Treat `.agents/skills/<name>/SKILL.md` and direct `skills/<name>/SKILL.md` snapshots as a shared skill layout across native frontends.

#### D11 implementation mechanics

1. In [`src/parsers/frontend.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/frontend.rs#L370-L398), classify `.agents/skills/` as a shared skill path rather than an exclusive `Antigravity` marker.
   Allow observations under `.agents/skills/...` to decode under any skill-capable frontend in `validate_native_observations`.
   In `detect_graph_format`, resolve to the specific harness frontend when unambiguous markers coexist with `.agents/skills/`.
2. Update `codex::skill_root` and `claude::skill_root` to recognize `.agents/skills/<name>/<member>` and `skills/<name>/<member>`.
   Support direct directory arguments where relative paths are `<name>/SKILL.md` or `SKILL.md`.

#### D11 rationale

- Resolves compatibility with the modern skills ecosystem without manual file copying.
- Preserves explicit provenance and deterministic collision detection.

#### D11 alternatives considered

- **Prescribe caller preparation:**
  Forces manual copying and causes failure when rules and skills coexist.
- **Add a separate `--from agent-skills` frontend:**
  Breaks unified compilation for projects with both native rules and shared skills.

---

## Proposed v0.1.0 acceptance cases

| ID | Input layout | Content | Target | Expected outcome | Emitted artifacts and findings |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A01** | `codex` (`.codex/skills/deploy/`) | `SKILL.md` + `run.sh` (`0755`) | `codex@project` | Supported (strict pass) | `.codex/skills/deploy/` preserved with `0755`. |
| **A02** | `codex` (`.codex/skills/deploy/`) | `SKILL.md` + `run.sh` (`0755`) | `claude@project` | Supported (strict pass) | `.claude/skills/deploy/` preserved with `0755`. |
| **A03** | `claude` (`.claude/skills/deploy/`) | `SKILL.md` + `run.sh` (`0755`) | `codex@user` | Supported (strict pass) | `skills/deploy/` emitted to user root with `0755`. |
| **A04** | `codex` (`.codex/skills/deploy/`) | `SKILL.md` + `run.sh` (`0755`) | `cursor@project` | Lossy / Dropped (strict fail) | Emits `.cursor/rules/deploy.mdc`; drops `run.sh` with findings. |
| **A05** | `codex` (`.codex/skills/deploy/`) | `SKILL.md` with unmapped frontmatter | `claude@project` | Lossy (strict fail) | Emits `.claude/skills/deploy/SKILL.md`; reports frontmatter loss. |
| **A06** | `codex` (`AGENTS.md` with `0755`) | Instruction markdown file with `+x` | `claude@project` | Lossy (strict fail) | Emits `CLAUDE.md` (`0644`); reports unrepresentable executable bit. |
| **A07** | Mixed `AGENTS.md` + `.agents/skills/` | Rule + shared skill directory | `claude@project` | Supported (strict pass) | Auto-detection resolves to `codex`; emits `CLAUDE.md` and `.claude/skills/`. |
| **A08** | Mixed `.cursor/rules/` + `.agents/skills/` | Rule + shared skill directory | `antigravity@project` | Supported (strict pass) | Auto-detection resolves to `cursor-mdc`; emits rules and skills. |
| **A09** | Pure `.agents/skills/` snapshot | Standalone skill snapshot | `codex@project` | Supported (strict pass) | Compiles with explicit `--from codex`; emits `.codex/skills/`. |
| **A10** | `.codex/skills/` and `.agents/skills/` | Duplicate skill name `skill:deploy` | Any | Rejected (pre-lowering) | Fails with duplicate semantic identity collision error. |
| **A11** | Skill with symlinked file | Companion file is symlink | Any | Rejected (input observation) | Fails observation; symlinks rejected. |
| **A12** | Valid skill package | Standard skill | `cursor@user` | Rejected (pre-stage target) | Fails pre-stage check; user scope unsupported for Cursor. |

---

## Summary of post-approval implementation work

1. **[`src/inputs/mod.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/inputs/mod.rs#L335-L342):**
   Read `(entry.header().mode()? & 0o111) != 0` for archive entries.
2. **[`src/emitters/lowering.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/emitters/lowering.rs#L241-L406):**
   Allow `SkillResource` companions across skill-directory-capable targets.
   Propagate `executable: resource.executable` onto `NativeArtifact` for `SkillResource`.
   Emit `Lossy` findings when non-empty skill `frontend_payload` fields drop.
3. **[`src/parsers/frontend.rs`](file:///home/mkobit/workspace/mkobit/rulette/src/parsers/frontend.rs#L250-L398) and parsers:**
   Treat `.agents/skills/...` and `skills/...` as universal skill paths.
   Extend `codex.rs` and `claude.rs` skill root matchers.
