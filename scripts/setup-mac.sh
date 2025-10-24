#!/bin/bash
set -e

echo "Setting up development environment for Mac..."
echo ""
echo "---------------------------------------"
echo "Installing Homebrew if not present..."
echo "---------------------------------------"
echo ""

if ! command -v brew &> /dev/null; then
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
    eval "$(/opt/homebrew/bin/brew shellenv)"
fi

echo "---------------------------------------"
echo "Installing sdl2..."
echo "---------------------------------------"
echo ""
brew install sdl2

echo "---------------------------------------"
echo "Setting up Rust environment..."
echo "---------------------------------------"
echo ""

if ! command -v rustup &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
    echo 'source "$HOME/.cargo/env"' >> ~/.zprofile
fi

source "$HOME/.cargo/env"

rustup target add arm-unknown-linux-gnueabihf

echo "---------------------------------------"
echo "Installing cross for cross-compilation..."
echo "---------------------------------------"
echo ""

cargo install cross --git https://github.com/cross-rs/cross

echo "---------------------------------------"
echo "Initializing git submodules..."
echo "---------------------------------------"
echo ""

git submodule update --init --recursive

echo "---------------------------------------"
echo "Setup complete."
echo "---------------------------------------"
