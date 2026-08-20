#!/bin/sh

APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BIN_PATH="$APP_DIR/bin/brick_reader"
LOG_PATH="$APP_DIR/log.txt"
CURSOR_PATH="/sys/class/graphics/fbcon/cursor_blink"

cd "$APP_DIR" || exit 1

# Redirect the complete launcher lifecycle, including dynamic-loader and shell
# errors, so a failed start is diagnosable from the SD card.
exec > "$LOG_PATH" 2>&1

cleanup() {
    if [ -w "$CURSOR_PATH" ]; then
        echo 1 > "$CURSOR_PATH"
    fi
    sync
}
trap cleanup EXIT HUP INT TERM

echo "tbb1.3.1 launcher started: $(date)"
echo "app_dir=$APP_DIR"
echo "kernel=$(uname -a)"
echo "framebuffer=$([ -e /dev/fb0 ] && echo present || echo missing)"

if [ ! -f "$BIN_PATH" ]; then
    echo "ERROR: missing executable: $BIN_PATH"
    exit 127
fi

chmod +x "$BIN_PATH" || {
    echo "ERROR: cannot make executable: $BIN_PATH"
    exit 126
}

if [ -w "$CURSOR_PATH" ]; then
    echo 0 > "$CURSOR_PATH"
fi

"$BIN_PATH"
STATUS=$?
echo "tbb1.3.1 exited with status $STATUS"
exit "$STATUS"
