#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
UPSTREAM_SO="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5"
DEFAULT_METADATA_FILE="$SAFE_ROOT/out/deb/default/metadata.env"
NOUDEB_METADATA_FILE="$SAFE_ROOT/out/deb/noudeb/metadata.env"
source "$SAFE_ROOT/scripts/phase6-common.sh"

phase6_require_phase4_inputs "$0"

DEFAULT_PACKAGE_DIR=
DEFAULT_INSTALL_ROOT=
NOUDEB_PACKAGE_DIR=
DEFAULT_INSTALL_SO=
if [[ -f $DEFAULT_METADATA_FILE ]]; then
    # shellcheck disable=SC1090
    source "$DEFAULT_METADATA_FILE"
    DEFAULT_PACKAGE_DIR=$PACKAGE_DIR
    DEFAULT_INSTALL_ROOT=$INSTALL_ROOT
    if [[ -n ${MULTIARCH:-} ]]; then
        DEFAULT_INSTALL_SO="$DEFAULT_INSTALL_ROOT/usr/lib/$MULTIARCH/libzstd.so.1.5.5"
    else
        DEFAULT_INSTALL_SO="$DEFAULT_INSTALL_ROOT/usr/lib/libzstd.so.1.5.5"
    fi
fi
if [[ -f $NOUDEB_METADATA_FILE ]]; then
    # shellcheck disable=SC1090
    source "$NOUDEB_METADATA_FILE"
    NOUDEB_PACKAGE_DIR=$PACKAGE_DIR
fi
STAMP_FILE=$(phase6_stamp_path verify-deb-profiles)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$DEFAULT_METADATA_FILE" \
    "$NOUDEB_METADATA_FILE" \
    "$DEFAULT_PACKAGE_DIR" \
    "$NOUDEB_PACKAGE_DIR" \
    "$DEFAULT_INSTALL_SO" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$SAFE_ROOT/Cargo.toml" \
        "$SAFE_ROOT/include" \
        "$SAFE_ROOT/src" \
        "$SAFE_ROOT/scripts/build-deb.sh" \
        "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/debian"
then
    phase6_log "Debian profile verification already fresh; skipping rerun"
    exit 0
fi

DEB_BUILD_PROFILES=noudeb bash "$SAFE_ROOT/scripts/build-deb.sh"

# shellcheck disable=SC1090
source "$SAFE_ROOT/out/deb/default/metadata.env"
DEFAULT_PACKAGE_DIR=$PACKAGE_DIR
DEFAULT_INSTALL_ROOT=$INSTALL_ROOT
DEFAULT_CANONICAL_INSTALL_ROOT=$CANONICAL_INSTALL_ROOT
DEFAULT_CANONICAL_HELPER_ROOT=$CANONICAL_HELPER_ROOT
# shellcheck disable=SC1090
source "$SAFE_ROOT/out/deb/noudeb/metadata.env"
NOUDEB_PACKAGE_DIR=$PACKAGE_DIR

[[ -d $DEFAULT_CANONICAL_INSTALL_ROOT ]] || {
    printf 'missing canonical install root: %s\n' "$DEFAULT_CANONICAL_INSTALL_ROOT" >&2
    exit 1
}
[[ -d $DEFAULT_CANONICAL_HELPER_ROOT ]] || {
    printf 'missing canonical helper root: %s\n' "$DEFAULT_CANONICAL_HELPER_ROOT" >&2
    exit 1
}

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

phase6_touch_stamp "$STAMP_FILE"
