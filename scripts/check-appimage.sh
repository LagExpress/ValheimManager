#!/usr/bin/env bash
set -euo pipefail

IMAGE=$(readlink -f "${1:?Usage: check-appimage.sh <AppImage>}")
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$ROOT_DIR/target"
CHECK_DIR=$(mktemp -d "$ROOT_DIR/target/appimage-check.XXXXXX")
trap 'rm -rf "$CHECK_DIR"' EXIT
OFFSET=$("$IMAGE" --appimage-offset)
unsquashfs -no-progress -d "$CHECK_DIR/AppDir" -o "$OFFSET" "$IMAGE" > /dev/null
if [[ ! -x "$CHECK_DIR/AppDir/usr/lib/glibc/ld-linux-x86-64.so.2" ]]; then
    echo 'AppImage is missing its bundled ELF loader.' >&2
    exit 1
fi
env -u LD_LIBRARY_PATH -u LD_PRELOAD VALHEIM_APPIMAGE_CHECK=1 "$CHECK_DIR/AppDir/AppRun"
for library in "$CHECK_DIR"/AppDir/usr/bin/valheim-modpack-studio "$CHECK_DIR"/AppDir/usr/lib/*.so* "$CHECK_DIR"/AppDir/usr/lib/glibc/*.so*; do
    if [[ $(patchelf --print-rpath "$library") == *'/nix/store/'* || $(patchelf --print-needed "$library") == *'/nix/store/'* ]]; then
        echo "Non-relocatable library: $library" >&2
        exit 1
    fi
done
printf 'AppImage runtime and library resolution verified.\n'
