#/bin/bash

set -euo pipefail

echo "$GLOBAL_PLURALSYNC_SIMPLY_PLURAL_READ_WRITE_ADMIN_TOKEN" > /dev/null

source "$(dirname "$0")/04-version.sh"
export VERSION=$(extract_version_from_git)
patch_cargo_versions_on_exit "$VERSION"

cargo run --bin pluralsync-global-manager
