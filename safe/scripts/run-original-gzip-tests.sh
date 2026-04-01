#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env
phase6_assert_uses_safe_lib "$BINDIR/zstd"

GZIP_DIR="$TESTS_ROOT/gzip"
STAMP_FILE=$(phase6_stamp_path run-original-gzip-tests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$BINDIR/zstd" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$GZIP_DIR"
then
    phase6_log "original gzip compatibility suite already fresh; skipping rerun"
    exit 0
fi

phase6_log "running original gzip compatibility shell suite against the safe CLI"

cleanup() {
    rm -f \
        "$GZIP_DIR/gzip" \
        "$GZIP_DIR/gunzip" \
        "$GZIP_DIR/zcat" \
        "$GZIP_DIR/gzcat"
}
trap cleanup EXIT

ln -sfn "$BINDIR/zstd" "$GZIP_DIR/gzip"
ln -sfn "$BINDIR/zstd" "$GZIP_DIR/gunzip"
ln -sfn "$BINDIR/zstd" "$GZIP_DIR/zcat"
ln -sfn "$BINDIR/zstd" "$GZIP_DIR/gzcat"

tests=(
    helin-segv
    hufts
    keep
    list
    memcpy-abuse
    mixed
    null-suffix-clobber
    stdin
    trailing-nul
    unpack-invalid
    zdiff
    zgrep-context
    zgrep-f
    zgrep-signal
    znew-k
    z-suffix
)

(
    cd "$GZIP_DIR"
    export srcdir="$GZIP_DIR"
    export abs_srcdir="$GZIP_DIR"
    local_test=
    expect_failure=no
    status=0
    result=
    for local_test in "${tests[@]}"; do
        expect_failure=no
        case "$local_test" in
            helin-segv|hufts|keep|list|null-suffix-clobber|trailing-nul|zgrep-context|zgrep-f|znew-k|z-suffix)
                # The preserved GNU gzip suite includes legacy .Z handling and
                # gzip-specific CLI semantics that upstream zstd's gzip-compat
                # frontend does not implement. Keep them as tracked expected
                # failures so the wrapper still catches unexpected regressions
                # in the supported subset.
                expect_failure=yes
                ;;
        esac
        set +e
        bash ./test-driver.sh \
            --test-name "$local_test" \
            --log-file "$local_test.log" \
            --trs-file "$local_test.trs" \
            --expect-failure "$expect_failure" \
            --color-tests yes \
            --enable-hard-errors yes \
            -- \
            bash "./$local_test.sh"
        status=$?
        set -e
        result=$(sed -n 's/^:test-result: //p' "$local_test.trs")
        if [[ $result == XPASS ]]; then
            printf 'expected-failure test unexpectedly passed: %s\n' "$local_test" >&2
            exit 1
        fi
        if [[ $status -ne 0 && $result != XFAIL ]]; then
            exit "$status"
        fi
    done
)

phase6_touch_stamp "$STAMP_FILE"
