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

# Bundle common Wayland/X11/Vulkan runtime libraries for maximum portability
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
  "libvulkan"
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

# Create robust AppRun script supporting standard Linux and NixOS graphics stacks
cat <<'EOF' > "$APPDIR/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"

# Add NixOS GPU driver paths if present, alongside AppDir libraries
NIX_GL_LIB="/run/opengl-driver/lib"
NIX_SW_LIB="/run/current-system/sw/lib"
NIX_GL_SHARE="/run/opengl-driver/share"
NIX_SW_SHARE="/run/current-system/sw/share"

export LD_LIBRARY_PATH="${HERE}/usr/lib:${NIX_GL_LIB}:${NIX_SW_LIB}:${LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${NIX_GL_SHARE}:${NIX_SW_SHARE}:${XDG_DATA_DIRS}"

# Auto-set VK_DRIVER_FILES on NixOS so Vulkan can locate NVIDIA / Mesa ICD descriptors
if [ -z "${VK_DRIVER_FILES}" ] && [ -z "${VK_ICD_FILENAMES}" ]; then
  if [ -d "/run/opengl-driver/share/vulkan/icd.d" ]; then
    ICD_LIST=""
    for icd in /run/opengl-driver/share/vulkan/icd.d/*.json; do
      if [ -f "$icd" ]; then
        ICD_LIST="${ICD_LIST}${ICD_LIST:+:}${icd}"
      fi
    done
    if [ -n "$ICD_LIST" ]; then
      export VK_DRIVER_FILES="$ICD_LIST"
    fi
  fi
fi

# Ensure desktop entry and icons are registered with host compositor (COSMIC / GNOME / KDE)
if [ -n "$APPIMAGE" ]; then
  LOCAL_APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
  LOCAL_ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
  mkdir -p "$LOCAL_APPS" "$LOCAL_ICONS" 2>/dev/null || true
  cp -u "${HERE}/usr/share/icons/hicolor/256x256/apps/valheim-modpack-studio.png" "$LOCAL_ICONS/" 2>/dev/null || true
  sed "s|Exec=valheim-modpack-studio|Exec=\"${APPIMAGE}\"|g" "${HERE}/usr/share/applications/valheim-modpack-studio.desktop" > "$LOCAL_APPS/valheim-modpack-studio.desktop" 2>/dev/null || true
fi

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

# Use zstd compression
mksquashfs "$APPDIR" "$SQUASHFS_TMP" -root-owned -noappend -comp zstd

OUTPUT_APPIMAGE="$DIST_DIR/ValheimModpackStudio-x86_64.AppImage"
cat "$RUNTIME" "$SQUASHFS_TMP" > "$OUTPUT_APPIMAGE"
chmod +x "$OUTPUT_APPIMAGE"
rm -f "$SQUASHFS_TMP"

echo ""
echo "✔ Successfully generated AppImage: $OUTPUT_APPIMAGE ($(du -h "$OUTPUT_APPIMAGE" | cut -f1))"
