# Tasks: Skill companion fidelity and shared layout interoperability

## 1. Specification and design baseline

- [ ] 1.1 Complete proposal, design, and specification deltas for D10 and D11.
- [ ] 1.2 Validate OpenSpec changes with `mise run spec-validate`.

## 2. Skill companions and executable metadata (rulette-6lj.2 / D10)

- [x] 2.1 Read archive entry mode bits in `src/inputs/mod.rs` to preserve executable flags in tar inputs.
- [x] 2.2 Allow `SkillResource` companions to lower across skill-directory targets in `src/emitters/lowering.rs`.
- [x] 2.3 Propagate executable permissions to `NativeArtifact` for `SkillResource` and preserve mode in staged plans.
- [x] 2.4 Report `SkillFrontmatterUnrepresentable` when unmapped skill frontmatter keys are dropped across domains.
- [x] 2.5 Add integration tests covering acceptance cases A01-A06.

## 3. Shared layout interoperability (rulette-6lj.3 / D11)

- [x] 3.1 Treat `.agents/skills/...` as a universal skill path across native frontends in `src/parsers/frontend.rs`.
- [x] 3.2 Resolve `--from auto` unambiguously when `.agents/skills/...` coexists with native rule files.
- [x] 3.3 Extend parser skill matchers in `src/parsers/codex.rs` and `src/parsers/claude.rs` for shared layouts.
- [x] 3.4 Add integration tests covering acceptance cases A07-A12.

## 4. Verification and closure (rulette-6lj.4)

- [ ] 4.1 Run full regression suite: `mise run check`.
- [ ] 4.2 Update competitor synthesis document and close `rulette-6lj`.
