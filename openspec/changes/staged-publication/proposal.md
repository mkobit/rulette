## Why

Rulette currently combines rendering and publication in one `transform` invocation, so a correct conversion can still write to an unintended scope or overwrite content that changed after review.
Tool configuration tiers are not portable enough to infer write authority from a target name, destination path, transform config, or environment.
Project-level configuration is the common portable tier, user-level paths vary by tool, and managed or system paths may be controlled by an administrator rather than the invoking user.
A staged workflow separates deterministic compilation from authorized publication while preserving Rulette's stateless, single-verb model.

## What changes

- Keep `transform` as the single source-to-sink verb rather than adding separate stage or publish commands.
- Add `transform --stage <directory>` to render outputs into an isolated staging tree without writing any configured destination.
- Require staging to produce a plan that identifies every artifact, intended target, scope, destination, semantic-loss decision, and content digest needed for later verification.
- Add `transform --apply <plan>` as the only mode that may publish staged artifacts to destination paths.
- Require each apply invocation to explicitly authorize every destination scope in the plan, with no authority inherited from the plan, a transform config, environment variables, or prior runs.
- Guarantee a project-scope destination mapping for every supported publication target.
- Permit user-scope publication only for target mappings verified against that tool's primary documentation and covered by conformance tests.
- Exclude portable `local` and `enterprise` scope publication from the v0.1 contract because their meaning, location, precedence, and ownership are not consistent across tools.
- Reject publication to managed or system-owned paths even when a plan requests them or replacement is otherwise authorized.
- Treat an existing byte-identical destination as unchanged and fail on an existing differing destination unless the apply invocation includes `--replace`.
- Verify every staged artifact against the plan's digest before any destination mutation and fail the entire apply if any artifact is missing, changed, or unverifiable.
- Apply every authorized artifact as one all-or-nothing transaction, including rollback of destinations already changed if a later publication fails.
- Fail staging on any semantic loss by default and require an explicit `--allow-lossy` opt-in that is recorded with the plan's loss findings.
- **BREAKING**: Preserve stdout rendering for pipeline composition, but replace direct publication to live tool configuration paths with the stage/apply workflow.

## Scope

This change covers the stage-plan-apply lifecycle, scope authorization, portable target mappings, overwrite policy, digest verification, semantic-loss gating, and transactional publication.
The staged plan is a complete, reviewable handoff between compilation and publication, and apply consumes its artifacts rather than reparsing sources or rerunning transformations.
Publication policy and transaction orchestration belong in the library, while the CLI remains a thin adapter for `transform` arguments and reporting.

## Non-goals

- This change does not add a new top-level verb, initialization workflow, daemon, lockfile, configuration discovery, or local authority database.
- This change does not promise portable `local` or `enterprise` publication in v0.1.
- This change does not install, edit, or bypass administrator-managed policy.
- This change does not infer write authority from filesystem location, target defaults, transform-config scope declarations, or interactive environment state.
- This change does not define remote publication, synchronization, deployment, or registry behavior.
- This change does not alter the IR entity model or target serialization formats.

## Capabilities

### New capabilities

- `staged-publication`: Deterministic staging, digest-bearing publication plans, explicit scope authorization, verified target-to-scope mappings, conflict policy, and transactional apply behavior.

### Modified capabilities

- `transform-pipeline`: Replace direct destination publication with the stage/apply lifecycle, preserve stdout composition, and apply digest, replacement, and all-or-nothing guarantees across every planned destination.
- `frontends-and-backends`: Require emitters to surface semantic loss before staging, fail on loss by default, support `--allow-lossy` as an explicit recorded exception, and expose only verified publication mappings for supported scopes.

## Expected base specification changes

