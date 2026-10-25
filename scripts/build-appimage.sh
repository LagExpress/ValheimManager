#!/usr/bin/env bash
set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "🦀 Building Valheim Modpack Studio (Release binary)..."
cargo build --release

APPDIR="$ROOT_DIR/build/AppDir"
DIST_DIR="$ROOT_DIR/dist"
mkdir -p "$DIST_DIR"
rm -rf "$APPDIR"

echo "📦 Creating AppDir layout..."
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/lib"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

# Copy binary
cp "$ROOT_DIR/target/release/valheim-modpack-studio" "$APPDIR/usr/bin/"
cp "$ROOT_DIR/target/release/valheim-modpack-studio" "$APPDIR/"

# Copy desktop file
cp "$ROOT_DIR/valheim-modpack-studio.desktop" "$APPDIR/usr/share/applications/"
cp "$ROOT_DIR/valheim-modpack-studio.desktop" "$APPDIR/"

# Copy icon
cp "$ROOT_DIR/src/assets/icon.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/valheim-modpack-studio.png"
cp "$ROOT_DIR/src/assets/icon.png" "$APPDIR/valheim-modpack-studio.png"
cp "$ROOT_DIR/src/assets/icon.png" "$APPDIR/.DirIcon"

# Bundle common Wayland/X11 runtime libraries for maximum portability
echo "📚 Bundling runtime client libraries..."
LIB_PATTERNS=(
  "libwayland-client"
  "libwayland-cursor"
  "libwayland-egl"
  "libxkbcommon"
  "libxkbcommon-x11"
  "libX11"
  "libX11-xcb"
  "libXcursor"
  "libXi"
  "libXrandr"
  "libXrender"
  "libxcb"
)

# Search LD_LIBRARY_PATH and standard Nix paths
IFS=':' read -ra DIRS <<< "${LD_LIBRARY_PATH:-}"
for p in "${LIB_PATTERNS[@]}"; do
  for d in "${DIRS[@]}"; do
    if [ -d "$d" ]; then
      find "$d" -maxdepth 1 -name "${p}.so*" -exec cp -u {} "$APPDIR/usr/lib/" \; 2>/dev/null || true
    fi
  done
done

# Create AppRun script
cat <<'EOF' > "$APPDIR/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS}"
exec "${HERE}/usr/bin/valheim-modpack-studio" "$@"
EOF
chmod +x "$APPDIR/AppRun"

echo "🛠️ Packaging AppImage using SquashFS (zstd compression) + Type 2 Runtime..."
RUNTIME="$ROOT_DIR/build/runtime-x86_64"
if [ ! -f "$RUNTIME" ]; then
    echo "Downloading AppImage Type 2 runtime..."
    curl -sL -o "$RUNTIME" "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64"
fi

SQUASHFS_TMP="$ROOT_DIR/build/appdir-zstd.squashfs"
rm -f "$SQUASHFS_TMP"

# Use zstd compression (supported out of the box by Type 2 runtime)
mksquashfs "$APPDIR" "$SQUASHFS_TMP" -root-owned -noappend -comp zstd

OUTPUT_APPIMAGE="$DIST_DIR/ValheimModpackStudio-x86_64.AppImage"
cat "$RUNTIME" "$SQUASHFS_TMP" > "$OUTPUT_APPIMAGE"
chmod +x "$OUTPUT_APPIMAGE"
rm -f "$SQUASHFS_TMP"

echo ""
echo "✔ Successfully generated AppImage: $OUTPUT_APPIMAGE ($(du -h "$OUTPUT_APPIMAGE" | cut -f1))"
