#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_export_safe_env

phase6_log "running educational decoder tests with packaged safe zstd"
make -C "$ORIGINAL_ROOT/doc/educational_decoder" clean
make -C "$ORIGINAL_ROOT/doc/educational_decoder" \
    test \
    ZSTD="$BINDIR/zstd"
