#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
METADATA_FILE="$SAFE_ROOT/out/deb/default/metadata.env"

SKIP_BUILD=0
PACKAGE_DIR_OVERRIDE=

usage() {
    cat <<'EOF'
usage: install-safe-debs.sh [--skip-build] [--package-dir PATH]
EOF
}

find_one_package() {
    local pattern=$1
    local -a matches=()

    shopt -s nullglob
    matches=($pattern)
    shopt -u nullglob

    if [[ ${#matches[@]} -ne 1 ]]; then
        printf 'expected exactly one package matching %s, found %d\n' \
            "$pattern" "${#matches[@]}" >&2
        exit 1
    fi

    printf '%s\n' "${matches[0]}"
}

ensure_default_phase4_roots() {
    bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
    bash "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh"
    bash "$SAFE_ROOT/scripts/build-deb.sh"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-build)
            SKIP_BUILD=1
            ;;
        --package-dir)
            PACKAGE_DIR_OVERRIDE=${2:?missing package dir}
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            printf 'unknown argument: %s\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

if [[ ${EUID:-$(id -u)} -ne 0 ]]; then
    printf 'install-safe-debs.sh must run as root\n' >&2
    exit 1
fi

if [[ -z $PACKAGE_DIR_OVERRIDE ]]; then
    if [[ $SKIP_BUILD -eq 0 ]]; then
        ensure_default_phase4_roots
    fi
    source "$METADATA_FILE"
    PACKAGE_DIR_OVERRIDE=${PACKAGE_DIR:?}
fi

if [[ ! -d $PACKAGE_DIR_OVERRIDE ]]; then
    printf 'package directory not found: %s\n' "$PACKAGE_DIR_OVERRIDE" >&2
    exit 1
fi

LIB_DEB=$(find_one_package "$PACKAGE_DIR_OVERRIDE/libzstd1_*.deb")
DEV_DEB=$(find_one_package "$PACKAGE_DIR_OVERRIDE/libzstd-dev_*.deb")
CLI_DEB=$(find_one_package "$PACKAGE_DIR_OVERRIDE/zstd_*.deb")

SAFE_VERSION=$(dpkg-deb -f "$LIB_DEB" Version)
for deb in "$DEV_DEB" "$CLI_DEB"; do
    [[ $(dpkg-deb -f "$deb" Version) == "$SAFE_VERSION" ]] || {
        printf 'package version mismatch under %s\n' "$PACKAGE_DIR_OVERRIDE" >&2
        exit 1
    }
done

# Unpack consumers first so dpkg reads their zstd-compressed control archives
# before libzstd1 is replaced in the current root.
dpkg --unpack "$DEV_DEB" "$CLI_DEB" "$LIB_DEB"
dpkg --configure libzstd1 zstd libzstd-dev

for pkg in libzstd1 libzstd-dev zstd; do
    INSTALLED_VERSION=$(dpkg-query -W -f='${Version}' "$pkg")
    if [[ $INSTALLED_VERSION != "$SAFE_VERSION" ]]; then
        printf '%s installed as %s instead of %s\n' \
            "$pkg" "$INSTALLED_VERSION" "$SAFE_VERSION" >&2
        exit 1
    fi
done

printf 'installed safe packages from %s at version %s\n' \
    "$PACKAGE_DIR_OVERRIDE" "$SAFE_VERSION"
