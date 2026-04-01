#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_ensure_datagen
phase6_export_safe_env
phase6_assert_uses_safe_lib "$BINDIR/zstd"

STAMP_FILE=$(phase6_stamp_path run-original-cli-tests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$BINDIR/zstd" \
    "$BINDIR/zstdgrep" \
    "$BINDIR/zstdless" \
    "$TESTS_ROOT/datagen" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$TESTS_ROOT/cli-tests" \
        "$TESTS_ROOT/datagencli.c" \
        "$ORIGINAL_ROOT/programs/datagen.c"
then
    phase6_log "original cli-tests already fresh; skipping rerun"
    exit 0
fi

phase6_log "running original cli-tests against the packaged safe CLI"
python3 "$TESTS_ROOT/cli-tests/run.py" \
    --preserve \
    --zstd "$BINDIR/zstd" \
    --zstdgrep "$BINDIR/zstdgrep" \
    --zstdless "$BINDIR/zstdless" \
    --datagen "$TESTS_ROOT/datagen" \
    --test-dir "$TESTS_ROOT/cli-tests"

phase6_touch_stamp "$STAMP_FILE"
