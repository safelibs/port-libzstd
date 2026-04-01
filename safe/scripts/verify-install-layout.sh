#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
SOURCE_DIR_NAME=libzstd-1.5.5+dfsg2
VERSION=1.5.5
SONAME=1

MODE=artifacts
if [[ ${1:-} == --debian ]]; then
    MODE=debian
fi

assert_exists() {
    local path=$1
    [[ -e $path ]] || {
        printf 'missing expected path: %s\n' "$path" >&2
        exit 1
    }
}

assert_exists_any() {
    local path=$1
    if [[ -e $path || -e ${path}.gz ]]; then
        return 0
    fi
    printf 'missing expected path: %s{,.gz}\n' "$path" >&2
    exit 1
}

assert_glob_exists() {
    local pattern=$1

    compgen -G "$pattern" >/dev/null || {
        printf 'missing expected path pattern: %s\n' "$pattern" >&2
        exit 1
    }
}

assert_symlink_target() {
    local path=$1
    local target=$2
    [[ -L $path ]] || {
        printf 'expected symlink: %s\n' "$path" >&2
        exit 1
    }
    [[ $(readlink "$path") == "$target" ]] || {
        printf 'unexpected symlink target for %s: %s\n' "$path" "$(readlink "$path")" >&2
        exit 1
    }
}

ensure_default_phase4_roots() {
    bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
    bash "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh"
    bash "$SAFE_ROOT/scripts/build-deb.sh"
}

ensure_default_phase4_roots

DEFAULT_INSTALL_ROOT="$SAFE_ROOT/out/install/release-default"
DEFAULT_HELPER_ROOT="$SAFE_ROOT/out/original-cli/lib"
DEFAULT_STAGE_ROOT="$SAFE_ROOT/out/debian-src/default/$SOURCE_DIR_NAME"
DEFAULT_DEB_ROOT="$SAFE_ROOT/out/deb/default"
DEFAULT_DEB_INSTALL_ROOT="$DEFAULT_DEB_ROOT/stage-root"
METADATA_FILE="$DEFAULT_DEB_ROOT/metadata.env"

assert_exists "$DEFAULT_INSTALL_ROOT/usr/include/zstd.h"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/bin/zstd"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/bin/zstdcat"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/bin/unzstd"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/bin/zstdmt"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/bin/zstdgrep"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/bin/zstdless"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/bin/pzstd"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/share/man/man1/zstd.1"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/share/man/man1/zstdcat.1"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/share/man/man1/unzstd.1"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/share/man/man1/zstdgrep.1"
assert_exists "$DEFAULT_INSTALL_ROOT/usr/share/man/man1/zstdless.1"
assert_exists "$DEFAULT_HELPER_ROOT/zstd.h"
assert_exists "$DEFAULT_STAGE_ROOT/debian/tests/control"
assert_exists "$DEFAULT_DEB_INSTALL_ROOT/usr/bin/zstd"
assert_exists "$METADATA_FILE"
assert_glob_exists "$DEFAULT_DEB_ROOT/packages/libzstd1_*.deb"
assert_glob_exists "$DEFAULT_DEB_ROOT/packages/libzstd-dev_*.deb"
assert_glob_exists "$DEFAULT_DEB_ROOT/packages/zstd_*.deb"

if [[ $MODE == artifacts ]]; then
    INSTALL_ROOT="$DEFAULT_INSTALL_ROOT"
    MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
else
    source "$METADATA_FILE"
fi

LIBDIR="$INSTALL_ROOT/usr/lib/$MULTIARCH"
if [[ ! -d $LIBDIR ]]; then
    LIBDIR="$INSTALL_ROOT/usr/lib"
fi

assert_exists "$INSTALL_ROOT/usr/include/zstd.h"
assert_exists "$INSTALL_ROOT/usr/include/zdict.h"
assert_exists "$INSTALL_ROOT/usr/include/zstd_errors.h"
assert_exists "$LIBDIR/libzstd.a"
assert_exists "$LIBDIR/libzstd.so.$VERSION"
assert_symlink_target "$LIBDIR/libzstd.so.$SONAME" "libzstd.so.$VERSION"
assert_symlink_target "$LIBDIR/libzstd.so" "libzstd.so.$VERSION"
assert_exists "$LIBDIR/pkgconfig/libzstd.pc"
assert_exists "$LIBDIR/cmake/zstd/zstdConfig.cmake"
assert_exists "$LIBDIR/cmake/zstd/zstdConfigVersion.cmake"
assert_exists "$LIBDIR/cmake/zstd/zstdTargets.cmake"
assert_exists "$LIBDIR/cmake/zstd/zstdTargets-noconfig.cmake"

