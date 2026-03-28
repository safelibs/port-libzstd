#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_command valgrind
phase6_ensure_safe_install
phase6_export_safe_env

EXAMPLE_SHIM="$ORIGINAL_ROOT/zlibWrapper/examples/zlib.h"

cleanup_example_shim() {
    rm -f "$EXAMPLE_SHIM"
}

install_example_shim() {
    cat >"$EXAMPLE_SHIM" <<'EOF'
#include "../zstd_zlibwrapper.h"
EOF
}

run_zlibwrapper_make() {
    local target=$1
    local recipe_line=$2
    local makefile_path=${3:-}
    local log
    log=$(mktemp)
    local -a make_args=()

    if [[ -n $makefile_path ]]; then
        make_args=(-f "$makefile_path")
    fi

    set +e
    make "${make_args[@]}" -i -C "$ORIGINAL_ROOT/zlibWrapper" \
        clean \
        gzclose.o \
        gzlib.o \
        gzread.o \
        gzwrite.o \
        "$target" \
        ZSTDLIBDIR="$HELPER_LIB_ROOT" \
        "LDLIBS=gzclose.o gzlib.o gzread.o gzwrite.o $HELPER_LIB_ROOT/common/xxhash.c -lz" \
        >"$log" 2>&1
    local status=$?
    set -e

    cat "$log"

    local ignored
    ignored=$(grep -c 'Error [0-9][0-9]* (ignored)' "$log" || true)
    if [[ $status -ne 0 ]]; then
        rm -f "$log"
        printf 'zlibWrapper target %s failed with exit status %d\n' "$target" "$status" >&2
        exit "$status"
    fi
    if [[ $ignored -eq 0 ]]; then
        rm -f "$log"
        return 0
    fi

    if [[ $ignored -eq 1 ]] \
        && grep -q 'inflate should report DATA_ERROR' "$log" \
        && grep -q ":${recipe_line}: ${target}" "$log"
    then
        rm -f "$log"
        phase6_log "allowing the known zlib 1.3 inflateSync expectation mismatch in $target"
        return 0
    fi

    rm -f "$log"
    printf 'unexpected ignored errors while running zlibWrapper target %s\n' "$target" >&2
    exit 1
}

run_zlibwrapper_valgrind_target() {
    local bench_dir="$PHASE6_OUT/zlibwrapper-valgrind-bench"
    local makefile_path

    rm -rf "$bench_dir"
    install -d "$bench_dir/lib" "$bench_dir/programs" "$bench_dir/tests"
    ln -sfn "$HELPER_LIB_ROOT/libzstd.a" "$bench_dir/lib/libzstd.a"
    ln -sfn "$ORIGINAL_ROOT/programs/fileio.c" "$bench_dir/programs/fileio.c"
    ln -sfn "$ORIGINAL_ROOT/programs/zstdcli.c" "$bench_dir/programs/zstdcli.c"
    ln -sfn "$ORIGINAL_ROOT/tests/fuzzer.c" "$bench_dir/tests/fuzzer.c"
    ln -sfn "$ORIGINAL_ROOT/tests/zstreamtest.c" "$bench_dir/tests/zstreamtest.c"

    makefile_path=$(mktemp "$PHASE6_OUT/zlibwrapper-valgrind.XXXXXX.mk")
    cat >"$makefile_path" <<EOF
include $ORIGINAL_ROOT/zlibWrapper/Makefile

test-valgrind: clean example fitblk example_zstd fitblk_zstd zwrapbench
	@echo "\\n ---- valgrind tests ----"
	\$(VALGRIND) ./example
	\$(VALGRIND) ./example_zstd
	\$(VALGRIND) ./fitblk 10240 <\$(TEST_FILE)
	\$(VALGRIND) ./fitblk 40960 <\$(TEST_FILE)
	\$(VALGRIND) ./fitblk_zstd 10240 <\$(TEST_FILE)
	\$(VALGRIND) ./fitblk_zstd 40960 <\$(TEST_FILE)
	\$(VALGRIND) ./zwrapbench -qi1b3B1K \$(TEST_FILE)
	\$(VALGRIND) ./zwrapbench -rqi1b1e1 $bench_dir/lib $bench_dir/programs $bench_dir/tests
EOF

    run_zlibwrapper_make test-valgrind 5 "$makefile_path"
    rm -f "$makefile_path"
}

trap cleanup_example_shim EXIT
install_example_shim

phase6_log "building zlibWrapper against the safe helper lib root"
run_zlibwrapper_make test 52
phase6_assert_uses_safe_lib \
    "$ORIGINAL_ROOT/zlibWrapper/example_zstd" \
    "$ORIGINAL_ROOT/zlibWrapper/fitblk_zstd" \
    "$ORIGINAL_ROOT/zlibWrapper/minigzip_zstd" \
    "$ORIGINAL_ROOT/zlibWrapper/zwrapbench"

phase6_log "running zlibWrapper valgrind coverage against the safe helper lib root"
run_zlibwrapper_valgrind_target
phase6_assert_uses_safe_lib \
    "$ORIGINAL_ROOT/zlibWrapper/example_zstd" \
    "$ORIGINAL_ROOT/zlibWrapper/fitblk_zstd" \
    "$ORIGINAL_ROOT/zlibWrapper/zwrapbench"
