#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
BUILD_DIR="$SAFE_ROOT/target/advanced-mt"
WORK_DIR="$BUILD_DIR/work"
RUNTIME_DIR="$BUILD_DIR/runtime"
UPSTREAM_ROOT="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"
DICT_FIXTURE="$UPSTREAM_ROOT/tests/golden-dictionaries/http-dict-missing-symbols"

mkdir -p "$BUILD_DIR" "$WORK_DIR" "$RUNTIME_DIR"

cargo rustc --manifest-path "$SAFE_ROOT/Cargo.toml" --release --crate-type cdylib
ln -sf "$SAFE_ROOT/target/release/libzstd.so" "$RUNTIME_DIR/libzstd.so.1"

CC_BIN=${CC:-cc}
CFLAGS=(
    -std=c11
    -Wall
    -Wextra
    -Werror
    -D_POSIX_C_SOURCE=200809L
    -Wno-deprecated-declarations
    -Wno-sign-compare
    -Wno-unused-function
    -Wno-unused-parameter
    -I"$SAFE_ROOT/include"
    -L"$SAFE_ROOT/target/release"
    "-Wl,-rpath,$RUNTIME_DIR"
)

compile_c() {
    local src=$1
    local out=$2
    shift 2
    "$CC_BIN" "${CFLAGS[@]}" "$@" "$src" -o "$out" -lzstd
}

compile_c "$SAFE_ROOT/tests/capi/zstream_driver.c" "$BUILD_DIR/zstream_driver"
compile_c "$SAFE_ROOT/tests/capi/thread_pool_driver.c" "$BUILD_DIR/thread_pool_driver"
compile_c "$SAFE_ROOT/tests/capi/sequence_api_driver.c" "$BUILD_DIR/sequence_api_driver"
compile_c "$SAFE_ROOT/tests/capi/dict_builder_driver.c" "$BUILD_DIR/dict_builder_driver"
compile_c "$UPSTREAM_ROOT/tests/zstreamtest.c" "$BUILD_DIR/upstream_zstreamtest"
compile_c "$UPSTREAM_ROOT/tests/poolTests.c" "$BUILD_DIR/upstream_poolTests" -pthread
compile_c "$UPSTREAM_ROOT/examples/streaming_memory_usage.c" "$BUILD_DIR/streaming_memory_usage"
compile_c "$UPSTREAM_ROOT/examples/streaming_compression_thread_pool.c" \
    "$BUILD_DIR/streaming_compression_thread_pool" \
    -DZSTD_STATIC_LINKING_ONLY \
    -pthread

export LD_LIBRARY_PATH="$RUNTIME_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

python3 - "$WORK_DIR" <<'PY'
import sys
from pathlib import Path

work = Path(sys.argv[1])
work.mkdir(parents=True, exist_ok=True)

def sample_bytes(size: int, seed: int) -> bytes:
    fragments = [
        b'{"tenant":"alpha","region":"west","kind":"session","payload":"',
        b'{"tenant":"beta","region":"east","kind":"metric","payload":"',
        b'{"tenant":"gamma","region":"north","kind":"record","payload":"',
    ]
    alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_"
    out = bytearray()
    state = seed | 1
    while len(out) < size:
        state ^= (state << 13) & 0xFFFFFFFF
        state ^= (state >> 17) & 0xFFFFFFFF
        state ^= (state << 5) & 0xFFFFFFFF
        fragment = fragments[state % len(fragments)]
        out.extend(fragment[: max(0, min(len(fragment), size - len(out)))])
        for _ in range(96):
            if len(out) >= size:
                break
            state ^= (state << 13) & 0xFFFFFFFF
            state ^= (state >> 17) & 0xFFFFFFFF
            state ^= (state << 5) & 0xFFFFFFFF
            out.append(alphabet[state % len(alphabet)])
        if len(out) < size:
            out.extend(b'"}\n'[: size - len(out)])
    return bytes(out[:size])

(work / "input-one.txt").write_bytes(sample_bytes(160 * 1024 + 17, 0x12345678))
(work / "input-two.txt").write_bytes(sample_bytes(128 * 1024 + 29, 0x89ABCDEF))
PY

"$BUILD_DIR/dict_builder_driver"
"$BUILD_DIR/sequence_api_driver"
"$BUILD_DIR/thread_pool_driver"
"$BUILD_DIR/zstream_driver" "$DICT_FIXTURE"
"$BUILD_DIR/upstream_zstreamtest" -t2
"$BUILD_DIR/upstream_poolTests"
"$BUILD_DIR/streaming_memory_usage"
"$BUILD_DIR/streaming_compression_thread_pool" 2 3 \
    "$WORK_DIR/input-one.txt" \
    "$WORK_DIR/input-two.txt"