grep -q '^Version: 1.5.5$' "$LIBDIR/pkgconfig/libzstd.pc" || {
    printf 'pkg-config metadata lost the ABI version\n' >&2
    exit 1
}
grep -q 'Libs: -L${libdir} -lzstd' "$LIBDIR/pkgconfig/libzstd.pc" || {
    printf 'pkg-config metadata does not point at libzstd\n' >&2
    exit 1
}
grep -q 'zstd::libzstd_shared' "$LIBDIR/cmake/zstd/zstdTargets-noconfig.cmake" || {
    printf 'CMake metadata does not export zstd::libzstd_shared\n' >&2
    exit 1
}
grep -q 'zstd::libzstd_static' "$LIBDIR/cmake/zstd/zstdTargets-noconfig.cmake" || {
    printf 'CMake metadata does not export zstd::libzstd_static\n' >&2
    exit 1
}
if grep -q 'Libs.private: -pthread' "$LIBDIR/pkgconfig/libzstd.pc"; then
    printf 'default pkg-config metadata still advertises pthread linkage\n' >&2
    exit 1
fi
nm -A "$LIBDIR/libzstd.a" 2>/dev/null | rg -q 'pthread_' && {
    printf 'default static archive still carries pthread references\n' >&2
    exit 1
}

assert_exists "$DEFAULT_HELPER_ROOT/libzstd.mk"
assert_exists "$DEFAULT_HELPER_ROOT/common/xxhash.c"
assert_exists "$DEFAULT_HELPER_ROOT/common/threading.h"
assert_exists "$DEFAULT_HELPER_ROOT/legacy/zstd_legacy.h"
assert_exists "$DEFAULT_HELPER_ROOT/zstd.h"
assert_exists "$DEFAULT_HELPER_ROOT/zdict.h"
assert_exists "$DEFAULT_HELPER_ROOT/zstd_errors.h"
assert_exists "$DEFAULT_HELPER_ROOT/libzstd.so.$VERSION"
assert_symlink_target "$DEFAULT_HELPER_ROOT/libzstd.so.$SONAME" "libzstd.so.$VERSION"
assert_symlink_target "$DEFAULT_HELPER_ROOT/libzstd.so" "libzstd.so.$VERSION"
grep -Eq '^INPUT[[:space:]]*\([[:space:]]*libzstd\.so[[:space:]]*\)$' "$DEFAULT_HELPER_ROOT/libzstd.a" || {
    printf 'helper libzstd.a is no longer an indirection file\n' >&2
    exit 1
}
cmp -s "$DEFAULT_HELPER_ROOT/zstd.h" "$DEFAULT_INSTALL_ROOT/usr/include/zstd.h" || {
    printf 'helper zstd.h diverged from the safe install tree header\n' >&2
    exit 1
}
cmp -s "$DEFAULT_HELPER_ROOT/zdict.h" "$DEFAULT_INSTALL_ROOT/usr/include/zdict.h" || {
    printf 'helper zdict.h diverged from the safe install tree header\n' >&2
    exit 1
}
cmp -s "$DEFAULT_HELPER_ROOT/zstd_errors.h" "$DEFAULT_INSTALL_ROOT/usr/include/zstd_errors.h" || {
    printf 'helper zstd_errors.h diverged from the safe install tree header\n' >&2
    exit 1
}

assert_exists "$DEFAULT_STAGE_ROOT/programs/Makefile"
assert_exists "$DEFAULT_STAGE_ROOT/zlibWrapper/Makefile"
assert_exists "$DEFAULT_STAGE_ROOT/examples/Makefile"
assert_exists "$DEFAULT_STAGE_ROOT/contrib/pzstd/Makefile"
assert_exists "$DEFAULT_STAGE_ROOT/doc/educational_decoder/Makefile"
assert_exists "$DEFAULT_STAGE_ROOT/CHANGELOG"
assert_exists "$DEFAULT_STAGE_ROOT/README.md"
assert_exists "$DEFAULT_STAGE_ROOT/TESTING.md"

if [[ $MODE == debian ]]; then
    if [[ -e $STAGE_ROOT/original ]]; then
        printf 'staged Debian source tree still contains a safe-rooted original/ subtree\n' >&2
        exit 1
    fi
    assert_exists "$INSTALL_ROOT/usr/share/doc/libzstd-dev/examples"
    assert_exists_any "$INSTALL_ROOT/usr/share/doc/zstd/CHANGELOG"
    assert_exists "$INSTALL_ROOT/usr/share/doc/zstd/CODE_OF_CONDUCT.md"
    assert_exists_any "$INSTALL_ROOT/usr/share/doc/zstd/CONTRIBUTING.md"
    assert_exists_any "$INSTALL_ROOT/usr/share/doc/zstd/README.md"
    assert_exists "$INSTALL_ROOT/usr/share/doc/zstd/TESTING.md"
    assert_exists "$INSTALL_ROOT/usr/bin/zstd"
    assert_exists "$INSTALL_ROOT/usr/bin/zstdgrep"
    assert_exists "$INSTALL_ROOT/usr/bin/zstdless"
    assert_exists "$INSTALL_ROOT/usr/bin/pzstd"
    assert_exists "$INSTALL_ROOT/usr/share/man/man1/zstd.1.gz"
    assert_exists "$INSTALL_ROOT/usr/share/man/man1/zstdmt.1.gz"
    assert_exists "$INSTALL_ROOT/usr/share/man/man1/zstdgrep.1.gz"
    assert_exists "$INSTALL_ROOT/usr/share/man/man1/zstdless.1.gz"
    assert_exists "$INSTALL_ROOT/usr/share/man/man1/pzstd.1.gz"
fi
