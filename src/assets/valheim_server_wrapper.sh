#!/bin/bash
BASEDIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Doorstop environment variables
export DOORSTOP_ENABLED=1
export DOORSTOP_ENABLE=TRUE
export DOORSTOP_TARGET_ASSEMBLY="$BASEDIR/BepInEx/core/BepInEx.Preloader.dll"
export DOORSTOP_INVOKE_DLL_PATH="$BASEDIR/BepInEx/core/BepInEx.Preloader.dll"

# Library paths and Preload (using absolute paths so ld.so never fails)
export LD_LIBRARY_PATH="$BASEDIR/doorstop_libs:$BASEDIR/linux64:$LD_LIBRARY_PATH"
export LD_PRELOAD="$BASEDIR/doorstop_libs/libdoorstop_x64.so:$LD_PRELOAD"
export SteamAppId=892970

# Enforce simulation distance (6 zones = 384m radius) without overriding panel flags
EXTRA_ARGS="-simulationdistance 6"

# Log startup arguments for troubleshooting
echo "[$(date)] Server launch arguments:" > "$BASEDIR/startup_args.log" 2>/dev/null || true
echo "$@ $EXTRA_ARGS" >> "$BASEDIR/startup_args.log" 2>/dev/null || true

if [ -f "$BASEDIR/valheim_server.x86_64.bin" ]; then
    exec "$BASEDIR/valheim_server.x86_64.bin" "$@" $EXTRA_ARGS
elif [ -f "$BASEDIR/valheim_server" ]; then
    exec "$BASEDIR/valheim_server" "$@" $EXTRA_ARGS
else
    echo "ERROR: Neither valheim_server.x86_64.bin nor valheim_server found!" >&2
    exit 1
fi
