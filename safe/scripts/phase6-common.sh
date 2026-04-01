#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
ORIGINAL_ROOT="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"
PHASE6_OUT="$SAFE_ROOT/out/phase6"
INSTALL_ROOT="$SAFE_ROOT/out/install/release-default"
HELPER_LIB_ROOT="$SAFE_ROOT/out/original-cli/lib"
PHASE4_METADATA_FILE="$SAFE_ROOT/out/deb/default/metadata.env"
TESTS_ROOT="$ORIGINAL_ROOT/tests"
VERSIONS_FIXTURE_ROOT="$SAFE_ROOT/tests/fixtures/versions"
REGRESSION_FIXTURE_ROOT="$SAFE_ROOT/tests/fixtures/regression"
FUZZ_FIXTURE_ROOT="$SAFE_ROOT/tests/fixtures/fuzz-corpora"
PHASE6_VARIANTS_ROOT="$PHASE6_OUT/original-cli-variants"
PHASE6_VARIANTS_PROGRAMS=
PHASE6_VARIANTS_TESTS=
PHASE6_DEB_STAGE_ROOT=
PHASE6_DEB_PACKAGE_DIR=
PHASE6_DEB_INSTALL_ROOT=
PHASE6_DEB_LIBDIR=
PHASE6_DEB_BUILD_SIGNATURE=
PHASE6_PHASE4_INPUTS_READY=0
PHASE6_CANONICAL_INSTALL_ROOT=
PHASE6_CANONICAL_HELPER_ROOT=

MULTIARCH=
if command -v dpkg-architecture >/dev/null 2>&1; then
    MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
fi

phase6_log() {
    printf '[phase6] %s\n' "$*" >&2
}

phase6_stamp_path() {
    local name=${1:?missing stamp name}
    local stamp_root="$PHASE6_OUT/verification-stamps"

    install -d "$stamp_root"
    printf '%s/%s.stamp\n' "$stamp_root" "$name"
}

phase6_stamp_is_fresh() {
    local stamp_file=${1:?missing stamp file}
    shift

    [[ -f $stamp_file ]] || return 1

    local dep
    for dep in "$@"; do
        [[ -e $dep ]] || return 1
        if [[ -d $dep ]]; then
            if find "$dep" -type f -newer "$stamp_file" -print -quit | grep -q .; then
                return 1
            fi
        elif [[ $dep -nt $stamp_file ]]; then
            return 1
        fi
    done

    return 0
}

