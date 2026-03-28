#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
UPSTREAM_SO="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5"

bash "$SAFE_ROOT/scripts/build-deb.sh"
DEB_BUILD_PROFILES=noudeb bash "$SAFE_ROOT/scripts/build-deb.sh"

source "$SAFE_ROOT/out/deb/default/metadata.env"
DEFAULT_PACKAGE_DIR=$PACKAGE_DIR
DEFAULT_INSTALL_ROOT=$INSTALL_ROOT
source "$SAFE_ROOT/out/deb/noudeb/metadata.env"
NOUDEB_PACKAGE_DIR=$PACKAGE_DIR

for pkg in libzstd1 libzstd-dev zstd; do
    compgen -G "$DEFAULT_PACKAGE_DIR/${pkg}_*.deb" >/dev/null || {
        printf 'missing default-profile package: %s\n' "$pkg" >&2
        exit 1
    }
    compgen -G "$NOUDEB_PACKAGE_DIR/${pkg}_*.deb" >/dev/null || {
        printf 'missing noudeb package: %s\n' "$pkg" >&2
        exit 1
    }
done

compgen -G "$DEFAULT_PACKAGE_DIR/libzstd1-udeb_*.udeb" >/dev/null || {
    printf 'default profile did not emit libzstd1-udeb\n' >&2
    exit 1
}

if compgen -G "$NOUDEB_PACKAGE_DIR/libzstd1-udeb_*.udeb" >/dev/null; then
    printf 'noudeb profile unexpectedly emitted libzstd1-udeb\n' >&2
    exit 1
fi

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

UDEB=$(printf '%s\n' "$DEFAULT_PACKAGE_DIR"/libzstd1-udeb_*.udeb | head -n1)
dpkg-deb -x "$UDEB" "$TMPDIR/udeb"
MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
UDEB_SO="$TMPDIR/udeb/lib/$MULTIARCH/libzstd.so.1.5.5"
if [[ ! -f $UDEB_SO ]]; then
    UDEB_SO="$TMPDIR/udeb/usr/lib/$MULTIARCH/libzstd.so.1.5.5"
fi

if [[ ! -f $UDEB_SO ]]; then
    printf 'libzstd1-udeb does not contain the shared library payload\n' >&2
    exit 1
fi

cmp -s "$UDEB_SO" "$UPSTREAM_SO" && {
    printf 'libzstd1-udeb payload matches the copied upstream binary\n' >&2
    exit 1
}

cmp -s "$UDEB_SO" "$DEFAULT_INSTALL_ROOT/usr/lib/$MULTIARCH/libzstd.so.1.5.5" || {
    printf 'libzstd1-udeb payload differs from the safe default build output\n' >&2
    exit 1
}
