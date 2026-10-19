# Change: Skill companion fidelity and shared layout interoperability

## Why

Skills authoring has converged on self-contained directory trees with portable `SKILL.md` entry points, companion scripts/references, and shared `.agents/skills/` project placement.
Currently, Rulette drops companion resources across different harness domains, discards `0755` executable bits with lossy findings, and treats `.agents/skills/` as an exclusive Antigravity path—causing auto-detection failure when `AGENTS.md` and `.agents/skills/` coexist.
To support a "write once, work everywhere" local compilation model, Rulette must preserve companion files and executable modes across skill-directory targets and recognize shared skills layouts uniformly.

## What changes

- Preserve relative `SkillResource` companions across all targets supporting skill directories (`codex`, `claude`, `opencode`, `antigravity`, `agent-plugin`) with `Supported` (`Representable`) capability status.
- Preserve `0755` executable mode on `SkillResource` artifacts during lowering, plan generation, and staged application, and extract entry modes in archive inputs.
- Reserve `Dropped` (`OpaqueResourceUnrepresentable`) and `Lossy` (`ExecutableBitUnrepresentable` / `SkillLoweredAsRule`) findings for targets without skill directory support (such as flat rules in `cursor`) and primary markdown instruction files.
- Report `Lossy` (`SkillFrontmatterUnrepresentable`) when unmapped skill `frontend_payload` keys drop across domains.
- Classify `.agents/skills/<name>/SKILL.md` and `skills/<name>/SKILL.md` as shared skill layouts across all skill-capable frontends rather than exclusive Antigravity markers.
- Resolve `--from auto` unambiguously to the companion rule harness (such as `codex` for `AGENTS.md`) when coexisting with `.agents/skills/`.

## Impact

Cross-domain skill compilation between Codex, Claude, OpenCode, Antigravity, and AgentPlugin succeeds under strict mode without dropping companion scripts or permissions.
Repositories with modern `.agents/skills/` layouts compile cleanly without requiring callers to manually copy files into `.codex/skills/`.
Flat rule targets (Cursor) and unmapped frontmatter retain strict loss rejection by default.
No remote registries, network access, or runtime plugin execution are introduced.
