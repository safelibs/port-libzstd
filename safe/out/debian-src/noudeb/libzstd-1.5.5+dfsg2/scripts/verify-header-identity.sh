#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
UPSTREAM_INCLUDE="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/lib"

for header in zstd.h zdict.h zstd_errors.h; do
  cmp -s "$SAFE_ROOT/include/$header" "$UPSTREAM_INCLUDE/$header" || {
    echo "header mismatch: $header" >&2
    exit 1
  }
done
