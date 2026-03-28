#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_export_safe_env

phase6_log "building and running upstream examples against the safe helper lib root"
make -C "$ORIGINAL_ROOT/examples" clean LIBDIR="$HELPER_LIB_ROOT"
make -C "$ORIGINAL_ROOT/examples" test LIBDIR="$HELPER_LIB_ROOT"
