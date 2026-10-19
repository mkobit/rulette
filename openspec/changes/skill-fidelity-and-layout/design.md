# Design: Skill companion fidelity and shared layout interoperability

## Context and architectural constraints

Rulette compiles AI rules and skills offline into deterministic package graphs and staged publication plans.
Per D10 and D11 decisions approved on `rulette-6lj`:

- Skills must preserve relative companions and executable permissions when lowered to targets supporting skill directories.
- Shared `.agents/skills` layouts must decode across native frontends without ambiguous detection errors.
- Hard constraints remain preserved: static binary, library logic, no ambient discovery, no network access, strict loss rejection by default.

## Architecture

### 1. Skill companion and permission lowering (D10)

1. **Companion resource emission:**
   In `lower_skill_package`, remove the domain equality constraint (`package.provenance.frontend == target.as_str()`) for `ResourceRole::Opaque`.
   When the target supports skill directories (`Codex`, `Claude`, `OpenCode`, `Antigravity`, `AgentPlugin`), emit all companion resources as `NativeArtifactClass::SkillResource` at `skills/{name}/{resource.path}` with `CapabilitySeverity::Supported`.
2. **Executable mode preservation:**
   For `NativeArtifactClass::SkillResource`, propagate `executable: resource.executable` to `NativeArtifact`.
   Do not emit `ExecutableBitUnrepresentable` when the target preserves the resource with `executable: true`.
   For primary instruction markdown files (`AGENTS.md`, `CLAUDE.md`, `*.mdc`, `SKILL.md`), enforce `executable: false`. If a markdown instruction file has `+x` set, emit `ExecutableBitUnrepresentable` (`Lossy`).
   In `inputs::observe_archive`, extract `(entry.header().mode()? & 0o111) != 0` so tar and gzip-tar archives preserve executable bits identically to directory inputs.
3. **Cursor flat rule target:**
   When targeting `cursor@project`, continue lowering skills to `.cursor/rules/{name}.mdc`.
   Emit `SkillLoweredAsRule` (`Lossy`) and drop all companion resources with `OpaqueResourceUnrepresentable` (`Dropped`).
   If a dropped companion has `executable: true`, emit `ExecutableBitUnrepresentable` (`Lossy`).
4. **Frontmatter loss reporting:**
   When non-empty skill `frontend_payload` fields are unmapped across domains, emit a `Lossy` finding (`SkillFrontmatterUnrepresentable`).

### 2. Shared layout interoperability (D11)

1. **Frontend candidate classification:**
   In `native_frontend_candidates`, separate `.agents/skills/` from Antigravity-exclusive markers (`.antigravity/`, `.agent/`, `.agents/rules/`).
   Allow paths under `.agents/skills/...` and `skills/...` to decode under any skill-capable frontend without foreign frontend errors in `validate_native_observations`.
2. **Auto-detection resolution:**
   In `detect_graph_format`, when `.agents/skills/...` coexists with unambiguous harness rule files:
   - `AGENTS.md` or `.codex/` resolves to `NativeFrontend::Codex`.
   - `.cursor/rules/` or `*.mdc` resolves to `NativeFrontend::CursorMdc`.
   - `CLAUDE.md` or `.claude/` resolves to `NativeFrontend::Claude`.
   - `opencode.json` or `.opencode/` resolves to `NativeFrontend::OpenCode`.
   - `.antigravity/`, `.agent/`, or `.agents/rules/` resolves to `NativeFrontend::Antigravity`.
   When only `.agents/skills/...` exists without other harness markers, resolve deterministically to a documented default skill frontend without reporting multi-frontend ambiguity.
3. **Parser skill root recognition:**
   Extend `codex.rs` and `claude.rs` skill root matchers to recognize `.agents/skills/<name>/...` and `skills/<name>/...`.
   Support direct directory inputs where relative paths produce `<name>/SKILL.md` or `SKILL.md`.

## Adversarial review considerations

1. **Risk of path traversal in companions:**
   All companion paths are validated as normalized relative paths within the package during observation. Traversal (`..`, leading `/`, symlinks) remains strictly rejected.
2. **Collision between layouts:**
   If a snapshot contains both `.codex/skills/foo/SKILL.md` and `.agents/skills/foo/SKILL.md`, aggregation deterministically rejects the build with a duplicate semantic identity collision.
3. **Overpromising Cursor fidelity:**
   Lowering skills to Cursor continues to report `SkillLoweredAsRule` and drops companions with explicit findings. Strict mode rejects the compilation unless `--allow-lossy` is passed.
