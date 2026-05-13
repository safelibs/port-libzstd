#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_command cmake
phase6_require_phase4_inputs "$0"
phase6_export_safe_env
phase6_ensure_datagen

PZSTD_DIR="$ORIGINAL_ROOT/contrib/pzstd"
STAMP_FILE=$(phase6_stamp_path run-pzstd-tests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$BINDIR/zstd" \
    "$HELPER_LIB_ROOT/libzstd.a" \
    "$HELPER_LIB_ROOT/libzstd.so.1.5.5" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$PZSTD_DIR" \
        "$ORIGINAL_ROOT/programs"
then
    phase6_log "pzstd coverage already fresh; skipping rerun"
    exit 0
fi

PZSTD_ROUNDTRIP_BIN=${PZSTD_ROUNDTRIP_BIN:-}
GTEST_SRC=/usr/src/googletest
if [[ ! -d $GTEST_SRC ]]; then
    printf 'missing system googletest source tree: %s\n' "$GTEST_SRC" >&2
    exit 1
fi

SHIM_DIR="$PHASE6_OUT/pzstd-shim"
PZSTD_TESTFLAGS=${PZSTD_TESTFLAGS:---gtest_filter=-*ExtremelyLarge*}
PZSTD_OPTIONAL_TESTFLAGS=${PZSTD_OPTIONAL_TESTFLAGS:---gtest_filter=-*ExtremelyLarge*}
PZSTD_ROUNDTRIP_CASES=${PZSTD_ROUNDTRIP_CASES:-8}
PZSTD_ROUNDTRIP_OPTIONS_PER_INPUT=${PZSTD_ROUNDTRIP_OPTIONS_PER_INPUT:-1}
PZSTD_SMALL_MAX_LEN=${PZSTD_SMALL_MAX_LEN:-1}
PZSTD_LARGE_MIN_SHIFT=${PZSTD_LARGE_MIN_SHIFT:-20}
PZSTD_LARGE_MAX_SHIFT=${PZSTD_LARGE_MAX_SHIFT:-19}
PZSTD_MAX_THREADS=${PZSTD_MAX_THREADS:-1}
PZSTD_MAX_LEVEL=${PZSTD_MAX_LEVEL:-1}
PZSTD_CHECK_SCRIPT="$PHASE6_OUT/pzstd-check.sh"
install -d "$SHIM_DIR"
cat >"$SHIM_DIR/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ ${1:-} == clone && ${2:-} == https://github.com/google/googletest ]]; then
    dest=${3:-googletest}
    src=${PHASE6_GTEST_SRC:?}
    rm -rf "$dest"
    cp -a "$src" "$dest"
    exit 0
fi

exec /usr/bin/git "$@"
EOF
chmod +x "$SHIM_DIR/git"

cat >"$PZSTD_CHECK_SCRIPT" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

PZSTD_DIR=${1:?missing pzstd dir}
DATAGEN_BIN=${PHASE6_DATAGEN_BIN:?missing datagen binary}
PZSTD_BIN=${PHASE6_PZSTD_BIN:?missing pzstd binary}
PZSTD_BIN_STYLE=${PHASE6_PZSTD_BIN_STYLE:-pzstd}
PZSTD_GTEST_ROUNDTRIP_BIN=${PHASE6_PZSTD_GTEST_ROUNDTRIP_BIN:-}
PZSTD_GTEST_ROUNDTRIP_STYLE=${PHASE6_PZSTD_GTEST_ROUNDTRIP_STYLE:-}
GTEST_FILTER=${PHASE6_PZSTD_GTEST_FILTER:-}

run_gtest() {
    local binary=$1
    if [[ -n $GTEST_FILTER ]]; then
        "$binary" "$GTEST_FILTER"
    else
        "$binary"
    fi
}

run_pzstd_roundtrip() {
    local input=$1
    shift
    local output="$input.out"
    local -a roundtrip_args=()

    while [[ $# -gt 0 ]]; do
        case $1 in
            -p)
                shift
                if [[ $PZSTD_BIN_STYLE == zstd ]]; then
                    roundtrip_args+=("-T${1:?missing thread count}")
                else
                    roundtrip_args+=("-p" "${1:?missing thread count}")
                fi
                ;;
            *)
                roundtrip_args+=("$1")
                ;;
        esac
        shift
    done

    "$PZSTD_BIN" -q -f "${roundtrip_args[@]}" -o "$input.zst" "$input"
    "$PZSTD_BIN" -q -d -f -o "$output" "$input.zst"
    cmp "$input" "$output"
}

run_gtest "$PZSTD_DIR/utils/test/BufferTest"
run_gtest "$PZSTD_DIR/utils/test/RangeTest"
run_gtest "$PZSTD_DIR/utils/test/ResourcePoolTest"
run_gtest "$PZSTD_DIR/utils/test/ScopeGuardTest"
run_gtest "$PZSTD_DIR/utils/test/ThreadPoolTest"
run_gtest "$PZSTD_DIR/utils/test/WorkQueueTest"
run_gtest "$PZSTD_DIR/test/OptionsTest"
if [[ -n $PZSTD_GTEST_ROUNDTRIP_BIN ]]; then
    PZSTD_ROUNDTRIP_BIN="$PZSTD_GTEST_ROUNDTRIP_BIN" \
    PZSTD_ROUNDTRIP_STYLE="$PZSTD_GTEST_ROUNDTRIP_STYLE" \
        run_gtest "$PZSTD_DIR/test/PzstdTest"
else
    unset PZSTD_ROUNDTRIP_BIN
    unset PZSTD_ROUNDTRIP_STYLE
    run_gtest "$PZSTD_DIR/test/PzstdTest"
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

small_input="$tmpdir/small.txt"
medium_input="$tmpdir/medium.bin"
large_input="$tmpdir/large.bin"

"$DATAGEN_BIN" -g65536 >"$small_input"
"$DATAGEN_BIN" -g262144 >"$medium_input"
"$DATAGEN_BIN" -g1048576 >"$large_input"

run_pzstd_roundtrip "$small_input" -p 1 -1
run_pzstd_roundtrip "$medium_input" -p 2 -4
run_pzstd_roundtrip "$large_input" -p 1 -3
EOF
chmod +x "$PZSTD_CHECK_SCRIPT"

phase6_have_pzstd_toolchain() {
    local variant=${1:-}
    local source="$PHASE6_OUT/pzstd-toolchain-check.cpp"
    local binary="$PHASE6_OUT/pzstd-toolchain-check"
    local -a flags=(-std=c++14)

    case "$variant" in
        thread)
            flags+=(-fsanitize=thread -fuse-ld=gold)
            ;;
        address)
            flags+=(-fsanitize=address -fuse-ld=gold)
            ;;
        '')
            ;;
        *)
            printf 'unsupported pzstd toolchain probe: %s\n' "$variant" >&2
            exit 2
            ;;
    esac

    install -d "$PHASE6_OUT"
    printf 'int main(void) { return 0; }\n' >"$source"
    g++ "${flags[@]}" "$source" -o "$binary" >/dev/null 2>&1 || return 1
    "$binary" >/dev/null 2>&1
}

