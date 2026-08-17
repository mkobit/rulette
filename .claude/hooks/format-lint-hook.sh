#!/usr/bin/env bash
# PostToolUse hook (Edit|Write|MultiEdit): auto-formats touched Rust and
# Markdown files in place.
set -euo pipefail

cd "${CLAUDE_PROJECT_DIR:-.}"

payload="$(cat)"

mapfile -t paths < <(jq -r '
  [.tool_input.file_path, (.tool_input.edits // [])[].file_path]
  | map(select(. != null))
  | unique[]
' <<<"$payload")

for p in "${paths[@]}"; do
  [ -f "$p" ] || continue
  case "$p" in
    *.rs) cargo fmt -- "$p" 2>/dev/null || true ;;
    *.md) markdownlint-cli2 --fix "$p" >/dev/null 2>&1 || true ;;
  esac
done

exit 0
