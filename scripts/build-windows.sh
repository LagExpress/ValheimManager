#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "🪟 Building Windows executable (x86_64-pc-windows-gnu)..."

# MinGW toolchain variables
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

# Setup linker shim directory for Linux case sensitivity (-lShell32) and pthread linking (-l:libpthread.a)
SHIM_DIR="$ROOT_DIR/build/mingw-lib-shim"
mkdir -p "$SHIM_DIR"

if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
  echo "Error: MinGW compiler is missing. Enter the configured devenv shell first." >&2
  exit 1
fi

MINGW_SHELL32=$(x86_64-w64-mingw32-gcc -print-file-name=libshell32.a)
if [ -f "$MINGW_SHELL32" ]; then
  ln -sf "$MINGW_SHELL32" "$SHIM_DIR/libShell32.a"
fi

MINGW_PTHREAD="${MINGW_PTHREAD_LIB:-$(x86_64-w64-mingw32-gcc -print-file-name=libpthread.a)}"
if [ -f "$MINGW_PTHREAD" ]; then
  ln -sf "$MINGW_PTHREAD" "$SHIM_DIR/libpthread.a"
fi

export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-L $SHIM_DIR"

cargo build --target x86_64-pc-windows-gnu --release

mkdir -p "$ROOT_DIR/dist"
cp -f "$ROOT_DIR/target/x86_64-pc-windows-gnu/release/valheim-modpack-studio.exe" "$ROOT_DIR/dist/ValheimModpackStudio-0.1.4-x86_64.exe"

echo ""
echo "✔ Successfully generated Windows executable: dist/ValheimModpackStudio-0.1.4-x86_64.exe ($(du -h "$ROOT_DIR/dist/ValheimModpackStudio-0.1.4-x86_64.exe" | cut -f1))"
