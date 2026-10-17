#!/bin/sh
set -eu

fail() {
    printf 'SBX preflight failed: %s\n' "$*" >&2
    exit 1
}

[ "$#" -eq 2 ] || fail 'Expected agent (codex or agy) and absolute checkout path'
agent=$1
case "$agent" in
    codex|agy) ;;
    *) fail "Unsupported agent: $agent" ;;
esac
case "$2" in
    /*) ;;
    *) fail 'Checkout path must be absolute' ;;
esac
expected=$(cd "$2" && pwd -P) || fail "Cannot read checkout: $2"
[ "$(pwd -P)" = "$expected" ] || fail "Workspace mismatch: expected $expected, got $(pwd -P)"
[ -f Cargo.toml ] && [ -f AGENTS.md ] && [ -d .sbx ] || fail 'Not a Rulette checkout'

for tool in "$agent" mise cargo rustc rustup bun bd file cc musl-gcc git rg tar gzip sha256sum mktemp chmod rm rmdir grep; do
    command -v "$tool" >/dev/null 2>&1 || fail "Missing tool: $tool"
done
for tool in "$agent" mise cargo rustc rustup bun bd file cc musl-gcc git rg; do
    "$tool" --version || fail "Cannot execute: $tool --version"
done
cargo fmt --version || fail 'Cannot execute Rust formatter'
cargo clippy --version || fail 'Cannot execute Rust clippy'
components=$(rustup component list --installed) || fail 'Cannot list installed Rust components'
printf '%s\n' "$components" | grep -q '^llvm-tools' || fail 'Missing Rust llvm-tools component'

scratch=$(mktemp -d "${TMPDIR:-/tmp}/rulette-sbx-smoke.XXXXXX") || fail 'Cannot create temporary directory'
cleanup() {
    status=$?
    trap - EXIT
    cleanup_failed=0
    if [ -e "$scratch/probe" ] || [ -L "$scratch/probe" ]; then
        rm "$scratch/probe" || cleanup_failed=1
    fi
    rmdir "$scratch" || cleanup_failed=1
    if [ "$cleanup_failed" -ne 0 ]; then
        printf 'SBX preflight cleanup failed: %s\n' "$scratch" >&2
        [ "$status" -ne 0 ] || status=1
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
printf '#!/bin/sh\nexit 0\n' > "$scratch/probe"
chmod 700 "$scratch/probe"
"$scratch/probe" || fail 'Cannot execute temporary files'
printf 'SBX preflight passed: agent=%s workspace=%s\n' "$agent" "$expected"
