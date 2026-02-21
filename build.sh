#!/usr/bin/env bash
set -e

echo "Installing PHP Version Manager (pvm) from source..."

if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust (https://rustup.rs/)."
    exit 1
fi

echo "Building pvm release binary..."
cargo build --release

PVM_DIR="${PVM_DIR:-$HOME/.local/share/pvm}"
PVM_BIN_DIR="$PVM_DIR/bin"

mkdir -p "$PVM_BIN_DIR"

echo "Installing binary to $PVM_BIN_DIR/pvm..."
cp target/release/pvm "$PVM_BIN_DIR/pvm"

echo "Successfully built and installed pvm!"
echo ""
echo "Please add the following to your shell profile (.bashrc, .zshrc, or .config/fish/config.fish):"
echo "  eval \"\$($PVM_BIN_DIR/pvm env)\""
