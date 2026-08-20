#!/usr/bin/env bash
# Build a self-contained ARM64 package for TrimUI Brick stock/TrimUI OS.
set -euo pipefail

PROJECT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
TARGET_TRIPLE="aarch64-unknown-linux-musl"
PACKAGE_DIR="$PROJECT_DIR/package/Apps/BrickReader"
OUTPUT_BIN="$PROJECT_DIR/target/$TARGET_TRIPLE/release/brick_reader"

cd "$PROJECT_DIR"

for required_tool in cargo rustc rustup readelf python3 file sha256sum install; do
    if ! command -v "$required_tool" >/dev/null 2>&1; then
        echo "ERROR: required tool not found: $required_tool" >&2
        exit 1
    fi
done

echo "==> [1/5] Installing Rust target: $TARGET_TRIPLE"
rustup target add "$TARGET_TRIPLE"

RUST_SYSROOT="$(rustc --print sysroot)"
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
RUST_LLD_PATH="$RUST_SYSROOT/lib/rustlib/$HOST_TRIPLE/bin/rust-lld"

if [ ! -x "$RUST_LLD_PATH" ]; then
    echo "ERROR: rust-lld not found at $RUST_LLD_PATH" >&2
    exit 1
fi

echo "==> [2/5] Running host tests"
cargo test --all-targets

echo "==> [3/5] Building static ARM64 release binary"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$RUST_LLD_PATH"
cargo build --release --target "$TARGET_TRIPLE"

echo "==> [4/5] Verifying TrimUI-compatible ELF"
if ! readelf -h "$OUTPUT_BIN" | grep -q 'Machine:.*AArch64'; then
    echo "ERROR: output is not an AArch64 executable" >&2
    exit 1
fi

if readelf -l "$OUTPUT_BIN" | grep -q 'INTERP'; then
    echo "ERROR: output contains a dynamic interpreter" >&2
    exit 1
fi

if readelf -d "$OUTPUT_BIN" 2>/dev/null | grep -q 'NEEDED'; then
    echo "ERROR: output still depends on shared libraries" >&2
    exit 1
fi

echo "==> [5/5] Assembling package"
install -d "$PACKAGE_DIR/bin"
install -m 0755 "$OUTPUT_BIN" "$PACKAGE_DIR/bin/brick_reader"
chmod 0755 "$PACKAGE_DIR/launch.sh"

python3 -m json.tool "$PACKAGE_DIR/config.json" >/dev/null
sh -n "$PACKAGE_DIR/launch.sh"

echo
echo "Build complete: $PACKAGE_DIR"
file "$PACKAGE_DIR/bin/brick_reader"
sha256sum "$PACKAGE_DIR/bin/brick_reader"
echo "Copy the BrickReader directory into F:\\Apps, then refresh Apps on the device."
