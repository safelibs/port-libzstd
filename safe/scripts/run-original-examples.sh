#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env

EXAMPLES_DIR="$ORIGINAL_ROOT/examples"
STAMP_FILE=$(phase6_stamp_path run-original-examples)

if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$HELPER_LIB_ROOT/libzstd.a" \
    "$HELPER_LIB_ROOT/libzstd.so.1.5.5" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$EXAMPLES_DIR"
then
    phase6_log "upstream example coverage already fresh; skipping rerun"
    exit 0
fi

phase6_log "building compression-focused upstream examples against the safe helper lib root"
make -C "$EXAMPLES_DIR" clean LIBDIR="$HELPER_LIB_ROOT"
make -C "$EXAMPLES_DIR" \
    simple_compression \
    multiple_simple_compression \
    multiple_streaming_compression \
    dictionary_compression \
    LIBDIR="$HELPER_LIB_ROOT"
phase6_assert_uses_safe_lib \
    "$EXAMPLES_DIR/simple_compression" \
    "$EXAMPLES_DIR/multiple_simple_compression" \
    "$EXAMPLES_DIR/multiple_streaming_compression" \
    "$EXAMPLES_DIR/dictionary_compression"

phase6_log "running compression-focused upstream examples against the safe helper lib root"
(
    cd "$EXAMPLES_DIR"
    cp README.md tmp
    cp Makefile tmp2
    ./simple_compression tmp
    ./multiple_simple_compression *.c
    ./multiple_streaming_compression *.c
    ./dictionary_compression tmp2 tmp README.md
)

make -C "$EXAMPLES_DIR" clean LIBDIR="$HELPER_LIB_ROOT"
phase6_touch_stamp "$STAMP_FILE"
