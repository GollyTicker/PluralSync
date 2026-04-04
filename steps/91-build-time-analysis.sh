#!/bin/bash

set -euo pipefail

source "$(dirname "$0")/04-version.sh"
export VERSION=$(extract_version_from_git)
patch_cargo_versions_on_exit "$VERSION"

echo "base-src"
(cd base-src && cargo build --release --timings)


echo "src"
cargo build --release --timings

echo "bridge-src-tauri"
export TAURI_APP_PATH="bridge-src-tauri"
cargo tauri build -- --timings


echo ""

echo "base-src: $PWD/base-src/target/cargo-timings/cargo-timing.html"
echo "src: $PWD/target/cargo-timings/cargo-timing.html"
echo "brdge-src-tauri: $PWD/bridge-src-tauri/target/cargo-timings/cargo-timing.html"
