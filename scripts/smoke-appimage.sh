#!/usr/bin/env bash
set -euo pipefail

IMAGE=$(readlink -f "${1:?Usage: smoke-appimage.sh <AppImage>}")
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$ROOT_DIR/target"
WORK=$(mktemp -d "$ROOT_DIR/target/appimage-smoke.XXXXXX")
APP_PID=''
XVFB_PID=''
# shellcheck disable=SC2329 # Invoked by the EXIT trap.
cleanup() {
    if [[ -n $APP_PID ]]; then
        kill -- "-$APP_PID" 2>/dev/null || true
        wait "$APP_PID" 2>/dev/null || true
    fi
    if [[ -n $XVFB_PID ]]; then
        kill "$XVFB_PID" 2>/dev/null || true
        wait "$XVFB_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT
mkdir -p "$WORK"/{home,config,data,cache,runtime}
chmod 700 "$WORK/runtime"
Xvfb -displayfd 3 -screen 0 800x600x24 -nolisten tcp 3> "$WORK/display" > "$WORK/xvfb.log" 2>&1 &
XVFB_PID=$!
for _ in {1..40}; do
    [[ -s $WORK/display ]] && break
    kill -0 "$XVFB_PID"
    sleep 0.25
done
DISPLAY=":$(cat "$WORK/display")"
export DISPLAY
setsid env -u LD_LIBRARY_PATH -u LD_PRELOAD -u WAYLAND_DISPLAY \
    HOME="$WORK/home" XDG_CONFIG_HOME="$WORK/config" XDG_DATA_HOME="$WORK/data" \
    XDG_CACHE_HOME="$WORK/cache" XDG_RUNTIME_DIR="$WORK/runtime" XDG_SESSION_TYPE=x11 \
    LIBGL_ALWAYS_SOFTWARE=1 "$IMAGE" --appimage-extract-and-run > "$WORK/app.log" 2>&1 &
APP_PID=$!
for _ in {1..60}; do
    if ! kill -0 "$APP_PID" 2>/dev/null; then
        cat "$WORK/app.log" >&2
        exit 1
    fi
    if xwininfo -name 'Valheim Modpack Studio' 2>/dev/null | grep -q 'Map State: IsViewable'; then
        sleep 2
        kill -0 "$APP_PID"
        printf 'Packaged application opened a mapped window. Logs: %s\n' "$WORK"
        exit 0
    fi
    sleep 0.5
done
cat "$WORK/app.log" >&2
echo "Packaged application did not open its window. Logs: $WORK" >&2
exit 1
