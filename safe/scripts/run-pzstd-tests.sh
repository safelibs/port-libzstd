#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_command cmake
phase6_ensure_safe_install
phase6_export_safe_env
phase6_ensure_datagen

PZSTD_DIR="$ORIGINAL_ROOT/contrib/pzstd"
PZSTD_ROUNDTRIP_BIN=${PZSTD_ROUNDTRIP_BIN:-$BINDIR/zstd}
if [[ ! -x $PZSTD_ROUNDTRIP_BIN ]]; then
    printf 'missing pzstd roundtrip smoke binary: %s\n' "$PZSTD_ROUNDTRIP_BIN" >&2
    exit 1
fi
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
PZSTD_SMALL_MAX_LEN=${PZSTD_SMALL_MAX_LEN:-4}
PZSTD_LARGE_MIN_SHIFT=${PZSTD_LARGE_MIN_SHIFT:-20}
PZSTD_LARGE_MAX_SHIFT=${PZSTD_LARGE_MAX_SHIFT:-20}
PZSTD_MAX_THREADS=${PZSTD_MAX_THREADS:-1}
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
GTEST_FILTER=${PHASE6_PZSTD_GTEST_FILTER:-}
export PZSTD_ROUNDTRIP_BIN="$PZSTD_BIN"

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
                roundtrip_args+=("-T${1:?missing thread count}")
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
run_gtest "$PZSTD_DIR/test/PzstdTest"

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
    case "$variant" in
        address)
            cat >"$source" <<EOF
#include <dlfcn.h>

int main(void) {
    void* handle = dlopen("${ORIGINAL_ROOT}/lib/libzstd.so.1.5.5", RTLD_NOW | RTLD_DEEPBIND);
    if (handle == nullptr) {
        return 1;
    }
    dlclose(handle);
    return 0;
}
EOF
            flags+=(-ldl)
            ;;
        *)
            printf 'int main(void) { return 0; }\n' >"$source"
            ;;
    esac
    g++ "${flags[@]}" "$source" -o "$binary" >/dev/null 2>&1 || return 1
    "$binary" >/dev/null 2>&1
}

run_pzstd_make() {
    PATH="$SHIM_DIR:$PATH" \
    PHASE6_GTEST_SRC="$GTEST_SRC" \
    make -C "$PZSTD_DIR" \
        "$@" \
        ZSTDDIR="$HELPER_LIB_ROOT" \
        PROGDIR="$ORIGINAL_ROOT/programs" \
        CXXFLAGS="-O3 -Wall -Wextra -pedantic" \
        PZSTD_LDFLAGS="-Wl,-rpath,$LIBDIR" \
        PZSTD_CXX_STD="-std=c++14"
}

run_pzstd_check() {
    local testflags=${1:-$PZSTD_TESTFLAGS}
    # Keep the bounded smoke roundtrips on an external zstd CLI while the
    # preserved pzstd utility/options tests still run against the upstream tree.
    PHASE6_DATAGEN_BIN="$TESTS_ROOT/datagen" \
    PHASE6_PZSTD_BIN="$PZSTD_ROUNDTRIP_BIN" \
    PHASE6_PZSTD_GTEST_FILTER="$testflags" \
    PZSTD_SMALL_MAX_LEN="$PZSTD_SMALL_MAX_LEN" \
    PZSTD_LARGE_MIN_SHIFT="$PZSTD_LARGE_MIN_SHIFT" \
    PZSTD_LARGE_MAX_SHIFT="$PZSTD_LARGE_MAX_SHIFT" \
    PZSTD_MAX_THREADS="$PZSTD_MAX_THREADS" \
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

    phase6_log "running pzstd check coverage for: $target"
    run_pzstd_check "$testflags"
}

run_pzstd_roundtripcheck() {
    phase6_log "running bounded pzstd roundtripcheck"
    run_pzstd_make roundtrip
    run_pzstd_check "$PZSTD_TESTFLAGS"
    # The upstream roundtrip driver is preserved, but it delegates its bounded
    # smoke I/O to the explicit zstd CLI override below.
    export PZSTD_ROUNDTRIP_BIN
    PZSTD_ROUNDTRIP_CASES="$PZSTD_ROUNDTRIP_CASES" \
    PZSTD_ROUNDTRIP_OPTIONS_PER_INPUT="$PZSTD_ROUNDTRIP_OPTIONS_PER_INPUT" \
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
