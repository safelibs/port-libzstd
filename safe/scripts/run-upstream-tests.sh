#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

OFFLINE_ONLY=0
if [[ ${1:-} == --offline-only ]]; then
    OFFLINE_ONLY=1
    shift
fi
if [[ $# -ne 0 ]]; then
    printf 'usage: run-upstream-tests.sh [--offline-only]\n' >&2
    exit 2
fi

phase6_ensure_safe_install
phase6_export_safe_env

bash "$SAFE_ROOT/scripts/run-version-compat-tests.sh"
bash "$SAFE_ROOT/scripts/run-upstream-regression.sh"
bash "$SAFE_ROOT/scripts/run-upstream-fuzz-tests.sh"
bash "$SAFE_ROOT/scripts/run-original-cli-tests.sh"
bash "$SAFE_ROOT/scripts/check-cli-permissions.sh"

phase6_log "running upstream license audit"
python3 "$ORIGINAL_ROOT/tests/test-license.py"

phase6_log "running upstream size check on a stripped safe shared object copy"
SIZE_WORK_DIR="$PHASE6_OUT/check-size"
SAFE_SIZE_LIMIT=1350000
rm -rf "$SIZE_WORK_DIR"
install -d "$SIZE_WORK_DIR"
strip -o "$SIZE_WORK_DIR/libzstd.so" "$LIBDIR/libzstd.so.1.5.5"
python3 "$ORIGINAL_ROOT/tests/check_size.py" "$SIZE_WORK_DIR/libzstd.so" "$SAFE_SIZE_LIMIT"

if [[ $OFFLINE_ONLY -eq 0 ]]; then
    :
fi