- Add a new `staged-publication` specification for plan contents, artifact isolation, scope authorization, mapping eligibility, digest verification, overwrite behavior, and transactional apply.
- Modify `transform-pipeline` to distinguish non-publishing stdout rendering, isolated staging writes, and destination publication performed only by `transform --apply`.
- Modify `transform-pipeline` so its existing all-or-nothing guarantee applies to the full planned publication set after all authorization, integrity, path-safety, and conflict checks pass.
- Modify `frontends-and-backends` so semantic loss is a default error for staged output and `--allow-lossy` is an explicit, auditable opt-in rather than an implicit warning-only path.
- Coordinate with the active `transform-config` capability so `scope` can select a requested tier but can never grant apply authority.
- Leave `ir-core` unchanged because publication scope and authority are destination policy rather than entity semantics.

## Safety rationale

Compilation and publication have different trust boundaries, so separating them allows review and digest verification without introducing hidden state.
Explicit apply-time scope authorization prevents an untrusted source file, generated plan, or checked-in transform config from granting itself broader write access.
The project tier is the portable baseline documented across [Claude settings](https://code.claude.com/docs/en/settings), [Codex instructions](https://learn.chatgpt.com/docs/agent-configuration/agents-md), [OpenCode rules](https://dev.opencode.ai/docs/rules/), [Cursor rules](https://cursor.com/docs/rules), and [Antigravity rules](https://antigravity.google/docs/ide-rules).
User-tier publication remains allow-listed because path and precedence behavior differ across [Claude memory](https://code.claude.com/docs/en/memory), [Codex configuration](https://learn.chatgpt.com/docs/config-file/config-basic), [OpenCode configuration](https://dev.opencode.ai/docs/config), [Cursor hooks](https://cursor.com/docs/hooks), and [Antigravity settings](https://antigravity.google/docs/settings?app=cli).
Managed and system destinations are excluded because vendor administration and policy mechanisms such as [Codex managed configuration](https://learn.chatgpt.com/docs/enterprise/managed-configuration), [OpenCode policies](https://dev.opencode.ai/docs/policies/), [Cursor enterprise deployment](https://cursor.com/docs/enterprise/deployment-patterns), and [Antigravity permissions](https://www.antigravity.google/docs/cli-permissions) are not ordinary portable user-write targets.
Digest verification closes the review-to-apply gap, while the default conflict and loss failures prevent silent overwrite or semantic degradation.

## Test and release rationale

- Contract tests SHALL prove that staging is deterministic, writes only inside its staging directory, and records a digest for every planned artifact.
- Integrity tests SHALL mutate, remove, and substitute staged artifacts and assert that apply fails before changing any destination.
- Scope tests SHALL cover project mappings for every supported target, each allow-listed user mapping, explicit authorization failures, unsupported tiers, and managed or system path rejection.
- Conflict tests SHALL cover missing, identical, and differing destinations with and without `--replace`.
- Loss tests SHALL prove that staging fails by default and that `--allow-lossy` records the accepted loss in the plan.
- Failure-injection tests SHALL prove preflight behavior and rollback to exact pre-apply content across multi-target publication.
- CLI integration tests SHALL prove that stdout rendering and staging never publish destinations and that only `transform --apply <plan>` can do so.
- Release notes SHALL call out the replacement of direct file publication with stage/apply and provide a migration example for existing `transform --out` workflows.
- The release SHALL preserve the fully static binary and add no runtime service, state store, or configuration dependency.

## Impact

- `src/cli/commands/transform.rs`: add stage and apply argument handling while delegating publication policy and transactions to the library.
- Library publication modules: add the staged-plan model, digest verification, scope authorization, verified path mapping, conflict checks, and transactional apply behavior.
- Emitter capability reporting: make loss detection authoritative for the default-fail staging gate and record accepted loss in plans.
- `openspec/specs/transform-pipeline/spec.md`: define the revised publication boundary and transactional behavior.
- `openspec/specs/frontends-and-backends/spec.md`: define loss-gating and verified scope-mapping responsibilities.
- `openspec/specs/staged-publication/spec.md`: add the new publication safety contract.
- `docs/2026-04-11-prd.md` and `docs/2026-04-11-man-page.md`: document the stage/apply workflow, scope authorization, `--replace`, and `--allow-lossy`.
- Existing transform-config artifacts: retain scope requests as declarative intent but never treat them as authorization.