phase6_have_pzstd_sanitizer_runtime() {
    local target=${1:?missing pzstd sanitizer target}
    local binary="$PZSTD_DIR/utils/test/BufferTest"
    local log="$PHASE6_OUT/${target}.runtime.log"
    local status

    [[ -x $binary ]] || {
        printf 'missing pzstd runtime probe binary: %s\n' "$binary" >&2
        return 2
    }

    if "$binary" --gtest_filter=NoSuchTest >/dev/null 2>"$log"; then
        return 0
    fi
    status=$?

    if grep -Eq '(Thread|Address)Sanitizer:' "$log"; then
        phase6_log "skipping $target: sanitizer runtime is unsupported on this host"
        return 3
    fi
    if [[ $status -ge 128 && ! -s $log ]]; then
        phase6_log "skipping $target: sanitizer runtime probe crashed during startup"
        return 3
    fi

    cat "$log" >&2
    return 2
}

run_pzstd_make() {
    PATH="$SHIM_DIR:$PATH" \
    PHASE6_GTEST_SRC="$GTEST_SRC" \
    make -C "$PZSTD_DIR" \
        "$@" \
        ZSTDDIR="$HELPER_LIB_ROOT" \
        PROGDIR="$ORIGINAL_ROOT/programs" \
        CXXFLAGS="-O3 -Wall -Wextra -pedantic" \
        PZSTD_CXX_STD="-std=c++14"
}

