#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
BUILD_DIR="$SAFE_ROOT/target/capi-decompression"
RUNTIME_DIR="$BUILD_DIR/runtime"

mkdir -p "$BUILD_DIR" "$RUNTIME_DIR"

resolve_upstream_lib() {
    if [[ -n ${SAFE_UPSTREAM_LIB:-} ]]; then
        printf '%s\n' "$SAFE_UPSTREAM_LIB"
        return 0
    fi

    local candidate
    candidate=$(ldconfig -p 2>/dev/null | awk '/libzstd\.so\.1 / {print $NF; exit}')
    if [[ -z $candidate || ! -e $candidate ]]; then
        echo "unable to resolve upstream libzstd.so.1" >&2
        exit 1
    fi

    printf '%s\n' "$candidate"
}

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

export SAFE_UPSTREAM_LIB
SAFE_UPSTREAM_LIB=$(resolve_upstream_lib)
export LD_LIBRARY_PATH="$RUNTIME_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

"$BUILD_DIR/decompress_smoke" "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests/golden-decompression/rle-first-block.zst"
"$BUILD_DIR/frame_probe" \
    "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests/golden-decompression/rle-first-block.zst" \
    "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests/golden-decompression/empty-block.zst"
"$BUILD_DIR/legacy_decode"
