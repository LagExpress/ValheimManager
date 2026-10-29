#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
VERSION=$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | head -n 1)
: "${VERSION:?Could not read the app version from Cargo.toml}"

cargo build --locked --release

APPDIR="$ROOT_DIR/build/AppDir"
DIST_DIR="$ROOT_DIR/dist"
mkdir -p "$DIST_DIR"
if [[ -d $APPDIR ]]; then
    find "$APPDIR" -type d -exec chmod u+w {} +
fi
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib/glibc" \
    "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
    "$APPDIR/usr/share/doc/valheim-runtime"
cp target/release/valheim-modpack-studio "$APPDIR/usr/bin/"
cp valheim-modpack-studio.desktop "$APPDIR/usr/share/applications/"
cp valheim-modpack-studio.desktop "$APPDIR/"
cp src/assets/icon.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/valheim-modpack-studio.png"
cp src/assets/icon.png "$APPDIR/valheim-modpack-studio.png"
cp src/assets/icon.png "$APPDIR/.DirIcon"
cp scripts/AppRun "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"

XKB_ROOT=${APPIMAGE_XKB_ROOT:-/usr/share/X11/xkb}
XLOCALE_ROOT=${APPIMAGE_XLOCALE_ROOT:-/usr/share/X11/locale}
if [[ ! -f "$XKB_ROOT/rules/evdev" || ! -f "$XLOCALE_ROOT/compose.dir" ]]; then
    echo 'Required XKB/compose data missing. Use the declared devenv environment or install xkb-data/libx11-data in CI.' >&2
    exit 1
fi
mkdir -p "$APPDIR/usr/share/X11/xkb" "$APPDIR/usr/share/X11/locale"
cp -aL "$XKB_ROOT/." "$APPDIR/usr/share/X11/xkb/"
cp -aL "$XLOCALE_ROOT/." "$APPDIR/usr/share/X11/locale/"
find "$APPDIR/usr/share/X11" -type d -exec chmod u+w {} +
# Compose includes support %S for the active XLOCALEDIR. Normalize both Nix and
# conventional absolute prefixes so copied locale data remains relocatable.
find "$APPDIR/usr/share/X11/locale" -type f -name Compose -exec \
    sed -i -E 's#/(nix/store/[^/]+/share/X11/locale|usr/share/X11/locale)/#%S/#g' {} +

# Include libraries loaded at runtime by winit, wgpu, and clipboard backends.
# ldd alone cannot discover these. Bundle their transitive imports too.
CLIENT_LIBRARIES=(
    libwayland-client.so.0 libwayland-cursor.so.0 libwayland-egl.so.1
    libxkbcommon.so.0 libxkbcommon-x11.so.0 libX11.so.6 libX11-xcb.so.1
    libXcursor.so.1 libXi.so.6 libXrandr.so.2 libXrender.so.1 libXext.so.6
    libXfixes.so.3 libXinerama.so.1 libxcb.so.1 libvulkan.so.1 libGL.so.1
    libEGL.so.1 libz.so.1
)
IFS=: read -ra SEARCH_DIRS <<< "${LD_LIBRARY_PATH:-}"
SEARCH_DIRS+=(/lib/x86_64-linux-gnu /usr/lib/x86_64-linux-gnu /lib64 /usr/lib64 /lib /usr/lib)

resolve_library() {
    local name=$1 directory
    for directory in "${SEARCH_DIRS[@]}"; do
        if [[ -f "$directory/$name" ]]; then
            readlink -f "$directory/$name"
            return
        fi
    done
    echo "Required runtime library not found: $name" >&2
    return 1
}

declare -A COPIED
bundle_library() {
    local source name directory
    source=$(readlink -f "$1")
    name=$(patchelf --print-soname "$source")
    name=${name:-${source##*/}}
    if [[ -n ${COPIED[$name]:-} ]]; then
        if [[ ${COPIED[$name]} != "$source" ]]; then
            echo "Conflicting runtime libraries for $name: ${COPIED[$name]} and $source" >&2
            return 1
        fi
        return
    fi
    COPIED[$name]=$source
    case "$name" in
        ld-linux*|libc.so.*|libm.so.*|libdl.so.*|libpthread.so.*|librt.so.*|libresolv.so.*|libutil.so.*|libanl.so.*|libnss_*.so.*)
            directory="$APPDIR/usr/lib/glibc" ;;
        *) directory="$APPDIR/usr/lib" ;;
    esac
    cp -L "$source" "$directory/$name"
    chmod u+w "$directory/$name"
    printf '%s\t%s\n' "$name" "$source" >> "$APPDIR/usr/share/doc/valheim-runtime/libraries.tsv"
}

