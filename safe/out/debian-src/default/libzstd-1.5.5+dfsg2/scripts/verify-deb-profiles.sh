#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
UPSTREAM_SO="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5"
DEFAULT_METADATA_FILE="$SAFE_ROOT/out/deb/default/metadata.env"
NOUDEB_METADATA_FILE="$SAFE_ROOT/out/deb/noudeb/metadata.env"
source "$SAFE_ROOT/scripts/phase6-common.sh"

noudeb_refresh_hint() {
    cat >&2 <<EOF
rerun the explicit noudeb Debian refresh before verification:
  DEB_BUILD_PROFILES=noudeb bash safe/scripts/build-deb.sh
EOF
}

require_noudeb_path() {
    local path=${1:?missing path}
    local description=${2:-$path}

    [[ -e $path ]] || {
        printf 'missing required prebuilt noudeb artifact (%s): %s\n' "$description" "$path" >&2
        noudeb_refresh_hint
        exit 1
    }
}

archive_is_valid() {
    local archive=$1

    [[ -f $archive ]] || return 1
    ar t "$archive" >/dev/null 2>&1
}

profile_static_archive_valid() {
    local package_dir=$1
    local install_root=${2:-}
    local archive
    local dev_deb
    local ok
    local tmpdir
    local -a matches=()

    if [[ -n $install_root ]]; then
        archive="$install_root/usr/lib/$MULTIARCH/libzstd.a"
        if [[ ! -f $archive ]]; then
            archive="$install_root/usr/lib/libzstd.a"
        fi
        archive_is_valid "$archive" || return 1
    fi

    [[ -d $package_dir ]] || return 1
    shopt -s nullglob
    matches=("$package_dir"/libzstd-dev_*.deb)
    shopt -u nullglob
    [[ ${#matches[@]} -eq 1 ]] || return 1

    dev_deb=${matches[0]}
    tmpdir=$(mktemp -d)
    ok=0
    if ! dpkg-deb -x "$dev_deb" "$tmpdir"; then
        ok=1
    else
        archive="$tmpdir/usr/lib/$MULTIARCH/libzstd.a"
        if [[ ! -f $archive ]]; then
            archive="$tmpdir/usr/lib/libzstd.a"
        fi
        archive_is_valid "$archive" || ok=1
    fi
    rm -rf "$tmpdir"
    return "$ok"
}

validate_profile_static_archive() {
    local profile=$1
    local package_dir=$2
    local install_root=$3

    profile_static_archive_valid "$package_dir" "$install_root" || {
        printf '%s profile libzstd-dev does not ship a real static archive\n' "$profile" >&2
        exit 1
    }
}

phase6_require_phase4_inputs "$0"
require_noudeb_path "$NOUDEB_METADATA_FILE" "noudeb Debian package metadata"

DEFAULT_PACKAGE_DIR=
DEFAULT_INSTALL_ROOT=
NOUDEB_PACKAGE_DIR=
NOUDEB_INSTALL_ROOT=
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
    [[ ${BUILD_TAG:-} == noudeb ]] || {
        printf 'noudeb metadata has unexpected BUILD_TAG: %s\n' "${BUILD_TAG:-}" >&2
        noudeb_refresh_hint
        exit 1
    }
    [[ ${PROFILES:-} == noudeb ]] || {
        printf 'noudeb metadata has unexpected PROFILES: %s\n' "${PROFILES:-}" >&2
        noudeb_refresh_hint
        exit 1
    }
    [[ ${SAFE_ENABLE_UDEB:-} == 0 ]] || {
        printf 'noudeb metadata unexpectedly enables udeb output\n' >&2
        noudeb_refresh_hint
        exit 1
    }
    NOUDEB_PACKAGE_DIR=$PACKAGE_DIR
    NOUDEB_INSTALL_ROOT=$INSTALL_ROOT
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
    "$SCRIPT_DIR/build-artifacts.sh" \
    "$SCRIPT_DIR/build-deb.sh" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$SAFE_ROOT/Cargo.toml" \
        "$SAFE_ROOT/include" \
        "$SAFE_ROOT/src" \
        "$SAFE_ROOT/scripts/build-artifacts.sh" \
        "$SAFE_ROOT/scripts/build-deb.sh" \
        "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh" \
        "$REPO_ROOT/original/libzstd-1.5.5+dfsg2/debian" \
    && profile_static_archive_valid "$DEFAULT_PACKAGE_DIR" "$DEFAULT_INSTALL_ROOT" \
    && profile_static_archive_valid "$NOUDEB_PACKAGE_DIR" "$NOUDEB_INSTALL_ROOT"
then
    phase6_log "Debian profile verification already fresh; skipping rerun"
    exit 0
fi

# shellcheck disable=SC1090
source "$SAFE_ROOT/out/deb/default/metadata.env"
DEFAULT_PACKAGE_DIR=$PACKAGE_DIR
DEFAULT_INSTALL_ROOT=$INSTALL_ROOT
DEFAULT_CANONICAL_INSTALL_ROOT=$CANONICAL_INSTALL_ROOT
DEFAULT_CANONICAL_HELPER_ROOT=$CANONICAL_HELPER_ROOT
# shellcheck disable=SC1090
source "$SAFE_ROOT/out/deb/noudeb/metadata.env"
[[ ${BUILD_TAG:-} == noudeb ]] || {
    printf 'noudeb metadata has unexpected BUILD_TAG: %s\n' "${BUILD_TAG:-}" >&2
    noudeb_refresh_hint
    exit 1
}
[[ ${PROFILES:-} == noudeb ]] || {
    printf 'noudeb metadata has unexpected PROFILES: %s\n' "${PROFILES:-}" >&2
    noudeb_refresh_hint
    exit 1
}
[[ ${SAFE_ENABLE_UDEB:-} == 0 ]] || {
    printf 'noudeb metadata unexpectedly enables udeb output\n' >&2
    noudeb_refresh_hint
    exit 1
}
NOUDEB_PACKAGE_DIR=$PACKAGE_DIR
NOUDEB_INSTALL_ROOT=$INSTALL_ROOT

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

validate_profile_static_archive default "$DEFAULT_PACKAGE_DIR" "$DEFAULT_INSTALL_ROOT"
validate_profile_static_archive noudeb "$NOUDEB_PACKAGE_DIR" "$NOUDEB_INSTALL_ROOT"

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
