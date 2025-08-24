#!/usr/bin/env bash

set -euo pipefail

# Ensure we're on an Unix system
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
  echo "This script is intended for Unix-like systems only"
  echo "Check if the \$OSTYPE variable is set correctly."
  exit 1
fi

# Ensure rustup is installed
if ! command -v rustup &>/dev/null; then
  echo "rustup not found. Press enter to install it."
  read -r
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source "$HOME/.cargo/env"
fi

# Install toolchain and components
rustup show

# Install cargo tools
cargo install \
  cargo-make \
  cargo-edit \
  cargo-watch \
  cargo-nextest \
  cargo-outdated \
  cargo-udeps \
  cargo-deny \
  cargo-audit \
  cargo-expand \
  concurrently

echo "Rust setup complete. Please restart your terminal to apply changes."
