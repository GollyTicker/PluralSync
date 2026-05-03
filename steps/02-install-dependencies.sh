#!/bin/bash

set -euo pipefail

# We use --no-upgrade to make dev times quicker as we don't always need the newest version.

# Tests
sudo apt-get install -y --no-upgrade oathtool

# Cross-compile to Windows from Linux
sudo apt-get install -y --no-upgrade gcc-mingw-w64-x86-64 gcc-mingw-w64-i686 nsis lld llvm

# Tauri
sudo apt-get install -y --no-upgrade \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

cargo install cargo-audit@^0
cargo install sqlx-cli@^0
cargo install cargo-xwin@^0
cargo install tauri-cli@^2

# Tauri e2e tests
cargo install tauri-driver@^2
sudo apt-get install -y --no-upgrade webkit2gtk-driver

# patch discord-rich-presence
echo "Patching discord-rich-presence..."
PATCHED="target/discord-rich-presence-patched"
rm -rf "$PATCHED" || true
git clone https://github.com/vionya/discord-rich-presence "$PATCHED"
( cd "$PATCHED" && VERSION="$(git tag | tail -n 1)" && echo "Using '$VERSION'" && git -c advice.detachedHead=false checkout "$VERSION" )
patch "$PATCHED/src/activity.rs" < steps/discord-rich-presence.activity.rs.patch
patch "$PATCHED/src/discord_ipc.rs" < steps/discord-rich-presence.discord_ipc.rs.patch
echo "Patch OK!"

# npm
(cd bridge-frontend && npm ci --ignore-scripts)
(cd frontend && npm ci --ignore-scripts)
