#!/bin/bash
# Jules environment setup
# Docs: https://jules.google/docs/environment/

set -euo pipefail

echo "Setting up environment..."
echo "User: $(whoami)"
echo "Git commit: $(git rev-parse --short HEAD) ($(git log -1 --format=%cI))"

# Install rustup if missing
if ! command -v rustup &> /dev/null; then
    echo "Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# Activate toolchain pinned in rust-toolchain.toml
rustup show

echo "Rust: $(rustc --version)"
echo "Cargo: $(cargo --version)"

# Check mise version
echo "Mise: $(mise --version || echo "mise not installed")"

echo "Environment ready"
