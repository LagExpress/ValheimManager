#!/usr/bin/env bash
set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "🪟 Building Windows executable (x86_64-pc-windows-gnu)..."

export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"

# MinGW toolchain variables
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

# Setup linker shim directory for Linux case sensitivity (-lShell32) and pthread linking (-l:libpthread.a)
SHIM_DIR="$ROOT_DIR/build/mingw-lib-shim"
mkdir -p "$SHIM_DIR"

MINGW_SHELL32=$(x86_64-w64-mingw32-gcc -print-file-name=libshell32.a)
if [ -f "$MINGW_SHELL32" ]; then
  ln -sf "$MINGW_SHELL32" "$SHIM_DIR/libShell32.a"
fi

if [ -f "/nix/store/mwjcay95pszi6r44ks5faf1h8b2zfh1q-mingw_w64-pthreads-x86_64-w64-mingw32-13.0.0/lib/libpthread.a" ]; then
  ln -sf "/nix/store/mwjcay95pszi6r44ks5faf1h8b2zfh1q-mingw_w64-pthreads-x86_64-w64-mingw32-13.0.0/lib/libpthread.a" "$SHIM_DIR/libpthread.a"
fi

export RUSTFLAGS="-L $SHIM_DIR"

cargo build --target x86_64-pc-windows-gnu --release

mkdir -p "$ROOT_DIR/dist"
cp "$ROOT_DIR/target/x86_64-pc-windows-gnu/release/valheim-modpack-studio.exe" "$ROOT_DIR/dist/ValheimModpackStudio-x86_64.exe"

echo ""
echo "✔ Successfully generated Windows executable: dist/ValheimModpackStudio-x86_64.exe ($(du -h "$ROOT_DIR/dist/ValheimModpackStudio-x86_64.exe" | cut -f1))"
