#!/usr/bin/env bash

# Ensure we're on an Unix system
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
  echo "This script is intended for Unix-like systems only"
  echo "Check if the \$OSTYPE variable is set correctly."
  exit 1
fi

export NVM_DIR="$HOME/.nvm"
if [ -s "$NVM_DIR/nvm.sh" ]; then
  . "$NVM_DIR/nvm.sh"
else
  echo "nvm not found. Press enter to install it."
  read -r
  curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
  . "$NVM_DIR/nvm.sh"
fi

# Install the node version specified in .nvmrc
nvm install

pushd src/webui || exit
npm install
popd || exit

echo "NodeJS setup complete. Please restart your terminal to apply changes."
