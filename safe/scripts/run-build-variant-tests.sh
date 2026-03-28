#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)

bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release --variant mt
bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release --variant nomt
bash "$SAFE_ROOT/scripts/verify-install-layout.sh"

DEFAULT_LIBDIR="$SAFE_ROOT/out/install/release-default/usr/lib/$MULTIARCH"
MT_LIBDIR="$SAFE_ROOT/out/install/release-mt/usr/lib/$MULTIARCH"
NOMT_LIBDIR="$SAFE_ROOT/out/install/release-nomt/usr/lib/$MULTIARCH"

for candidate in "$DEFAULT_LIBDIR" "$MT_LIBDIR" "$NOMT_LIBDIR"; do
    if [[ ! -d $candidate ]]; then
        candidate=${candidate%/$MULTIARCH}
    fi
    readelf -lW "$candidate/libzstd.so.1.5.5" | grep -q 'GNU_STACK' || {
        printf 'missing GNU_STACK program header: %s\n' "$candidate/libzstd.so.1.5.5" >&2
        exit 1
    }
    if readelf -lW "$candidate/libzstd.so.1.5.5" | grep 'GNU_STACK' | grep -q 'RWE'; then
        printf 'shared object requests an executable stack: %s\n' "$candidate/libzstd.so.1.5.5" >&2
        exit 1
    fi
done

grep -q 'Libs.private: -pthread' "$MT_LIBDIR/pkgconfig/libzstd.pc" || {
    printf 'mt pkg-config metadata lost pthread linkage\n' >&2
    exit 1
}
if grep -q 'Libs.private: -pthread' "$NOMT_LIBDIR/pkgconfig/libzstd.pc"; then
    printf 'nomt pkg-config metadata still advertises pthread linkage\n' >&2
    exit 1
fi

if cmp -s "$MT_LIBDIR/libzstd.a" "$NOMT_LIBDIR/libzstd.a"; then
    printf 'mt and nomt static archives are unexpectedly identical\n' >&2
    exit 1
fi
if cmp -s "$MT_LIBDIR/libzstd.so.1.5.5" "$NOMT_LIBDIR/libzstd.so.1.5.5"; then
    printf 'mt and nomt shared objects are unexpectedly identical\n' >&2
    exit 1
fi
