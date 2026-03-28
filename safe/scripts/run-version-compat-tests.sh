#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_export_safe_env

WORK_DIR="$PHASE6_OUT/version-compat"
install -d "$WORK_DIR"
STAMP_FILE="$WORK_DIR/.stamp"

version_compat_is_fresh() {
    [[ -f $STAMP_FILE ]] || return 1
    local dep
    for dep in \
        "$SCRIPT_DIR/run-version-compat-tests.sh" \
        "$BINDIR/zstd" \
        "$VERSIONS_FIXTURE_ROOT"
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

touch "$STAMP_FILE"
