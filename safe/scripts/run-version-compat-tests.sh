#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_export_safe_env

WORK_DIR="$PHASE6_OUT/version-compat"
rm -rf "$WORK_DIR"
install -d "$WORK_DIR"

phase6_log "running offline version-compatibility fixtures"

"$BINDIR/zstd" -q -t "$VERSIONS_FIXTURE_ROOT/empty-block.zst"
"$BINDIR/zstd" -q -t "$VERSIONS_FIXTURE_ROOT/rle-first-block.zst"
"$BINDIR/zstd" -q -t "$VERSIONS_FIXTURE_ROOT/huffman-compressed-larger"

"$BINDIR/zstd" -q -dc "$VERSIONS_FIXTURE_ROOT/hello.zst" >"$WORK_DIR/hello.out"
cmp -s "$VERSIONS_FIXTURE_ROOT/hello" "$WORK_DIR/hello.out"

"$BINDIR/zstd" -q -dc "$VERSIONS_FIXTURE_ROOT/helloworld.zst" >"$WORK_DIR/helloworld.out"
cmp -s "$VERSIONS_FIXTURE_ROOT/helloworld" "$WORK_DIR/helloworld.out"

"$BINDIR/zstd" -q -f \
    -D "$VERSIONS_FIXTURE_ROOT/http-dict-missing-symbols" \
    "$VERSIONS_FIXTURE_ROOT/http" \
    -o "$WORK_DIR/http.zst"
"$BINDIR/zstd" -q -dc \
    -D "$VERSIONS_FIXTURE_ROOT/http-dict-missing-symbols" \
    "$WORK_DIR/http.zst" >"$WORK_DIR/http.out"
cmp -s "$VERSIONS_FIXTURE_ROOT/http" "$WORK_DIR/http.out"
