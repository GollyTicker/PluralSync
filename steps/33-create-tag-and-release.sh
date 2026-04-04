#!/bin/bash

# Fetches the current git tags, creates a new tag (incrementing the minor version),
# and then runs step 32 to publish the release.
#
# Usage:
#   ./steps/33-create-tag-and-release.sh           # Creates v2.66 from v2.65
#   ./steps/33-create-tag-and-release.sh --dev     # Creates v2.66-rc from v2.65

set -euo pipefail

DEV_MODE=false

if [[ "${1:-}" == "--dev" ]]; then
    DEV_MODE=true
fi

# Get the latest tag
LATEST_TAG=$(git tag --list | sort -V | tail -1)

if [ -z "$LATEST_TAG" ]; then
    echo "Error: No tags found in the repository."
    exit 1
fi

echo "Latest tag: $LATEST_TAG"

# Parse the version number (e.g., v2.65 -> 2.65, v2.67-rc -> 2.67)
VERSION_NUM="${LATEST_TAG#v}"
# Remove any -rc suffix before parsing
VERSION_NUM="${VERSION_NUM%%-*}"

# Split into major and minor components
MAJOR="${VERSION_NUM%%.*}"
MINOR="${VERSION_NUM#*.}"

# Increment the minor version
NEW_MINOR=$((MINOR + 1))

# Determine the new tag
if [ "$DEV_MODE" = true ]; then
    NEW_TAG="v${MAJOR}.${NEW_MINOR}-rc"
else
    NEW_TAG="v${MAJOR}.${NEW_MINOR}"
fi

echo "Creating new tag: $NEW_TAG"

source "$(dirname "$0")/04-version.sh"
export VERSION=$(extract_version_from_git)
patch_tauri_config "$VERSION"

git tag "$NEW_TAG"

echo "Tag $NEW_TAG created successfully."

# Run the publish release script
./steps/32-publish-release.sh

