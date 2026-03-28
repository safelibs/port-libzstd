#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
ORIGINAL_ROOT="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"
PHASE6_OUT="$SAFE_ROOT/out/phase6"
INSTALL_ROOT="$SAFE_ROOT/out/install/release-default"
HELPER_LIB_ROOT="$SAFE_ROOT/out/original-cli/lib"
TESTS_ROOT="$ORIGINAL_ROOT/tests"
VERSIONS_FIXTURE_ROOT="$SAFE_ROOT/tests/fixtures/versions"
REGRESSION_FIXTURE_ROOT="$SAFE_ROOT/tests/fixtures/regression"
FUZZ_FIXTURE_ROOT="$SAFE_ROOT/tests/fixtures/fuzz-corpora"

MULTIARCH=
if command -v dpkg-architecture >/dev/null 2>&1; then
    MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
fi

phase6_log() {
    printf '[phase6] %s\n' "$*" >&2
}

phase6_refresh_layout() {
    if [[ -n $MULTIARCH ]] && [[ -d $INSTALL_ROOT/usr/lib/$MULTIARCH ]]; then
        LIBDIR="$INSTALL_ROOT/usr/lib/$MULTIARCH"
    else
        LIBDIR="$INSTALL_ROOT/usr/lib"
    fi
    BINDIR="$INSTALL_ROOT/usr/bin"
    INCLUDEDIR="$INSTALL_ROOT/usr/include"
}

phase6_refresh_layout

phase6_with_install_lock() {
    install -d "$PHASE6_OUT"
    (
        flock 9
        "$@"
    ) 9>"$PHASE6_OUT/install.lock"
}

phase6_install_is_stale() {
    phase6_refresh_layout
    if [[ ! -x $BINDIR/zstd || ! -x $BINDIR/pzstd || ! -e $LIBDIR/libzstd.so.1.5.5 ]]; then
        return 0
    fi

    find \
        "$SAFE_ROOT/src" \
        "$SAFE_ROOT/include" \
        "$ORIGINAL_ROOT/programs" \
        "$ORIGINAL_ROOT/contrib/pzstd" \
        -type f \
        \( \
            -name '*.c' -o \
            -name '*.h' -o \
            -name '*.cpp' -o \
            -name '*.hpp' -o \
            -name '*.mk' -o \
            -name 'Makefile' \
        \) \
        -newer "$BINDIR/zstd" -print -quit | grep -q .
}

_phase6_rebuild_install() {
    phase6_refresh_layout
    if phase6_install_is_stale; then
        phase6_log "building safe install tree"
        bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
        bash "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh"
    fi
    phase6_refresh_layout
}

phase6_ensure_safe_install() {
    phase6_with_install_lock _phase6_rebuild_install
}

phase6_ensure_datagen() {
    if [[ ! -x $TESTS_ROOT/datagen ]] || find \
        "$TESTS_ROOT/datagencli.c" \
        "$ORIGINAL_ROOT/programs/datagen.c" \
        -newer "$TESTS_ROOT/datagen" -print -quit | grep -q .
    then
        phase6_log "building original tests/datagen"
        make -C "$TESTS_ROOT" datagen
    fi
}

phase6_export_safe_env() {
    phase6_refresh_layout
    export PATH="$BINDIR${PATH:+:$PATH}"
    export PKG_CONFIG_SYSROOT_DIR="$INSTALL_ROOT"
    export PKG_CONFIG_LIBDIR="$LIBDIR/pkgconfig"
    export CMAKE_PREFIX_PATH="$INSTALL_ROOT/usr${CMAKE_PREFIX_PATH:+:$CMAKE_PREFIX_PATH}"
}

phase6_have_command() {
    command -v "$1" >/dev/null 2>&1
}

phase6_require_command() {
    phase6_have_command "$1" || {
        printf 'missing required command: %s\n' "$1" >&2
        exit 1
    }
}

phase6_have_pkg() {
    dpkg -s "$1" >/dev/null 2>&1
}

phase6_prepare_compat_bin_dir() {
    local out_dir=${1:?missing output directory}
    phase6_refresh_layout
    install -d "$out_dir"

    local zstd_bin="$BINDIR/zstd"
    local zstdgrep_bin="$BINDIR/zstdgrep"
    local zstdless_bin="$BINDIR/zstdless"
    local link
    for link in \
        zstd zstdmt unzstd zstdcat \
        gzip gunzip zcat gzcat \
        xz unxz lzma unlzma \
        lz4 unlz4
    do
        ln -sfn "$zstd_bin" "$out_dir/$link"
    done
    ln -sfn "$zstdgrep_bin" "$out_dir/zstdgrep"
    ln -sfn "$zstdless_bin" "$out_dir/zstdless"
    ln -sfn "$zstdgrep_bin" "$out_dir/zegrep"
    ln -sfn "$zstdgrep_bin" "$out_dir/zfgrep"
}