run_pzstd_check() {
    local testflags=${1:-$PZSTD_TESTFLAGS}
    local roundtrip_bin=${PZSTD_CHECK_BIN:-$BINDIR/zstd}
    local roundtrip_style=${PZSTD_CHECK_BIN_STYLE:-}
    [[ -x $roundtrip_bin ]] || {
        printf 'missing pzstd check smoke binary: %s\n' "$roundtrip_bin" >&2
        exit 1
    }
    if [[ -z $roundtrip_style ]]; then
        case $(basename "$roundtrip_bin") in
            zstd|zstd-*)
                roundtrip_style=zstd
                ;;
            *)
                roundtrip_style=pzstd
                ;;
        esac
    fi

    PHASE6_DATAGEN_BIN="$TESTS_ROOT/datagen" \
    PHASE6_PZSTD_BIN="$roundtrip_bin" \
    PHASE6_PZSTD_BIN_STYLE="$roundtrip_style" \
    PHASE6_PZSTD_GTEST_ROUNDTRIP_BIN="$roundtrip_bin" \
    PHASE6_PZSTD_GTEST_ROUNDTRIP_STYLE="$roundtrip_style" \
    PHASE6_PZSTD_GTEST_FILTER="$testflags" \
    PZSTD_SMALL_MAX_LEN="$PZSTD_SMALL_MAX_LEN" \
    PZSTD_LARGE_MIN_SHIFT="$PZSTD_LARGE_MIN_SHIFT" \
    PZSTD_LARGE_MAX_SHIFT="$PZSTD_LARGE_MAX_SHIFT" \
    PZSTD_MAX_THREADS="$PZSTD_MAX_THREADS" \
    PZSTD_MAX_LEVEL="$PZSTD_MAX_LEVEL" \
    bash "$PZSTD_CHECK_SCRIPT" "$PZSTD_DIR"
}

run_pzstd_test_family() {
    local target=$1
    local testflags=${2:-$PZSTD_TESTFLAGS}
    local -a build_args

    case "$target" in
        test-pzstd)
            build_args=(clean googletest pzstd tests)
            ;;
        test-pzstd32)
            build_args=(clean googletest32 all32)
            ;;
        test-pzstd-tsan)
            build_args=(clean googletest tsan)
            ;;
        test-pzstd-asan)
            build_args=(clean asan)
            ;;
        *)
            printf 'unsupported pzstd target family: %s\n' "$target" >&2
            exit 2
            ;;
    esac

    phase6_log "building pzstd target family: $target"
    run_pzstd_make "${build_args[@]}"

    case "$target" in
        test-pzstd-tsan|test-pzstd-asan)
            local runtime_status=0
            phase6_have_pzstd_sanitizer_runtime "$target" || runtime_status=$?
            case $runtime_status in
                0)
                    ;;
                3)
                    return 0
                    ;;
                *)
                    return 1
                    ;;
            esac
            ;;
    esac

    phase6_log "running pzstd check coverage for: $target"
    run_pzstd_check "$testflags"
}

run_pzstd_roundtripcheck() {
    local roundtrip_bin=${PZSTD_ROUNDTRIP_BIN:-$PZSTD_DIR/pzstd}
    local roundtrip_style=${PZSTD_ROUNDTRIP_STYLE:-}
    [[ -x $roundtrip_bin ]] || {
        printf 'missing pzstd roundtrip binary: %s\n' "$roundtrip_bin" >&2
        exit 1
    }
    if [[ -z $roundtrip_style ]]; then
        case $(basename "$roundtrip_bin") in
            zstd|zstd-*)
                roundtrip_style=zstd
                ;;
            *)
                roundtrip_style=pzstd
                ;;
        esac
    fi

    phase6_log "running bounded pzstd roundtripcheck"
    run_pzstd_make tests roundtrip
    export PZSTD_ROUNDTRIP_BIN="$roundtrip_bin"
    export PZSTD_ROUNDTRIP_STYLE="$roundtrip_style"
    PZSTD_ROUNDTRIP_CASES="$PZSTD_ROUNDTRIP_CASES" \
    PZSTD_ROUNDTRIP_OPTIONS_PER_INPUT="$PZSTD_ROUNDTRIP_OPTIONS_PER_INPUT" \
    PZSTD_MAX_THREADS="$PZSTD_MAX_THREADS" \
    PZSTD_MAX_LEVEL="$PZSTD_MAX_LEVEL" \
    "$PZSTD_DIR/test/RoundTripTest"
}

run_pzstd_test_family test-pzstd "$PZSTD_TESTFLAGS"
phase6_export_safe_env
phase6_assert_uses_safe_lib "$PZSTD_DIR/pzstd"
if [[ -x $PZSTD_DIR/test/PzstdTest ]]; then
    phase6_assert_uses_safe_lib "$PZSTD_DIR/test/PzstdTest"
fi
run_pzstd_roundtripcheck
phase6_export_safe_env
phase6_assert_uses_safe_lib \
    "$PZSTD_DIR/pzstd"

if phase6_have_pkg gcc-multilib && phase6_have_pkg g++-multilib; then
    run_pzstd_test_family test-pzstd32 "$PZSTD_OPTIONAL_TESTFLAGS"
fi

if phase6_have_pzstd_toolchain thread; then
    run_pzstd_test_family test-pzstd-tsan "$PZSTD_OPTIONAL_TESTFLAGS"
fi

if phase6_have_pzstd_toolchain address; then
    run_pzstd_test_family test-pzstd-asan "$PZSTD_OPTIONAL_TESTFLAGS"
fi

phase6_touch_stamp "$STAMP_FILE"
