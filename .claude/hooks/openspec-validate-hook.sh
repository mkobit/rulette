#!/usr/bin/env bash
# PostToolUse hook (Edit|Write|MultiEdit): validates any openspec/changes/<name>/
# directory touched by this tool call and blocks with the failure reason.
set -euo pipefail

cd "${CLAUDE_PROJECT_DIR:-.}"

payload="$(cat)"

mapfile -t paths < <(jq -r '
  [.tool_input.file_path, (.tool_input.edits // [])[].file_path]
  | map(select(. != null))
  | unique[]
' <<<"$payload")

declare -A seen
names=()
for p in "${paths[@]}"; do
  if [[ "$p" =~ openspec/changes/([^/]+)/ ]]; then
    name="${BASH_REMATCH[1]}"
    if [ "$name" != "archive" ] && [ -z "${seen[$name]:-}" ]; then
      seen[$name]=1
      names+=("$name")
    fi
  fi
done

[ "${#names[@]}" -eq 0 ] && exit 0

fail_output=""
for name in "${names[@]}"; do
  if ! out="$(npx @fission-ai/openspec validate "$name" --strict 2>&1)"; then
    fail_output+="openspec change \"${name}\" failed validation:
${out}

"
  fi
done

if [ -n "$fail_output" ]; then
  jq -n --arg reason "$fail_output" '{decision: "block", reason: $reason}'
fi

exit 0
