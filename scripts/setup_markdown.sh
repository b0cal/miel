#!/usr/bin/env bash

# Ensure we're on an Unix system
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
  echo "This script is intended for Unix-like systems only"
  echo "Check if the \$OSTYPE variable is set correctly."
  exit 1
fi

# Ensure node  is installed
if ! command -v node &>/dev/null; then
  echo "node not found. Please run setup_node.sh first."
  exit 1
fi

npm install -g markdownlint-cli2@0.18.1
