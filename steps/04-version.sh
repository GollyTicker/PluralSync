#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------- version extraction ----------

extract_version_from_git() {
    local tag
    tag=$(git describe --tags --exact-match 2>/dev/null) && {
        normalize_tag "$tag"
        return
    }

    local latest_tag
    latest_tag=$(git describe --tags --abbrev=0)
    local base
    base=$(normalize_tag "$latest_tag")

    local base_version="${base%%-*}"
    echo "${base_version}-dev"
}

normalize_tag() {
    local tag="$1"

    tag="${tag#v}"

    local main_part="${tag%%-*}"
    local suffix="${tag#*-}"
    if [ "$suffix" = "$tag" ]; then
        suffix=""
    fi

    local major minor patch
    IFS='.' read -r major minor patch <<< "$main_part"
    major="${major:-0}"
    minor="${minor:-0}"
    patch="${patch:-0}"

    local normalized="${major}.${minor}.${patch}"

    if [ -n "$suffix" ]; then
        echo "${normalized}-${suffix}"
    else
        echo "$normalized"
    fi
}

# Patches bridge-src-tauri/template.tauri.conf.json in-place with version and public key,
# then copies it to tauri.conf.json. Makes the version persistent.
patch_tauri_config() {
    local version="${1}"
    local template="bridge-src-tauri/template.tauri.conf.json"
    local output="bridge-src-tauri/tauri.conf.json"

    if [[ ! -f "$template" ]]; then
        echo "Error: $template not found"
        return 1
    fi

    cp "$template" "$output"

    # Patch template in-place (persistent)
    sed -i -e "0,/\"version\": \".*\"/{s/\"version\": \".*\"/\"version\": \"$version\"/}" \
           -e "s/<PUBLIC_KEY_HERE>/$TAURI_SIGNING_PUBLIC_KEY/" \
        "$output"
}
