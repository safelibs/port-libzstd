#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
BUILD_DIR="$SAFE_ROOT/target/capi-roundtrip"
WORK_DIR="$BUILD_DIR/work"
RUNTIME_DIR="$BUILD_DIR/runtime"
CARGO_TARGET_DIR="$BUILD_DIR/cargo-target"
SAFE_RELEASE_DIR="$CARGO_TARGET_DIR/release"
EXAMPLES_DIR="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/examples"
DICT_FIXTURE="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests/golden-dictionaries/http-dict-missing-symbols"

mkdir -p "$BUILD_DIR" "$WORK_DIR" "$RUNTIME_DIR" "$CARGO_TARGET_DIR"

CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo build --manifest-path "$SAFE_ROOT/Cargo.toml" --release
ln -sf "$SAFE_RELEASE_DIR/libzstd.so" "$RUNTIME_DIR/libzstd.so.1"

CC_BIN=${CC:-cc}
CFLAGS=(
    -std=c11
    -Wall
    -Wextra
    -Werror
    -D_POSIX_C_SOURCE=200809L
    -Wno-deprecated-declarations
    -Wno-unused-function
    -Wno-unused-parameter
    -include
    sys/types.h
    -I"$SAFE_ROOT/include"
    -L"$SAFE_RELEASE_DIR"
    "-Wl,-rpath,$RUNTIME_DIR"
)

compile_c() {
    local src=$1
    local out=$2
    "$CC_BIN" "${CFLAGS[@]}" "$src" -o "$out" -lzstd
}

compile_c "$SAFE_ROOT/tests/capi/roundtrip_smoke.c" "$BUILD_DIR/roundtrip_smoke"
compile_c "$SAFE_ROOT/tests/capi/bigdict_driver.c" "$BUILD_DIR/bigdict_driver"
compile_c "$SAFE_ROOT/tests/capi/invalid_dictionaries_driver.c" "$BUILD_DIR/invalid_dictionaries_driver"
compile_c "$SAFE_ROOT/tests/capi/zstream_driver.c" "$BUILD_DIR/zstream_driver"
compile_c "$SAFE_ROOT/tests/capi/paramgrill_driver.c" "$BUILD_DIR/paramgrill_driver"
compile_c "$SAFE_ROOT/tests/capi/external_matchfinder_driver.c" "$BUILD_DIR/external_matchfinder_driver"

compile_c "$EXAMPLES_DIR/simple_compression.c" "$BUILD_DIR/simple_compression"
compile_c "$EXAMPLES_DIR/multiple_simple_compression.c" "$BUILD_DIR/multiple_simple_compression"
compile_c "$EXAMPLES_DIR/multiple_streaming_compression.c" "$BUILD_DIR/multiple_streaming_compression"
compile_c "$EXAMPLES_DIR/dictionary_compression.c" "$BUILD_DIR/dictionary_compression"
compile_c "$EXAMPLES_DIR/dictionary_decompression.c" "$BUILD_DIR/dictionary_decompression"

STREAMING_WRAPPER="$BUILD_DIR/streaming_compression_wrapper.c"
cat > "$STREAMING_WRAPPER" <<EOF
#include <stddef.h>
#include "zstd.h"

size_t safe_single_thread_set_parameter(ZSTD_CCtx* cctx, ZSTD_cParameter param, int value)
{
    if (param == ZSTD_c_nbWorkers && value > 0) {
        value = 0;
    }
    return ZSTD_CCtx_setParameter(cctx, param, value);
}

#define ZSTD_CCtx_setParameter safe_single_thread_set_parameter
#include "$EXAMPLES_DIR/streaming_compression.c"
EOF
compile_c "$STREAMING_WRAPPER" "$BUILD_DIR/streaming_compression"

export LD_LIBRARY_PATH="$RUNTIME_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

WORK_DIR="$WORK_DIR" python - <<'PY'
import os
from pathlib import Path

work = Path(os.environ["WORK_DIR"])
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

(work / "input-one.txt").write_bytes(sample_bytes(128 * 1024 + 37, 0x12345678))
(work / "input-two.txt").write_bytes(sample_bytes(96 * 1024 + 11, 0xABCDEF01))
(work / "input-three.txt").write_bytes(sample_bytes(160 * 1024 + 73, 0xDEADBEEF))
PY

"$BUILD_DIR/roundtrip_smoke" "$DICT_FIXTURE"
"$BUILD_DIR/bigdict_driver"
"$BUILD_DIR/invalid_dictionaries_driver"
"$BUILD_DIR/zstream_driver" "$DICT_FIXTURE"
"$BUILD_DIR/paramgrill_driver"
"$BUILD_DIR/external_matchfinder_driver"

"$BUILD_DIR/simple_compression" "$WORK_DIR/input-one.txt"
"$BUILD_DIR/multiple_simple_compression" "$WORK_DIR/input-one.txt" "$WORK_DIR/input-two.txt"
"$BUILD_DIR/streaming_compression" "$WORK_DIR/input-one.txt" 3 1
"$BUILD_DIR/multiple_streaming_compression" "$WORK_DIR/input-one.txt" "$WORK_DIR/input-three.txt"
"$BUILD_DIR/dictionary_compression" "$WORK_DIR/input-one.txt" "$WORK_DIR/input-two.txt" "$DICT_FIXTURE"
"$BUILD_DIR/dictionary_decompression" \
    "$WORK_DIR/input-one.txt.zst" \
    "$WORK_DIR/input-two.txt.zst" \
    "$DICT_FIXTURE"
