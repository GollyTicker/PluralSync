#!/bin/bash

# Publishes the tag at the current revision (assuming the current git revision is tagged like v2.13 or so).
# Uploads artifacts to content.radicle.ayake.net via SSH.

set -euo pipefail

TAG=$(git describe --tags --exact-match)

if [ -z "$TAG" ]; then
    echo "Error: No tag found for the current revision."
    exit 1
fi

echo "Found tag: $TAG"

./steps/30-build-release.sh

git push
git push --tags
git push github
git push --tags github

IS_PRERELEASE=false
if [[ "$TAG" == *"-"* ]]; then
  IS_PRERELEASE=true
fi

COMMIT_HASH=$(git rev-parse HEAD)

# Upload artifacts to the webserver via SSH
SSH_HOST="content.radicle.ayake.net"
REMOTE_BASE="~/web5/www/PluralSync/releases"
REMOTE_DIR="${REMOTE_BASE}/${TAG}"

echo "Uploading artifacts to ${SSH_HOST}:${REMOTE_DIR}..."

ssh "$SSH_HOST" "mkdir -p '$REMOTE_DIR'" || true
scp target/release_builds/* "${SSH_HOST}:${REMOTE_DIR}/"

ssh "$SSH_HOST" "cd '$REMOTE_DIR' && touch '$TAG'.txt"

echo "{\"commit\":\"${COMMIT_HASH}\",\"tag\":\"$TAG\"}" | ssh "$SSH_HOST" "cat > '${REMOTE_DIR}/info.json'"

echo "Uploaded artifacts for $TAG."

# For non-prerelease tags, update the 'latest' symlink
if [[ "$IS_PRERELEASE" == false ]]; then
    echo "Setting 'latest' symlink to $TAG..."
    ssh "$SSH_HOST" "cd '$REMOTE_BASE' && rm -f latest ; ln -s '$TAG' latest"
fi
ssh "$SSH_HOST" "cd '$REMOTE_BASE' && rm -f dev-latest ; ln -s '$TAG' dev-latest"

echo "Release $TAG published successfully."
