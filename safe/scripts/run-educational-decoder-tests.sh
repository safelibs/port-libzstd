#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env
phase6_assert_uses_safe_lib "$BINDIR/zstd"

STAMP_FILE=$(phase6_stamp_path run-educational-decoder-tests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$BINDIR/zstd" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$ORIGINAL_ROOT/doc/educational_decoder"
then
    phase6_log "educational decoder tests already fresh; skipping rerun"
    exit 0
fi

phase6_log "running educational decoder tests with packaged safe zstd"
make -C "$ORIGINAL_ROOT/doc/educational_decoder" clean
make -C "$ORIGINAL_ROOT/doc/educational_decoder" \
    test \
    ZSTD="$BINDIR/zstd"

phase6_touch_stamp "$STAMP_FILE"
