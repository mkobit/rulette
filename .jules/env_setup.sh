#!/bin/bash
# Jules environment setup
# Docs: https://jules.google/docs/environment/

set -euo pipefail

echo "Setting up environment..."
echo "--- Diagnostic Information ---"
echo "User: $(whoami)"
echo "Git commit: $(git rev-parse --short HEAD) ($(git log -1 --format=%cI))"
echo "------------------------------"

# Install mise if missing
if ! command -v mise &> /dev/null; then
    echo "Installing mise..."
    MISE_VERSION="v2026.5.15"
    mkdir -p ~/.local/bin
    curl -L "https://github.com/jdx/mise/releases/download/${MISE_VERSION}/mise-${MISE_VERSION}-linux-x64" > ~/.local/bin/mise
    chmod +x ~/.local/bin/mise
    export PATH="$HOME/.local/bin:$PATH"
fi

echo "Installing tools with mise..."
mise trust
mise install
eval "$(mise activate bash)"
eval "$(mise env bash)"

if ! grep -q "mise activate bash" ~/.bashrc 2>/dev/null; then
    echo 'eval "$(mise activate bash)"' >> ~/.bashrc
fi

if command -v rustc &> /dev/null; then
    echo "Rust: $(rustc --version)"
else
    echo "Error: rustc not found after mise install"
    exit 1
fi

if command -v cargo &> /dev/null; then
    echo "Cargo: $(cargo --version)"
else
    echo "Error: cargo not found after mise install"
    exit 1
fi

echo "Mise: $(mise --version)"

echo "Environment ready"
