#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_ensure_datagen
phase6_export_safe_env

phase6_log "running playTests.sh against the safe install tree"
(
    cd "$TESTS_ROOT"
    ZSTD_BIN="$BINDIR/zstd" \
    DATAGEN_BIN="$TESTS_ROOT/datagen" \
    EXEC_PREFIX="${EXEC_PREFIX:-}" \
    bash ./playTests.sh
)

phase6_log "building original CLI variants against the safe library"
phase6_build_original_cli_variants

phase6_log "running test-variants.sh against the safe library variants"
(
    cd "$TESTS_ROOT"
    sh ./test-variants.sh
)
