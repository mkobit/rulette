# Goals

## Core requirements
Produce a fully static binary with no runtime dependencies.
Read from stdin or explicit file paths, write to stdout or explicit destinations.
Parse Markdown and TOML frontmatter into a unified typed IR.
Process tar archives containing multiple rule or skill files as a single input.
Apply an ordered transformation pipeline over the IR before emission (filter, map, rename, merge).
Emit platform-specific outputs for Gemini extensions and Claude extensions.
Support additional output systems additively without modifying existing backends.

## Distribution
Distribute via static binary releases on GitHub.
Support installation via uvx, bunx, and direct download.
