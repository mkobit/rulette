# Why Rulette exists: the case for a compiler

2026-04-11

## The AI tooling fragmentation problem

It is April 2026 and there are at least seven major AI coding assistants, each with its own rule format.
If you maintain coding standards for an organization, you maintain them seven times.
If a new tool launches next month (and one will), you maintain them eight times.

This is not a hypothetical.
Teams at companies using Claude Code, Cursor, and Copilot simultaneously already maintain three separate copies of their coding standards.
The rules drift.
Nobody notices until a junior developer gets contradictory guidance from different tools.

For individuals, the problem is similar but personal: your preferred coding style, security policies, and workflow conventions live in `~/.claude/CLAUDE.md` on one machine, `.cursorrules` in every project, and nowhere for Copilot.
There is no `chezmoi` for AI rules.

## The supply chain problem

The current solution to "I want to share rules" is:

1. Copy-paste from cursor.directory (a community wiki with no review process)
2. `npx skills add` from Vercel's library (fetches HEAD of a GitHub repo)
3. Clone a "awesome-cursorrules" repo (unknown provenance, mutable content)
4. Use rulesync (Node.js script that templates strings into files)

None of these verify what they download.
None pin to a known-good version.
None produce reproducible output.
All require a JavaScript runtime.

For a security-conscious organization, this is unacceptable.
AI coding rules are _system prompts_ — they control what code your AI assistant writes.
A compromised rule can instruct an AI to introduce vulnerabilities, exfiltrate code, or ignore security policies.
This is not theoretical; prompt injection in shared rules has been demonstrated repeatedly.

## What Rulette does differently

**Rulette is `protoc` for AI rules.**
It reads any format, compiles to a typed IR, transforms deterministically, and emits to any target.

### 1. One source of truth, every target format

Write your rules once as generic skills (or in any format you prefer).
Rulette converts to every target:

```sh
rulette convert rules/ --to claude --out .claude/skills/
rulette convert rules/ --to cursor-mdc --out .cursor/rules/
rulette convert rules/ --to codex --out AGENTS.md --merge
```

Need more control? Pipe individual commands:

```sh
rulette parse rules/ | rulette transform --filter 'license == "MIT"' | rulette emit --to claude
```

When the eighth AI tool launches, add a backend.
Your rules don't change.

### 2. Works for you, not just your project

Rulette handles both project-scoped and user-scoped rules.
Manage your personal AI configuration across machines the same way you manage dotfiles:

```sh
# Emit personal rules to every AI tool's user-global config
rulette convert ~/dotfiles/ai-rules/ --to claude --scope user
rulette convert ~/dotfiles/ai-rules/ --to cursor-mdc --scope user
rulette convert ~/dotfiles/ai-rules/ --to codex --scope user
```

Integrates cleanly with chezmoi, stow, or any dotfile manager.
No more `npx skills add` or tool-specific install commands.

### 3. Deterministic and reproducible

Rulette is a pure function: same input, same output, every time.
No network calls.
No mutable state.
No "it worked on my machine."

This makes it safe for CI/CD, Bazel builds, and air-gapped environments.
Pin your rules in a lockfile, verify integrity in CI, and ship with confidence.

### 4. Composable with Unix tools

Rulette's IR is JSON.
Pipe it through `jq` to filter.
Pipe rule bodies through `markdownlint` to lint.
Chain with `grep`, `sed`, or any tool in your pipeline.

```sh
rulette parse rules/ --to json \
  | jq '[.[] | select(.metadata.license == "MIT")]' \
  | rulette emit --to claude
```

### 5. Static binary, zero dependencies

One binary.
No Node.js.
No Python.
No Docker.
Runs in Bazel sandboxes, Alpine containers, and air-gapped build machines.

Download it, verify the checksum, put it in your PATH.
It will still work in five years.

### 6. Security by default

Network access is opt-in and requires explicit flags.
Fetched content is verified against a lockfile.
The dangerous path (`--allow-mutable --no-verify`) is intentionally verbose.

```sh
# Safe: verified against lockfile
rulette fetch --lockfile rules.lock

# Unsafe: must type the whole thing
rulette fetch some-repo --allow-mutable --no-verify
```

## CLI at a glance

All commands are flat and top-level, like `protoc` and `jq`:

```text
rulette parse               Parse any rule format into IR
rulette emit                Emit IR to any target format
rulette convert             Parse + emit in one step (the 80% case)
rulette inspect             Pretty-print IR for debugging
rulette schema              Output JSON Schema for any format
rulette transform           Apply filters, renames, merges, shell pipes (v0.1.1)

rulette archive             Bundle into content-addressed archive (v0.2)
```

Top-level aliases (`rulette parse`, `rulette emit`, etc.) work for the common case.

## Who should use Rulette

- **Organizations using multiple AI coding assistants**: Write rules once, emit everywhere.
- **Security-conscious teams**: Pin, verify, and audit every rule in your codebase.
- **Platform/DevEx teams**: Distribute coding standards as versioned, content-addressed archives.
- **Open source maintainers**: Publish rules that consumers can pin and verify.
- **Bazel/Nix/hermetic build users**: A tool that actually works in your build system.
- **Dotfile enthusiasts**: Manage personal AI rules across machines with chezmoi or stow.

## Who should not use Rulette

- If you use one AI tool and maintain rules by hand, you don't need a compiler.
- If you don't care about reproducibility, `npx skills add` is fine.
- If you want a GUI rule editor, Rulette is a CLI tool.
