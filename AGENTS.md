# Rulette context

## Hard constraints
The binary must be fully static with no runtime dependencies.
The CLI is a thin wrapper; all logic lives in the library.
No initialization phase, no local state, no configuration files.

## Inputs
Single files (path or stdin) and tar archives containing multiple files are both valid inputs.

## Pipeline
Between parsing and emission, the IR passes through an ordered transformation pipeline (filter, map, rename, merge) before reaching any backend.
New backends are additive and do not affect existing ones.
