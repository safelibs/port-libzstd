#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env

phase6_log "building and running contrib/seekable_format tests against the safe library"
make -C "$ORIGINAL_ROOT/contrib/seekable_format/tests" clean ZSTDLIB_PATH="$HELPER_LIB_ROOT"
SEEKABLE_OBJS="../zstdseek_compress.c ../zstdseek_decompress.c $HELPER_LIB_ROOT/common/xxhash.c $HELPER_LIB_ROOT/libzstd.a"
make -C "$ORIGINAL_ROOT/contrib/seekable_format/tests" \
    test \
    ZSTDLIB_PATH="$HELPER_LIB_ROOT" \
    "SEEKABLE_OBJS=$SEEKABLE_OBJS"
phase6_assert_uses_safe_lib "$ORIGINAL_ROOT/contrib/seekable_format/tests/seekable_tests"
