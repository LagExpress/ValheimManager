#!/usr/bin/env bash
set -euo pipefail

IMAGE=$(readlink -f "${1:?Usage: check-appimage.sh <AppImage>}")
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$ROOT_DIR/target"
CHECK_DIR=$(mktemp -d "$ROOT_DIR/target/appimage-check.XXXXXX")
trap 'find "$CHECK_DIR" -type d -exec chmod u+w {} +; rm -rf "$CHECK_DIR"' EXIT
OFFSET=$("$IMAGE" --appimage-offset)
unsquashfs -no-progress -d "$CHECK_DIR/AppDir" -o "$OFFSET" "$IMAGE" > /dev/null
if [[ ! -x "$CHECK_DIR/AppDir/usr/lib/glibc/ld-linux-x86-64.so.2" ]]; then
    echo 'AppImage is missing its bundled ELF loader.' >&2
    exit 1
fi
if [[ ! -f "$CHECK_DIR/AppDir/usr/share/X11/xkb/rules/evdev" || ! -f "$CHECK_DIR/AppDir/usr/share/X11/locale/compose.dir" ]]; then
    echo 'AppImage is missing its keyboard/compose data.' >&2
    exit 1
fi
if grep -R -l '/nix/store/' "$CHECK_DIR/AppDir/usr/share/X11"; then
    echo 'Keyboard/compose resources contain build-store paths.' >&2
    exit 1
fi
env -u LD_LIBRARY_PATH -u LD_PRELOAD VALHEIM_APPIMAGE_CHECK=1 "$CHECK_DIR/AppDir/AppRun"
for library in "$CHECK_DIR"/AppDir/usr/bin/valheim-modpack-studio "$CHECK_DIR"/AppDir/usr/lib/*.so* "$CHECK_DIR"/AppDir/usr/lib/glibc/*.so*; do
    if [[ $(patchelf --print-rpath "$library") == *'/nix/store/'* || $(patchelf --print-needed "$library") == *'/nix/store/'* ]]; then
        echo "Non-relocatable library: $library" >&2
        exit 1
    fi
    [[ ${library##*/} == ld-linux* ]] && continue
    resolved=$(env -u LD_LIBRARY_PATH -u LD_PRELOAD \
        "$CHECK_DIR/AppDir/usr/lib/glibc/ld-linux-x86-64.so.2" --inhibit-cache \
        --library-path "$CHECK_DIR/AppDir/usr/lib/glibc:$CHECK_DIR/AppDir/usr/lib" --list "$library")
    while IFS= read -r dependency; do
        case "$dependency" in
            "$CHECK_DIR"/AppDir/*) ;;
            *) echo "Unbundled dependency of $library: $dependency" >&2; exit 1 ;;
        esac
    done < <(sed -n 's/^.*=> \(.*\) (0x[0-9a-f]*).*$/\1/p' <<< "$resolved")
done
printf 'AppImage runtime and library resolution verified.\n'