bundle_dependencies() {
    local source=$1 dependencies dependency
    dependencies=$(ldd "$source")
    if [[ "$dependencies" == *'not found'* ]]; then
        printf 'Unresolved dependencies for %s:\n%s\n' "$source" "$dependencies" >&2
        return 1
    fi
    while IFS= read -r dependency; do
        [[ -z $dependency ]] || bundle_library "$dependency"
    done < <(awk '/=> \// {print $3} /^[[:space:]]*\// && !/=>/ {print $1}' <<< "$dependencies")
}

BINARY="$ROOT_DIR/target/release/valheim-modpack-studio"
INTERPRETER=$(readlink -f "$(patchelf --print-interpreter "$BINARY")")
bundle_library "$INTERPRETER"
bundle_dependencies "$BINARY"
for library in "${CLIENT_LIBRARIES[@]}"; do
    source=$(resolve_library "$library")
    bundle_library "$source"
    bundle_dependencies "$source"
done
# Compatibility SONAMEs may still be needed by host graphics libraries even
# though glibc moved these APIs into libc. Keep them from the same glibc build.
for name in libdl.so.2 libpthread.so.0 librt.so.1 libresolv.so.2 libutil.so.1 libanl.so.1; do
    if [[ -f "${INTERPRETER%/*}/$name" ]]; then
        bundle_library "${INTERPRETER%/*}/$name"
    fi
done

# Remove development-machine RPATHs. Every runtime import must be relocatable.
for library in "$APPDIR"/usr/lib/*.so* "$APPDIR"/usr/lib/glibc/*.so*; do
    [[ ${library##*/} == ld-linux* ]] && continue
    # shellcheck disable=SC2016 # Expanded by the ELF loader, not the shell.
    patchelf --set-rpath '$ORIGIN:$ORIGIN/glibc' "$library"
done
# shellcheck disable=SC2016 # Expanded by the ELF loader, not the shell.
patchelf --set-rpath '$ORIGIN/../lib:$ORIGIN/../lib/glibc' "$APPDIR/usr/bin/valheim-modpack-studio"
patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 "$APPDIR/usr/bin/valheim-modpack-studio"

# Include distribution copyright notices when packaging on Debian/Ubuntu.
if command -v dpkg-query > /dev/null; then
    for package in libc6 libgcc-s1 zlib1g libwayland-client0 libwayland-cursor0 libwayland-egl1 \
        libxkbcommon0 libxkbcommon-x11-0 libx11-6 libx11-xcb1 libxcursor1 libxi6 \
        libxrandr2 libxrender1 libxext6 libxfixes3 libxinerama1 libxcb1 libvulkan1 \
        libgl1 libegl1 libglx0 libglvnd0 libglvnd-core-dev libffi8 libxau6 libxdmcp6 libbsd0 libmd0 xkb-data libx11-data; do
        if [[ -f /usr/share/doc/$package/copyright ]]; then
            cp "/usr/share/doc/$package/copyright" "$APPDIR/usr/share/doc/valheim-runtime/$package.copyright"
        fi
    done
    cp -L /usr/share/common-licenses/LGPL-2.1 "$APPDIR/usr/share/doc/valheim-runtime/"
fi

RUNTIME="$ROOT_DIR/build/runtime-x86_64"
RUNTIME_SHA256=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
if ! printf '%s  %s\n' "$RUNTIME_SHA256" "$RUNTIME" | sha256sum --check --status; then
    curl -fL --retry 3 -o "$RUNTIME.download" \
        https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64
    printf '%s  %s\n' "$RUNTIME_SHA256" "$RUNTIME.download" | sha256sum --check --strict
    mv "$RUNTIME.download" "$RUNTIME"
fi

SQUASHFS_TMP=$(mktemp "$ROOT_DIR/build/appdir.XXXXXX.squashfs")
OUTPUT_APPIMAGE="$DIST_DIR/ValheimManager-${VERSION}.AppImage"
TMP_APPIMAGE="$OUTPUT_APPIMAGE.tmp.$$"
trap 'rm -f "$SQUASHFS_TMP" "$TMP_APPIMAGE"' EXIT
mksquashfs "$APPDIR" "$SQUASHFS_TMP" -root-owned -noappend -comp zstd
cat "$RUNTIME" "$SQUASHFS_TMP" > "$TMP_APPIMAGE"
chmod +x "$TMP_APPIMAGE"
mv "$TMP_APPIMAGE" "$OUTPUT_APPIMAGE"
bash scripts/check-appimage.sh "$OUTPUT_APPIMAGE"
printf '\nCreated %s (%s)\n' "$OUTPUT_APPIMAGE" "$(du -h "$OUTPUT_APPIMAGE" | cut -f1)"
