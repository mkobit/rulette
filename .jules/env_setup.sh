#!/bin/bash
# Jules environment setup
# Docs: https://jules.google/docs/environment/

set -euo pipefail

echo "Setting up environment..."

# Diagnostic Info
echo "User: $(whoami)"
echo "Git Commit: $(git rev-parse --short HEAD) ($(git log -1 --format=%cI))"

# Install mise if missing
if ! command -v mise &> /dev/null; then
    echo "Installing mise..."
    # Pin to latest version for security and convergence
    MISE_VERSION="v2026.4.5" curl https://mise.run | sh
    export PATH="$HOME/.local/bin:$PATH"
fi

echo "Installing tools with mise..."
mise trust
mise install

# Activate mise
eval "$(mise activate bash)"
eval "$(mise env bash)"

# Check if mise activation is already in .bashrc to avoid duplicates
if ! grep -q "mise activate bash" ~/.bashrc; then
    echo 'eval "$(mise activate bash)"' >> ~/.bashrc
fi

# Verify Environment
if command -v rustc &> /dev/null; then
    echo "Rust version: $(rustc --version)"
else
    echo "Error: Rust not found after mise install"
    exit 1
fi

if command -v cargo &> /dev/null; then
    echo "Cargo version: $(cargo --version)"
else
    echo "Error: Cargo not found after mise install"
    exit 1
fi

echo "Environment ready"
