#!/bin/bash

set -euo pipefail

TARGETS=(
    "x86_64-pc-windows-gnu"    # 64-bit Windows
    "x86_64-unknown-linux-gnu" # 64-bit Linux (glibc)
)

LINUX_TARGET="x86_64-unknown-linux-gnu"


cleanup_output() {
    OUT_DIR="target/release_builds"
    echo "Cleaning '$OUT_DIR'"
    rm -rf "$OUT_DIR" || true
    mkdir -p "${OUT_DIR}"
}


add_rust_targets() {
    for target in "${TARGETS[@]}"; do
        rustup target add "$target"
    done
}


build_binaries() {
    for target in "${TARGETS[@]}"; do
        echo "🛠️ pluralsync-bridge $target"
        ./steps/22-bridge-frontend-tauri-release.sh --target "$target"
        BUILD_OUT_PATH="bridge-src-tauri/target/$target/release/bundle"
        if [[ "$target" == *"windows"* ]]; then
            cp -v "$BUILD_OUT_PATH"/*/*.exe "$OUT_DIR/PluralSync-Bridge-Windows-Setup.exe"
        else
            cp -v "$BUILD_OUT_PATH"/*/*.rpm "$OUT_DIR/PluralSync-Bridge-Linux.rpm"
            cp -v "$BUILD_OUT_PATH"/*/*.deb "$OUT_DIR/PluralSync-Bridge-Linux.deb"
            cp -v "$BUILD_OUT_PATH"/*/*.AppImage "$OUT_DIR/PluralSync-Bridge-Linux.AppImage"
        fi
        echo "✅ pluralsync-bridge $target"

        echo ""
    done


    echo "🛠️ pluralsync-global-manager $LINUX_TARGET"
    ./steps/12-backend-cargo-build.sh --release --bin pluralsync-global-manager --target "$LINUX_TARGET"
    src_path="target/$LINUX_TARGET/release/pluralsync-global-manager"
    dest_path="${OUT_DIR}/pluralsync-global-manager"
    cp -v "$src_path" "$dest_path"
    echo "✅ pluralsync-global-manager $target"

    echo ""

    echo "🛠️ pluralsync-api $LINUX_TARGET"
    ./steps/12-backend-cargo-build.sh --release --target "$LINUX_TARGET"
    src_path="target/$LINUX_TARGET/release/pluralsync"
    dest_path="${OUT_DIR}/pluralsync-api"
    cp -v "$src_path" "$dest_path"
    echo "✅ pluralsync-api $target"

    echo ""

    echo "🛠️ pluralsync-frontend $LINUX_TARGET"
    ./steps/17-frontend-npm-build.sh
    tar -czvf "$OUT_DIR/pluralsync-frontend.tar.gz" -C frontend/dist .
    echo "✅ pluralsync-frontend $target"
}


main() {
    cleanup_output
    add_rust_targets
    build_binaries

    echo ""
    echo "✅✅✅ Build process finished. Output in: ${PWD}/${OUT_DIR}"
}


main
