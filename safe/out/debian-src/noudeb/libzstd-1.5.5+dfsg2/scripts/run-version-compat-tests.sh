#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env
phase6_assert_uses_safe_lib "$BINDIR/zstd"

WORK_DIR="$PHASE6_OUT/version-compat"
install -d "$WORK_DIR"
STAMP_FILE="$WORK_DIR/.stamp"

version_compat_is_fresh() {
    [[ -f $STAMP_FILE ]] || return 1
    local dep
    for dep in \
        "$SCRIPT_DIR/run-version-compat-tests.sh" \
        "$ORIGINAL_ROOT/tests/test-zstd-versions.py" \
        "$BINDIR/zstd" \
        "$VERSIONS_FIXTURE_ROOT" \
        "$ORIGINAL_ROOT/tests/golden-compression" \
        "$ORIGINAL_ROOT/tests/golden-decompression" \
        "$ORIGINAL_ROOT/tests/golden-dictionaries"
    do
        if [[ -d $dep ]]; then
            if find "$dep" -type f -newer "$STAMP_FILE" -print -quit | grep -q .; then
                return 1
            fi
        elif [[ $dep -nt $STAMP_FILE ]]; then
            return 1
        fi
    done
    return 0
}

phase6_log "running offline version-compatibility fixtures"

if version_compat_is_fresh; then
    phase6_log "version-compatibility fixtures already fresh; skipping rerun"
    exit 0
fi

PHASE6_VERSION_FIXTURE_ROOT="$VERSIONS_FIXTURE_ROOT" \
PHASE6_VERSION_WORK_DIR="$WORK_DIR" \
ZSTD_VERSION_BIN="$BINDIR/zstd" \
python3 "$ORIGINAL_ROOT/tests/test-zstd-versions.py"

touch "$STAMP_FILE"
