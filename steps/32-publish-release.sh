#!/bin/bash

# Publishes the tag at the current revision (assuming the current git revision is tagged like v2.13 or so).

set -euo pipefail

TAG=$(git describe --tags --exact-match)

if [ -z "$TAG" ]; then
    echo "Error: No tag found for the current revision."
    exit 1
fi

echo "Found tag: $TAG"

if ! gh auth status; then
    echo "You are not logged into GitHub."
    echo "Please login to publish the release."
    gh auth login --web
fi

./steps/30-build-release.sh

git push
git push --tags
# Push to github as well, since we host releases there.
git push github
git push --tags github

ADDITIONAL_ARGS=()
if [[ "$TAG" == *"-"* ]]; then
  ADDITIONAL_ARGS+=(--prerelease)
fi
gh release create "$TAG" target/release_builds/* --title "$TAG" --notes "" "${ADDITIONAL_ARGS[@]}"

echo "Release $TAG created successfully."
