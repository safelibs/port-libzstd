#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

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

if [[ $MODE == artifacts ]]; then
    if [[ ! -d $SAFE_ROOT/out/install/release-default ]]; then
        bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
    fi
    INSTALL_ROOT="$SAFE_ROOT/out/install/release-default"
    MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
else
    if [[ ! -f $SAFE_ROOT/out/deb/default/metadata.env ]]; then
        bash "$SAFE_ROOT/scripts/build-deb.sh"
    fi
    source "$SAFE_ROOT/out/deb/default/metadata.env"
fi

LIBDIR="$INSTALL_ROOT/usr/lib/$MULTIARCH"
if [[ ! -d $LIBDIR ]]; then
    LIBDIR="$INSTALL_ROOT/usr/lib"
fi

assert_exists "$INSTALL_ROOT/usr/include/zstd.h"
assert_exists "$INSTALL_ROOT/usr/include/zdict.h"
assert_exists "$INSTALL_ROOT/usr/include/zstd_errors.h"
assert_exists "$LIBDIR/libzstd.a"
assert_exists "$LIBDIR/libzstd.so.1.5.5"
assert_symlink_target "$LIBDIR/libzstd.so.1" "libzstd.so.1.5.5"
assert_symlink_target "$LIBDIR/libzstd.so" "libzstd.so.1.5.5"
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

if [[ $MODE == debian ]]; then
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
