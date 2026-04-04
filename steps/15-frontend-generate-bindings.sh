#/bin/bash

set -euo pipefail

source "$(dirname "$0")/04-version.sh"
export VERSION=$(extract_version_from_git)
patch_cargo_versions_on_exit "$VERSION"

cargo run --bin ts-bindings
