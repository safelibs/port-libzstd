#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
BUILD_DIR="$SAFE_ROOT/target/capi-decompression"
RUNTIME_DIR="$BUILD_DIR/runtime"

mkdir -p "$BUILD_DIR" "$RUNTIME_DIR"

cargo build --manifest-path "$SAFE_ROOT/Cargo.toml" --release
ln -sf "$SAFE_ROOT/target/release/libzstd.so" "$RUNTIME_DIR/libzstd.so.1"

CC_BIN=${CC:-cc}
CFLAGS=(
    -std=c11
    -Wall
    -Wextra
    -Werror
    -Wno-deprecated-declarations
    -I"$SAFE_ROOT/include"
    -L"$SAFE_ROOT/target/release"
    "-Wl,-rpath,$RUNTIME_DIR"
)

"$CC_BIN" "${CFLAGS[@]}" "$SAFE_ROOT/tests/capi/decompress_smoke.c" -o "$BUILD_DIR/decompress_smoke" -lzstd
"$CC_BIN" "${CFLAGS[@]}" "$SAFE_ROOT/tests/capi/frame_probe.c" -o "$BUILD_DIR/frame_probe" -lzstd
"$CC_BIN" "${CFLAGS[@]}" "$SAFE_ROOT/tests/capi/legacy_decode.c" -o "$BUILD_DIR/legacy_decode" -lzstd

export LD_LIBRARY_PATH="$RUNTIME_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

"$BUILD_DIR/decompress_smoke" "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests/golden-decompression/rle-first-block.zst"
head -c 31 /dev/zero > "$BUILD_DIR/tiny-single-segment.bin"
zstd -q -f "$BUILD_DIR/tiny-single-segment.bin" -o "$BUILD_DIR/tiny-single-segment.zst"
"$BUILD_DIR/decompress_smoke" "$BUILD_DIR/tiny-single-segment.zst"
"$BUILD_DIR/frame_probe" \
    "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests/golden-decompression/rle-first-block.zst" \
    "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests/golden-decompression/empty-block.zst"
"$BUILD_DIR/legacy_decode"
