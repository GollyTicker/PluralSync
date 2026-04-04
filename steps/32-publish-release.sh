#!/bin/bash

# Publishes the tag at the current revision (assuming the current git revision is tagged like v2.13 or so).

set -euo pipefail

TAG=$(git describe --tags --exact-match || true)

if [ "$TAG" == "" ]; then
    echo "Error: No tag found for the current revision."
    exit 1
fi

echo "Found tag: $TAG"
VERSION="${TAG#v}"

if ! gh auth status; then
    echo "You are not logged into GitHub."
    echo "Please login to publish the release."
    gh auth login --web
fi

./steps/30-build-release.sh

OUT_DIR="target/release_builds"

# Sign artifacts
if [ -f "$OUT_DIR/PluralSync-Bridge-Windows-Setup.exe" ]; then
  echo "Signing Windows installer..."
  TAURI_SIGNING_PRIVATE_KEY="$TAURI_SIGNING_PRIVATE_KEY" \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" \
  cargo tauri signer sign \
    "$OUT_DIR/PluralSync-Bridge-Windows-Setup.exe"
fi

if [ -f "$OUT_DIR/PluralSync-Bridge-Linux.AppImage" ]; then
  echo "Signing Linux AppImage..."
  TAURI_SIGNING_PRIVATE_KEY="$TAURI_SIGNING_PRIVATE_KEY" \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" \
  cargo tauri signer sign \
    "$OUT_DIR/PluralSync-Bridge-Linux.AppImage"
fi

# Generate current.json
cat > "$OUT_DIR/current.json" << EOF
{
  "version": "$VERSION",
  "notes": "Release $VERSION",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo "current.json: $(cat "$OUT_DIR/current.json")"

git push
git push --tags

ADDITIONAL_ARGS=()
if [[ "$TAG" == *"-"* ]]; then
  ADDITIONAL_ARGS+=(--prerelease)
fi
gh release create "$TAG" target/release_builds/* --title "$TAG" --notes "" "${ADDITIONAL_ARGS[@]}"

echo "Release $TAG created successfully."
