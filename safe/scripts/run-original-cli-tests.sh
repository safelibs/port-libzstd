#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_ensure_datagen
phase6_export_safe_env

phase6_log "running original cli-tests against the packaged safe CLI"
python3 "$TESTS_ROOT/cli-tests/run.py" \
    --zstd "$BINDIR/zstd" \
    --zstdgrep "$BINDIR/zstdgrep" \
    --zstdless "$BINDIR/zstdless" \
    --datagen "$TESTS_ROOT/datagen" \
    --test-dir "$TESTS_ROOT/cli-tests"
