#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_ensure_datagen
phase6_export_safe_env
phase6_require_command rsync

STAMP_FILE=$(phase6_stamp_path run-original-playtests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$BINDIR/zstd" \
    "$HELPER_LIB_ROOT/libzstd.a" \
    "$HELPER_LIB_ROOT/libzstd.so.1.5.5" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$TESTS_ROOT" \
        "$ORIGINAL_ROOT/programs"
then
    phase6_log "playTests.sh and variant coverage already fresh; skipping rerun"
    exit 0
fi

stage_playtests_sandbox() {
    local sandbox_root="$PHASE6_OUT/playtests-sandbox"

    rm -rf "$sandbox_root"
    install -d "$sandbox_root"
    rsync -a "$TESTS_ROOT/" "$sandbox_root/tests/"
    rsync -a "$ORIGINAL_ROOT/programs/" "$sandbox_root/programs/"
    printf '%s\n' "$sandbox_root"
}

playtests_sandbox=$(stage_playtests_sandbox)

phase6_log "running playTests.sh against the safe install tree from a staged sandbox"
(
    cd "$playtests_sandbox/tests"
    ZSTD_BIN="$BINDIR/zstd" \
    DATAGEN_BIN="$playtests_sandbox/tests/datagen" \
    EXEC_PREFIX="${EXEC_PREFIX:-}" \
    bash ./playTests.sh
)

phase6_log "building original CLI variants against the safe library"
phase6_build_original_cli_variants

phase6_log "running test-variants.sh against the safe library variants"
(
    cd "$PHASE6_VARIANTS_TESTS"
    sh ./test-variants.sh
)

phase6_touch_stamp "$STAMP_FILE"
