#!/bin/bash

set -euo pipefail

source "$(dirname "$0")/04-version.sh"
export VERSION=$(extract_version_from_git)

patch_cargo_versions_on_exit "$VERSION"

export TAURI_APP_PATH="bridge-src-tauri"

if [[ "${TAURI_SIGNING_PRIVATE_KEY:-}" == "" ]]; then
  echo "Warning: TAURI_SIGNING_PRIVATE_KEY not set, artifacts cannot be signed"
  false
fi

cargo tauri build "$@"