phase6_tracked_repo_paths_are_fresh() {
    local stamp_file=${1:?missing stamp file}
    shift

    [[ -f $stamp_file ]] || return 1
    [[ $# -gt 0 ]] || return 0

    local -a pathspecs=()
    local path
    local rel
    local saw_tracked=0

    for path in "$@"; do
        if [[ $path == "$REPO_ROOT" ]]; then
            pathspecs+=(.)
        elif [[ $path == "$REPO_ROOT/"* ]]; then
            pathspecs+=("${path#"$REPO_ROOT/"}")
        else
            pathspecs+=("$path")
        fi
    done

    while IFS= read -r -d '' rel; do
        saw_tracked=1
        if [[ ! -e $REPO_ROOT/$rel || $REPO_ROOT/$rel -nt $stamp_file ]]; then
            return 1
        fi
    done < <(git -C "$REPO_ROOT" ls-files -z -- "${pathspecs[@]}")

    [[ $saw_tracked -eq 1 ]]
}

phase6_touch_stamp() {
    local stamp_file=${1:?missing stamp file}
    install -d "$(dirname "$stamp_file")"
    touch "$stamp_file"
}

phase6_abspath() {
    local path=${1:?missing path}

    if [[ $path == /* ]]; then
        printf '%s\n' "$path"
        return 0
    fi

    printf '%s/%s\n' "$(cd "$(dirname "$path")" && pwd)" "$(basename "$path")"
}

phase6_refresh_hint() {
    cat >&2 <<EOF
rerun the canonical Phase 4 refresh sequence:
  bash safe/scripts/build-artifacts.sh --release
  bash safe/scripts/build-original-cli-against-safe.sh
  bash safe/scripts/build-deb.sh
EOF
}

phase6_require_path() {
    local path=${1:?missing path}
    local description=${2:-$path}

    [[ -e $path ]] || {
        printf 'missing required Phase 4 input (%s): %s\n' "$description" "$path" >&2
        phase6_refresh_hint
        exit 1
    }
}

phase6_require_glob() {
    local pattern=${1:?missing glob pattern}
    local description=${2:-$pattern}

    compgen -G "$pattern" >/dev/null || {
        printf 'missing required Phase 4 input (%s): %s\n' "$description" "$pattern" >&2
        phase6_refresh_hint
        exit 1
    }
}

phase6_resolve_libdir() {
    local root=${1:?missing root}

    if [[ -n $MULTIARCH ]] && [[ -d $root/usr/lib/$MULTIARCH ]]; then
        printf '%s\n' "$root/usr/lib/$MULTIARCH"
    else
        printf '%s\n' "$root/usr/lib"
    fi
}

phase6_refresh_layout() {
    LIBDIR=$(phase6_resolve_libdir "$INSTALL_ROOT")
    BINDIR="$INSTALL_ROOT/usr/bin"
    INCLUDEDIR="$INSTALL_ROOT/usr/include"
    if [[ -n ${PHASE6_DEB_INSTALL_ROOT:-} ]]; then
        PHASE6_DEB_LIBDIR=$(phase6_resolve_libdir "$PHASE6_DEB_INSTALL_ROOT")
    else
        PHASE6_DEB_LIBDIR=
    fi
}

phase6_refresh_layout

phase6_load_phase4_metadata() {
    if [[ ${PHASE6_PHASE4_INPUTS_READY:-0} -eq 1 ]]; then
        return 0
    fi

    phase6_require_path "$PHASE4_METADATA_FILE" "Debian package metadata"

    local -a meta=()
    mapfile -t meta < <(
        bash -c '
            set -euo pipefail
            source "$1"
            printf "%s\n%s\n%s\n%s\n%s\n%s\n%s\n" \
                "$STAGE_ROOT" \
                "$PACKAGE_DIR" \
                "$INSTALL_ROOT" \
                "${MULTIARCH:-}" \
                "${CANONICAL_INSTALL_ROOT:-}" \
                "${CANONICAL_HELPER_ROOT:-}" \
                "${BUILD_SIGNATURE:-}"
        ' bash "$PHASE4_METADATA_FILE"
    )

    [[ ${#meta[@]} -eq 7 ]] || {
        printf 'malformed Phase 4 metadata file: %s\n' "$PHASE4_METADATA_FILE" >&2
        phase6_refresh_hint
        exit 1
    }

    PHASE6_DEB_STAGE_ROOT=${meta[0]}
    PHASE6_DEB_PACKAGE_DIR=${meta[1]}
    PHASE6_DEB_INSTALL_ROOT=${meta[2]}
    if [[ -n ${meta[3]} ]]; then
        MULTIARCH=${meta[3]}
    fi
    PHASE6_CANONICAL_INSTALL_ROOT=${meta[4]}
    PHASE6_CANONICAL_HELPER_ROOT=${meta[5]}
    PHASE6_DEB_BUILD_SIGNATURE=${meta[6]}

    [[ $PHASE6_CANONICAL_INSTALL_ROOT == "$INSTALL_ROOT" ]] || {
        printf 'Phase 4 metadata points at unexpected install root: %s\n' "$PHASE6_CANONICAL_INSTALL_ROOT" >&2
        phase6_refresh_hint
        exit 1
    }
    [[ $PHASE6_CANONICAL_HELPER_ROOT == "$HELPER_LIB_ROOT" ]] || {
        printf 'Phase 4 metadata points at unexpected helper root: %s\n' "$PHASE6_CANONICAL_HELPER_ROOT" >&2
        phase6_refresh_hint
        exit 1
    }

    phase6_refresh_layout
    PHASE6_PHASE4_INPUTS_READY=1
}

phase6_assert_staged_copy_matches_repo() {
    local repo_path=${1:?missing repository path}
    local staged_path
    local rel

    repo_path=$(phase6_abspath "$repo_path")
    [[ $repo_path == "$SAFE_ROOT/"* ]] || return 0

    rel=${repo_path#"$SAFE_ROOT/"}
    staged_path="$PHASE6_DEB_STAGE_ROOT/$rel"
    phase6_require_path "$staged_path" "staged Debian source copy for $rel"

    cmp -s "$repo_path" "$staged_path" || {
        printf 'staged Debian source tree is stale for %s\n' "$rel" >&2
        phase6_refresh_hint
        exit 1
    }
}

phase6_require_phase4_inputs() {
    local caller=${1:-}

    phase6_load_phase4_metadata
    phase6_assert_staged_copy_matches_repo "$SCRIPT_DIR/phase6-common.sh"
    if [[ -n $caller ]]; then
        phase6_assert_staged_copy_matches_repo "$caller"
    fi

    phase6_require_path "$INSTALL_ROOT" "release install tree"
    phase6_require_path "$HELPER_LIB_ROOT" "original CLI helper root"
    phase6_require_path "$PHASE6_DEB_STAGE_ROOT" "staged Debian source tree"
    phase6_require_path "$PHASE6_DEB_INSTALL_ROOT" "staged Debian install root"
    phase6_require_path "$BINDIR/zstd" "release install zstd binary"
    phase6_require_path "$BINDIR/pzstd" "release install pzstd binary"
    phase6_require_path "$BINDIR/zstdgrep" "release install zstdgrep binary"
    phase6_require_path "$BINDIR/zstdless" "release install zstdless binary"
    phase6_require_path "$INCLUDEDIR/zstd.h" "release install zstd.h"
    phase6_require_path "$INCLUDEDIR/zdict.h" "release install zdict.h"
    phase6_require_path "$INCLUDEDIR/zstd_errors.h" "release install zstd_errors.h"
    phase6_require_path "$LIBDIR/libzstd.so.1.5.5" "release install shared library"
    phase6_require_path "$LIBDIR/libzstd.a" "release install static archive"
    phase6_require_path "$HELPER_LIB_ROOT/libzstd.so.1.5.5" "helper shared library"
    phase6_require_path "$HELPER_LIB_ROOT/libzstd.a" "helper archive indirection file"
    phase6_require_path "$HELPER_LIB_ROOT/libzstd.mk" "helper libzstd.mk"
    phase6_require_path "$HELPER_LIB_ROOT/common/xxhash.c" "helper common/xxhash.c"
    phase6_require_path "$HELPER_LIB_ROOT/common/threading.h" "helper common/threading.h"
    phase6_require_path "$HELPER_LIB_ROOT/legacy/zstd_legacy.h" "helper legacy/zstd_legacy.h"
    phase6_require_path "$PHASE6_DEB_STAGE_ROOT/debian/tests/control" "staged Debian autopkgtest control"
    phase6_require_path "$PHASE6_DEB_INSTALL_ROOT/usr/bin/zstd" "staged package zstd binary"
    phase6_require_glob "$PHASE6_DEB_PACKAGE_DIR/libzstd1_*.deb" "libzstd1 Debian package"
    phase6_require_glob "$PHASE6_DEB_PACKAGE_DIR/libzstd-dev_*.deb" "libzstd-dev Debian package"
    phase6_require_glob "$PHASE6_DEB_PACKAGE_DIR/zstd_*.deb" "zstd Debian package"

    cmp -s "$HELPER_LIB_ROOT/libzstd.so.1.5.5" "$LIBDIR/libzstd.so.1.5.5" || {
        printf 'helper shared library diverged from the Phase 4 install tree: %s\n' "$HELPER_LIB_ROOT/libzstd.so.1.5.5" >&2
        phase6_refresh_hint
        exit 1
    }
    cmp -s "$HELPER_LIB_ROOT/zstd.h" "$INCLUDEDIR/zstd.h" || {
        printf 'helper zstd.h diverged from the Phase 4 install tree\n' >&2
        phase6_refresh_hint
        exit 1
    }
    cmp -s "$HELPER_LIB_ROOT/zdict.h" "$INCLUDEDIR/zdict.h" || {
        printf 'helper zdict.h diverged from the Phase 4 install tree\n' >&2
        phase6_refresh_hint
        exit 1
    }
    cmp -s "$HELPER_LIB_ROOT/zstd_errors.h" "$INCLUDEDIR/zstd_errors.h" || {
        printf 'helper zstd_errors.h diverged from the Phase 4 install tree\n' >&2
        phase6_refresh_hint
        exit 1
    }
    grep -Eq '^INPUT[[:space:]]*\([[:space:]]*libzstd\.so[[:space:]]*\)$' "$HELPER_LIB_ROOT/libzstd.a" || {
        printf 'helper libzstd.a no longer redirects to the shared Phase 4 library\n' >&2
        phase6_refresh_hint
        exit 1
    }
}

phase6_prepare_upstream_tests_helper_root() {
    local overlay_root=${1:?missing upstream helper overlay root}
    local entry
    local dir

    phase6_require_phase4_inputs
    rm -rf "$overlay_root"
    install -d "$overlay_root"

    for entry in \
        Makefile \
        libzstd.mk \
        libzstd.a \
        libzstd.so \
        libzstd.so.1 \
        libzstd.so.1.5.5 \
        zstd.h \
        zdict.h \
        zstd_errors.h \
        common \
        legacy
    do
        ln -sfn "$HELPER_LIB_ROOT/$entry" "$overlay_root/$entry"
    done

    for dir in compress decompress dictBuilder deprecated; do
        ln -sfn "$ORIGINAL_ROOT/lib/$dir" "$overlay_root/$dir"
    done

    printf '%s\n' "$overlay_root"
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
    phase6_require_phase4_inputs
    phase6_refresh_layout
    export PATH="$BINDIR${PATH:+:$PATH}"
    export LD_LIBRARY_PATH="$LIBDIR:$HELPER_LIB_ROOT${PHASE6_DEB_LIBDIR:+:$PHASE6_DEB_LIBDIR}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    export PKG_CONFIG_SYSROOT_DIR="$INSTALL_ROOT"
    export PKG_CONFIG_LIBDIR="$LIBDIR/pkgconfig"
    export CMAKE_PREFIX_PATH="$INSTALL_ROOT/usr${CMAKE_PREFIX_PATH:+:$CMAKE_PREFIX_PATH}"
    export PHASE6_DEB_STAGE_ROOT
    export PHASE6_DEB_PACKAGE_DIR
    export PHASE6_DEB_INSTALL_ROOT
    export PHASE6_DEB_BUILD_SIGNATURE
    export PHASE4_METADATA_FILE
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
    phase6_require_phase4_inputs
    local candidate
    for candidate in "$@"; do
        [[ -x $candidate ]] || {
            printf 'missing expected executable: %s\n' "$candidate" >&2
            exit 1
        }
        local resolved
        resolved=$(
            env LD_LIBRARY_PATH="$LIBDIR:$HELPER_LIB_ROOT${PHASE6_DEB_LIBDIR:+:$PHASE6_DEB_LIBDIR}" \
                ldd "$candidate" 2>/dev/null | awk '/libzstd\.so/ {print $3; exit}'
        )
        [[ -n $resolved ]] || {
            printf 'unable to resolve libzstd for %s via the Phase 4 artifact roots\n' "$candidate" >&2
            exit 1
        }

        resolved=$(readlink -f "$resolved")
        if [[ $resolved == "$ORIGINAL_ROOT"/lib/* ]]; then
            printf 'binary %s still resolves libzstd from upstream tree: %s\n' "$candidate" "$resolved" >&2
            exit 1
        fi
        case "$resolved" in
            "$LIBDIR"/libzstd.so.*|"$HELPER_LIB_ROOT"/libzstd.so.*)
                ;;
            "$PHASE6_DEB_LIBDIR"/libzstd.so.*)
                ;;
            *)
                printf 'binary %s did not resolve libzstd from a Phase 4 artifact root: %s\n' "$candidate" "$resolved" >&2
                exit 1
                ;;
        esac
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
    local output_dir=${PHASE6_VARIANTS_PROGRAMS:?phase6 cli variants output root is not initialized}

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
    cmd+=(-o "$output_dir/$output")

    "${cmd[@]}"
}

phase6_build_original_cli_variants() {
    phase6_require_phase4_inputs
    phase6_export_safe_env

    local programs_dir="$ORIGINAL_ROOT/programs"
    local helper_common="$HELPER_LIB_ROOT/common"
    local variants_root="$PHASE6_VARIANTS_ROOT"
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

    rm -rf "$variants_root"
    PHASE6_VARIANTS_PROGRAMS="$variants_root/programs"
    PHASE6_VARIANTS_TESTS="$variants_root/tests"
    install -d "$PHASE6_VARIANTS_PROGRAMS" "$PHASE6_VARIANTS_TESTS"
    install -m 0755 "$TESTS_ROOT/test-variants.sh" "$PHASE6_VARIANTS_TESTS/test-variants.sh"
    ln -sfn "$BINDIR/zstd" "$PHASE6_VARIANTS_PROGRAMS/zstd"
    ln -sfn "$BINDIR/zstd" "$PHASE6_VARIANTS_PROGRAMS/zstdmt"

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

    phase6_assert_uses_safe_lib \
        "$PHASE6_VARIANTS_PROGRAMS/zstd" \
        "$PHASE6_VARIANTS_PROGRAMS/zstd-nolegacy" \
        "$PHASE6_VARIANTS_PROGRAMS/zstd-compress" \
        "$PHASE6_VARIANTS_PROGRAMS/zstd-decompress" \
        "$PHASE6_VARIANTS_PROGRAMS/zstd-dictBuilder" \
        "$PHASE6_VARIANTS_PROGRAMS/zstd-frugal" \
        "$PHASE6_VARIANTS_PROGRAMS/zstd-nomt"
}
