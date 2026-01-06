#!/bin/bash

set -euo pipefail

source test/source.sh

./steps/21-bridge-frontend-tauri-build.sh

echo "! Test assumes that pluralsync-api is running !"

echo "TODO: fix me. Ensure that window is opened into the foreground for the test!"
exit 1

# also bug on windows icon not being shown: https://github.com/tauri-apps/tauri/issues/2692

(cd bridge-frontend && npm run e2e)
