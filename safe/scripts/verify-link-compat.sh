#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
source "$SAFE_ROOT/scripts/phase6-common.sh"

STAMP_FILE=$(phase6_stamp_path verify-link-compat)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$SAFE_ROOT/Cargo.toml" \
        "$SAFE_ROOT/src" \
        "$SAFE_ROOT/tests/link-compat" \
        "$SAFE_ROOT/tests/capi" \
        "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/tests" \
        "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/examples"
then
    phase6_log "link compatibility coverage already fresh; skipping rerun"
    exit 0
fi

make -C "$SAFE_ROOT/tests/link-compat" clean >/dev/null
make -C "$SAFE_ROOT/tests/link-compat" run \
    SAFE_ROOT="$SAFE_ROOT" \
    REPO_ROOT="$REPO_ROOT"

phase6_touch_stamp "$STAMP_FILE"
