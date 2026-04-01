#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)

make -C "$SAFE_ROOT/tests/link-compat" clean >/dev/null
make -C "$SAFE_ROOT/tests/link-compat" run \
    SAFE_ROOT="$SAFE_ROOT" \
    REPO_ROOT="$REPO_ROOT"
