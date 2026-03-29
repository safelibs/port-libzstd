#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)

resolve_upstream_lib() {
    if [[ -n ${SAFE_UPSTREAM_LIB:-} ]]; then
        printf '%s\n' "$SAFE_UPSTREAM_LIB"
        return 0
    fi

    local candidate
    candidate=$(ldconfig -p 2>/dev/null | awk '/libzstd\.so\.1 / {print $NF; exit}')
    if [[ -z $candidate || ! -e $candidate ]]; then
        echo "unable to resolve upstream libzstd.so.1" >&2
        exit 1
    fi

    printf '%s\n' "$candidate"
}

make -C "$SAFE_ROOT/tests/link-compat" clean >/dev/null
SAFE_UPSTREAM_LIB="$(resolve_upstream_lib)" make -C "$SAFE_ROOT/tests/link-compat" run \
    SAFE_ROOT="$SAFE_ROOT" \
    REPO_ROOT="$REPO_ROOT"
