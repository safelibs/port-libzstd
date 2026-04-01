#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

OFFLINE_ONLY=0
if [[ ${1:-} == --offline-only ]]; then
    OFFLINE_ONLY=1
    shift
fi
if [[ $# -ne 0 ]]; then
    printf 'usage: run-upstream-tests.sh [--offline-only]\n' >&2
    exit 2
fi

phase6_require_phase4_inputs "$0"
phase6_export_safe_env
phase6_assert_uses_safe_lib "$BINDIR/zstd" "$BINDIR/pzstd"

stamp_name=run-upstream-tests
if [[ $OFFLINE_ONLY -eq 1 ]]; then
    stamp_name+=-offline-only
fi
STAMP_FILE=$(phase6_stamp_path "$stamp_name")
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$BINDIR/zstd" \
    "$BINDIR/pzstd" \
    "$HELPER_LIB_ROOT" \
    "$LIBDIR/libzstd.so.1.5.5" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$TESTS_ROOT" \
        "$ORIGINAL_ROOT/programs"
then
    phase6_log "upstream release-gate coverage already fresh; skipping rerun"
    exit 0
fi

UPSTREAM_TESTS_LIB_ROOT=$(phase6_prepare_upstream_tests_helper_root "$PHASE6_OUT/upstream-tests/lib")
UPSTREAM_TESTS_BUILD_ROOT="$PHASE6_OUT/upstream-tests/obj"
install -d "$UPSTREAM_TESTS_BUILD_ROOT"

phase6_make_upstream_test_targets() {
    make -C "$TESTS_ROOT" \
        BUILD_DIR="$UPSTREAM_TESTS_BUILD_ROOT" \
        LIBZSTD="$UPSTREAM_TESTS_LIB_ROOT" \
        PRGDIR="$ORIGINAL_ROOT/programs" \
        "$@"
}

phase6_try_optional_make_targets() {
    local label=${1:?missing label}
    local reason=${2:?missing skip reason}
    local pattern=${3:?missing skip pattern}
    shift 3

    local log
    log=$(mktemp)

    set +e
    phase6_make_upstream_test_targets "$@" >"$log" 2>&1
    local status=$?
    set -e

    if [[ $status -eq 0 ]]; then
        rm -f "$log"
        return 0
    fi

    if grep -Eq "$pattern" "$log"; then
        phase6_log "skipping $label ($reason)"
        rm -f "$log"
        return 1
    fi

    cat "$log" >&2
    rm -f "$log"
    exit "$status"
}

phase6_have_32bit_toolchain() {
    local source="$PHASE6_OUT/tests-32bit-check.c"
    local binary="$PHASE6_OUT/tests-32bit-check"

    printf 'int main(void) { return 0; }\n' >"$source"
    cc -m32 "$source" -o "$binary" >/dev/null 2>&1 || return 1
    "$binary" >/dev/null 2>&1
}

phase6_have_sanitizer_toolchain() {
    local kind=${1:?missing sanitizer kind}
    local source="$PHASE6_OUT/tests-sanitizer-check.c"
    local binary="$PHASE6_OUT/tests-sanitizer-check"
    local -a flags=()

    case "$kind" in
        address)
            flags=(-fsanitize=address)
            ;;
        thread)
            flags=(-fsanitize=thread)
            ;;
        undefined)
            flags=(-fsanitize=undefined)
            ;;
        *)
            printf 'unsupported sanitizer probe: %s\n' "$kind" >&2
            exit 2
            ;;
    esac

    printf 'int main(void) { return 0; }\n' >"$source"
    cc "${flags[@]}" "$source" -o "$binary" >/dev/null 2>&1 || return 1
    "$binary" >/dev/null 2>&1
}

phase6_run_fullbench_smoke() {
    local binary=${1:?missing fullbench binary}
    "$TESTS_ROOT/$binary" -i1 >/dev/null
    "$TESTS_ROOT/$binary" -i1 -P0 >/dev/null
}

phase6_run_fuzzer_smoke() {
    local binary=${1:?missing fuzzer binary}
    "$TESTS_ROOT/$binary" -T1s -t1 >/dev/null
}

phase6_run_zstreamtest_smoke() {
    local binary=${1:?missing zstreamtest binary}
    "$TESTS_ROOT/$binary" -v -T1s >/dev/null
    "$TESTS_ROOT/$binary" --newapi -t1 -T1s >/dev/null
}

phase6_try_zstreamtest_smoke() {
    local label=${1:?missing label}
    local binary=${2:?missing zstreamtest binary}
    local reason=${3:?missing skip reason}
    local pattern=${4:?missing skip pattern}
    local log
    log=$(mktemp)

    set +e
    phase6_run_zstreamtest_smoke "$binary" >"$log" 2>&1
    local status=$?
    set -e

    if [[ $status -eq 0 ]]; then
        rm -f "$log"
        return 0
    fi
    if grep -Eq "$pattern" "$log"; then
        phase6_log "skipping $label ($reason)"
        rm -f "$log"
        return 1
    fi

    cat "$log" >&2
    rm -f "$log"
    exit "$status"
}