phase6_assert_uses_safe_lib() {
    phase6_refresh_layout
    local candidate
    for candidate in "$@"; do
        [[ -x $candidate ]] || {
            printf 'missing expected executable: %s\n' "$candidate" >&2
            exit 1
        }
        local resolved
        resolved=$(ldd "$candidate" 2>/dev/null | awk '/libzstd\.so/ {print $3; exit}')
        if [[ -n $resolved ]]; then
            if [[ $resolved == "$ORIGINAL_ROOT"/lib/* ]]; then
                printf 'binary %s still resolves libzstd from upstream tree: %s\n' "$candidate" "$resolved" >&2
                exit 1
            fi
        fi
    done
}

phase6_detect_cli_feature_flags() {
    phase6_refresh_layout
    PHASE6_CLI_DEFS=()
    PHASE6_CLI_LIBS=()

    if phase6_have_pkg zlib1g-dev; then
        PHASE6_CLI_DEFS+=(-DZSTD_GZCOMPRESS -DZSTD_GZDECOMPRESS)
        PHASE6_CLI_LIBS+=(-lz)
    fi
    if phase6_have_pkg liblzma-dev; then
        PHASE6_CLI_DEFS+=(-DZSTD_LZMACOMPRESS -DZSTD_LZMADECOMPRESS)
        PHASE6_CLI_LIBS+=(-llzma)
    fi
    if phase6_have_pkg liblz4-dev; then
        PHASE6_CLI_DEFS+=(-DZSTD_LZ4COMPRESS -DZSTD_LZ4DECOMPRESS)
        PHASE6_CLI_LIBS+=(-llz4)
    fi
}

phase6_compile_cli_variant() {
    local output=${1:?missing output name}
    local threaded=${2:?missing thread mode}
    local extra_flags=${3:-}
    shift 3
    local -a sources=("$@")

    phase6_detect_cli_feature_flags
    local programs_dir="$ORIGINAL_ROOT/programs"
    local -a cmd=(
        gcc
        -O3
        -Wall
        -Wextra
        -I"$HELPER_LIB_ROOT"
        -I"$HELPER_LIB_ROOT/common"
        -I"$programs_dir"
        -Wno-deprecated-declarations
        -Wno-unused-parameter
    )

    if [[ $threaded == 1 ]]; then
        cmd+=(-DZSTD_MULTITHREAD)
    fi
    cmd+=("${PHASE6_CLI_DEFS[@]}")

    if [[ -n $extra_flags ]]; then
        local -a extra_arr=()
        read -r -a extra_arr <<< "$extra_flags"
        cmd+=("${extra_arr[@]}")
    fi

    cmd+=("${sources[@]}")
    cmd+=("$HELPER_LIB_ROOT/libzstd.a")

    if [[ $threaded == 1 ]]; then
        cmd+=(-pthread)
    fi
    cmd+=("${PHASE6_CLI_LIBS[@]}")
    cmd+=(-o "$programs_dir/$output")

    "${cmd[@]}"
}

phase6_build_original_cli_variants() {
    phase6_ensure_safe_install
    phase6_export_safe_env

    local programs_dir="$ORIGINAL_ROOT/programs"
    local helper_common="$HELPER_LIB_ROOT/common"
    local -a internals=(
        "$helper_common/xxhash.c"
        "$helper_common/pool.c"
        "$helper_common/threading.c"
    )
    local -a core_sources=(
        "$programs_dir/fileio.c"
        "$programs_dir/fileio_asyncio.c"
        "$programs_dir/timefn.c"
        "$programs_dir/util.c"
        "$programs_dir/zstdcli.c"
    )
    local -a full_sources=(
        "$programs_dir/benchfn.c"
        "$programs_dir/benchzstd.c"
        "$programs_dir/datagen.c"
        "$programs_dir/dibio.c"
        "$programs_dir/fileio.c"
        "$programs_dir/fileio_asyncio.c"
        "$programs_dir/timefn.c"
        "$programs_dir/util.c"
        "$programs_dir/zstdcli.c"
        "$programs_dir/zstdcli_trace.c"
    )

    phase6_compile_cli_variant zstd 1 "" \
        "${internals[@]}" "${full_sources[@]}"
    phase6_compile_cli_variant zstd-nolegacy 1 "-UZSTD_LEGACY_SUPPORT -DZSTD_LEGACY_SUPPORT=0" \
        "${internals[@]}" "${full_sources[@]}"
    phase6_compile_cli_variant zstd-compress 1 "-DZSTD_NOBENCH -DZSTD_NODICT -DZSTD_NODECOMPRESS -DZSTD_NOTRACE -UZSTD_LEGACY_SUPPORT -DZSTD_LEGACY_SUPPORT=0" \
        "${internals[@]}" "${core_sources[@]}"
    phase6_compile_cli_variant zstd-decompress 1 "-DZSTD_NOBENCH -DZSTD_NODICT -DZSTD_NOCOMPRESS -DZSTD_NOTRACE -UZSTD_LEGACY_SUPPORT -DZSTD_LEGACY_SUPPORT=0" \
        "${internals[@]}" "${core_sources[@]}"
    phase6_compile_cli_variant zstd-dictBuilder 1 "-DZSTD_NOBENCH -DZSTD_NODECOMPRESS -DZSTD_NOTRACE" \
        "${internals[@]}" "${core_sources[@]}" "$programs_dir/dibio.c"
    phase6_compile_cli_variant zstd-frugal 1 "-DZSTD_NOBENCH -DZSTD_NODICT -DZSTD_NOTRACE -UZSTD_LEGACY_SUPPORT -DZSTD_LEGACY_SUPPORT=0" \
        "${internals[@]}" "${core_sources[@]}"
    phase6_compile_cli_variant zstd-nomt 0 "" \
        "${internals[@]}" "${full_sources[@]}"

    ln -sfn zstd "$programs_dir/zstdmt"
}
