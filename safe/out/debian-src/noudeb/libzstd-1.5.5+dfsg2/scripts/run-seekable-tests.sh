#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env

STAMP_FILE=$(phase6_stamp_path run-seekable-tests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$HELPER_LIB_ROOT/libzstd.a" \
    "$HELPER_LIB_ROOT/libzstd.so.1.5.5" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$ORIGINAL_ROOT/contrib/seekable_format"
then
    phase6_log "seekable format tests already fresh; skipping rerun"
    exit 0
fi

phase6_log "building and running contrib/seekable_format tests against the safe library"
make -C "$ORIGINAL_ROOT/contrib/seekable_format/tests" clean ZSTDLIB_PATH="$HELPER_LIB_ROOT"
SEEKABLE_OBJS="../zstdseek_compress.c ../zstdseek_decompress.c $HELPER_LIB_ROOT/common/xxhash.c $HELPER_LIB_ROOT/libzstd.a"
make -C "$ORIGINAL_ROOT/contrib/seekable_format/tests" \
    test \
    ZSTDLIB_PATH="$HELPER_LIB_ROOT" \
    "SEEKABLE_OBJS=$SEEKABLE_OBJS"
phase6_assert_uses_safe_lib "$ORIGINAL_ROOT/contrib/seekable_format/tests/seekable_tests"

phase6_touch_stamp "$STAMP_FILE"