phase6_run_optional_tests_makefile_coverage() {
    phase6_log "dispatching upstream tests/Makefile release-gate coverage through the Phase 4 helper-root overlay"

    phase6_log "running tests:allnothread aggregate build"
    phase6_make_upstream_test_targets allnothread

    if phase6_have_32bit_toolchain; then
        if phase6_try_optional_make_targets \
            "tests:all32/tests:fullbench32/tests:fuzzer32/tests:zstreamtest32" \
            "32-bit toolchain unavailable or unsupported on this host" \
            'missing IBT and SHSTK properties|cannot find .*(crt|libgcc|libstdc\+\+)|skipping incompatible|file in wrong format' \
            all32
        then
            phase6_log "running tests:fullbench32 smoke"
            phase6_run_fullbench_smoke fullbench32

            phase6_log "running tests:fuzzer32 smoke"
            phase6_run_fuzzer_smoke fuzzer32

            phase6_log "running tests:zstreamtest32 smoke"
            phase6_run_zstreamtest_smoke zstreamtest32
        fi
    else
        phase6_log "skipping tests:all32/tests:fullbench32/tests:fuzzer32/tests:zstreamtest32 (32-bit toolchain unavailable)"
    fi

    if phase6_have_sanitizer_toolchain address; then
        phase6_log "running tests:zstreamtest_asan smoke"
        phase6_make_upstream_test_targets zstreamtest_asan
        phase6_run_zstreamtest_smoke zstreamtest_asan
    else
        phase6_log "skipping tests:zstreamtest_asan (address sanitizer toolchain unavailable)"
    fi

    if phase6_have_sanitizer_toolchain thread; then
        phase6_log "running tests:zstreamtest_tsan smoke"
        phase6_make_upstream_test_targets zstreamtest_tsan
        phase6_try_zstreamtest_smoke \
            "tests:zstreamtest_tsan" \
            zstreamtest_tsan \
            "thread sanitizer runtime unsupported on this host" \
            'FATAL: ThreadSanitizer: unexpected memory mapping' || :
    else
        phase6_log "skipping tests:zstreamtest_tsan (thread sanitizer toolchain unavailable)"
    fi

    if phase6_have_sanitizer_toolchain undefined; then
        phase6_log "running tests:zstreamtest_ubsan smoke"
        phase6_make_upstream_test_targets zstreamtest_ubsan
        phase6_run_zstreamtest_smoke zstreamtest_ubsan
    else
        phase6_log "skipping tests:zstreamtest_ubsan (undefined-behavior sanitizer toolchain unavailable)"
    fi

    if phase6_have_command valgrind; then
        phase6_log "running tests:test-valgrind reduced smoke"
        phase6_make_upstream_test_targets datagen fullbench fuzzer
        valgrind --leak-check=full --show-leak-kinds=all --error-exitcode=1 \
            "$TESTS_ROOT/datagen" -g1M >/dev/null
        "$TESTS_ROOT/datagen" -g1M | \
            valgrind --leak-check=full --show-leak-kinds=all --error-exitcode=1 \
                "$BINDIR/zstd" -q -f - -c >/dev/null
        valgrind --leak-check=full --show-leak-kinds=all --error-exitcode=1 \
            "$TESTS_ROOT/fuzzer" -T1s -t1 >/dev/null
        valgrind --leak-check=full --show-leak-kinds=all --error-exitcode=1 \
            "$TESTS_ROOT/fullbench" -i1 >/dev/null
    else
        phase6_log "skipping tests:test-valgrind (valgrind unavailable)"
    fi
}

bash "$SAFE_ROOT/scripts/run-version-compat-tests.sh"
bash "$SAFE_ROOT/scripts/run-upstream-regression.sh"
bash "$SAFE_ROOT/scripts/run-upstream-fuzz-tests.sh"
phase6_run_optional_tests_makefile_coverage
bash "$SAFE_ROOT/scripts/run-original-cli-tests.sh"
bash "$SAFE_ROOT/scripts/check-cli-permissions.sh"
bash "$SAFE_ROOT/scripts/run-performance-smoke.sh"

phase6_log "running upstream license audit"
python3 "$ORIGINAL_ROOT/tests/test-license.py"

phase6_log "running upstream size check on a stripped safe shared object copy"
SIZE_WORK_DIR="$PHASE6_OUT/check-size"
SAFE_SIZE_LIMIT=1350000
rm -rf "$SIZE_WORK_DIR"
install -d "$SIZE_WORK_DIR"
strip -o "$SIZE_WORK_DIR/libzstd.so" "$LIBDIR/libzstd.so.1.5.5"
python3 "$ORIGINAL_ROOT/tests/check_size.py" "$SIZE_WORK_DIR/libzstd.so" "$SAFE_SIZE_LIMIT"

if [[ $OFFLINE_ONLY -eq 0 ]]; then
    :
else
    phase6_log "upstream release gate already runs entirely offline"
fi

phase6_touch_stamp "$STAMP_FILE"
