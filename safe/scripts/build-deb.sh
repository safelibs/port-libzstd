#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
UPSTREAM_ROOT="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"

VERSION=1.5.5
SOURCE_DIR_NAME=libzstd-1.5.5+dfsg2
MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
GNU_TYPE=$(dpkg-architecture -qDEB_HOST_GNU_TYPE)
PROFILES=${DEB_BUILD_PROFILES:-}
SAFE_ENABLE_UDEB=1
if [[ " $PROFILES " == *" noudeb "* ]] || [[ ",$PROFILES," == *,noudeb,* ]]; then
    SAFE_ENABLE_UDEB=0
fi
BUILD_TAG=${PROFILES// /-}
BUILD_TAG=${BUILD_TAG//,/--}
if [[ -z $BUILD_TAG ]]; then
    BUILD_TAG=default
fi

STAGE_PARENT="$SAFE_ROOT/out/debian-src/$BUILD_TAG"
STAGE_ROOT="$STAGE_PARENT/$SOURCE_DIR_NAME"
BUILD_ROOT="$SAFE_ROOT/out/deb/$BUILD_TAG"
PACKAGE_DIR="$BUILD_ROOT/packages"
INSTALL_ROOT="$BUILD_ROOT/stage-root"
METADATA_FILE="$BUILD_ROOT/metadata.env"

rsync_tree() {
    local src=$1
    local dest=$2
    shift 2
    rsync -a --delete "$@" "$src" "$dest"
}

rm -rf "$STAGE_PARENT" "$BUILD_ROOT"
install -d "$STAGE_ROOT" "$PACKAGE_DIR" "$INSTALL_ROOT"

rsync_tree "$SAFE_ROOT/include/" "$STAGE_ROOT/include/"
rsync_tree "$SAFE_ROOT/src/" "$STAGE_ROOT/src/"
rsync_tree "$SAFE_ROOT/scripts/" "$STAGE_ROOT/scripts/"
rsync_tree "$SAFE_ROOT/pkgconfig/" "$STAGE_ROOT/pkgconfig/"
rsync_tree "$SAFE_ROOT/cmake/" "$STAGE_ROOT/cmake/"
rsync_tree "$SAFE_ROOT/debian/" "$STAGE_ROOT/debian/"
if [[ $SAFE_ENABLE_UDEB -eq 1 ]]; then
    : >"$STAGE_ROOT/.safelibs-enable-udeb"
else
    rm -f "$STAGE_ROOT/.safelibs-enable-udeb"
fi
install -m 644 "$SAFE_ROOT/Cargo.toml" "$STAGE_ROOT/Cargo.toml"
install -m 644 "$SAFE_ROOT/build.rs" "$STAGE_ROOT/build.rs"
if [[ -f $SAFE_ROOT/rust-toolchain.toml ]]; then
    install -m 644 "$SAFE_ROOT/rust-toolchain.toml" "$STAGE_ROOT/rust-toolchain.toml"
fi
if [[ -f $SAFE_ROOT/Cargo.lock ]]; then
    install -m 644 "$SAFE_ROOT/Cargo.lock" "$STAGE_ROOT/Cargo.lock"
fi

install -d "$STAGE_ROOT/lib"
rsync_tree "$UPSTREAM_ROOT/lib/common/" "$STAGE_ROOT/lib/common/"
rsync_tree "$UPSTREAM_ROOT/lib/legacy/" "$STAGE_ROOT/lib/legacy/"
install -m 644 "$UPSTREAM_ROOT/lib/libzstd.mk" "$STAGE_ROOT/lib/libzstd.mk"
install -m 644 "$SAFE_ROOT/include/zstd.h" "$STAGE_ROOT/lib/zstd.h"
install -m 644 "$SAFE_ROOT/include/zdict.h" "$STAGE_ROOT/lib/zdict.h"
install -m 644 "$SAFE_ROOT/include/zstd_errors.h" "$STAGE_ROOT/lib/zstd_errors.h"

install -d "$STAGE_ROOT/original/libzstd-1.5.5+dfsg2"
rsync_tree "$UPSTREAM_ROOT/lib/" "$STAGE_ROOT/original/libzstd-1.5.5+dfsg2/lib/" \
    --exclude='*.o' \
    --exclude='*.a' \
    --exclude='*.so' \
    --exclude='*.so.*' \
    --exclude='obj'

rsync_tree "$UPSTREAM_ROOT/programs/" "$STAGE_ROOT/programs/" \
    --exclude='.gitignore' \
    --exclude='*.o' \
    --exclude='*.d' \
    --exclude='zstd' \
    --exclude='zstd-compress' \
    --exclude='zstd-decompress' \
    --exclude='zstd-dictBuilder' \
    --exclude='zstd-frugal' \
    --exclude='zstd-nolegacy' \
    --exclude='zstd-small'
rsync_tree "$UPSTREAM_ROOT/zlibWrapper/" "$STAGE_ROOT/zlibWrapper/" \
    --exclude='.gitignore' \
    --exclude='*.o' \
    --exclude='*.d'
rsync_tree "$UPSTREAM_ROOT/examples/" "$STAGE_ROOT/examples/" \
    --exclude='.gitignore' \
    --exclude='*.o' \
    --exclude='*.d'
install -d "$STAGE_ROOT/contrib"
rsync_tree "$UPSTREAM_ROOT/contrib/pzstd/" "$STAGE_ROOT/contrib/pzstd/" \
    --exclude='.gitignore' \
    --exclude='*.o' \
    --exclude='*.d' \
    --exclude='*.Td' \
    --exclude='googletest' \
    --exclude='pzstd'
install -d "$STAGE_ROOT/doc"
rsync_tree "$UPSTREAM_ROOT/doc/educational_decoder/" "$STAGE_ROOT/doc/educational_decoder/" \
    --exclude='.gitignore' \
    --exclude='*.o' \
    --exclude='*.d' \
    --exclude='harness'

for doc_file in CHANGELOG CODE_OF_CONDUCT.md CONTRIBUTING.md COPYING LICENSE README.md TESTING.md; do
    if [[ -f $UPSTREAM_ROOT/$doc_file ]]; then
        install -m 644 "$UPSTREAM_ROOT/$doc_file" "$STAGE_ROOT/$doc_file"
    fi
done

(
    cd "$STAGE_ROOT"
    dpkg-buildpackage -d -b -us -uc
)

if [[ $SAFE_ENABLE_UDEB -eq 1 ]]; then
    (
        cd "$STAGE_ROOT"
        rm -rf debian/libzstd1-udeb
        install -d "debian/libzstd1-udeb/lib/$MULTIARCH"
        cp -a \
            "debian/libzstd1/usr/lib/$MULTIARCH/libzstd.so.1" \
            "debian/libzstd1/usr/lib/$MULTIARCH/libzstd.so.1.5.5" \
            "debian/libzstd1-udeb/lib/$MULTIARCH/"
        fakeroot sh -ec '
            dh_shlibdeps -plibzstd1-udeb
            install -d debian/libzstd1-udeb/DEBIAN
            echo misc:Depends= >> debian/libzstd1-udeb.substvars
            echo misc:Pre-Depends= >> debian/libzstd1-udeb.substvars
            dh_gencontrol -plibzstd1-udeb -Pdebian/libzstd1-udeb
            dh_md5sums -plibzstd1-udeb
            dh_builddeb -plibzstd1-udeb
        '
    )
fi

find "$STAGE_PARENT" -maxdepth 1 -type f \
    \( -name '*.deb' -o -name '*.udeb' -o -name '*.changes' -o -name '*.buildinfo' \) \
    -exec cp '{}' "$PACKAGE_DIR/" ';'

find "$PACKAGE_DIR" -maxdepth 1 -type f -name '*.deb' -print0 |
    while IFS= read -r -d '' deb; do
        dpkg-deb -x "$deb" "$INSTALL_ROOT"
    done

cat >"$METADATA_FILE" <<EOF
BUILD_TAG='$BUILD_TAG'
PROFILES='$PROFILES'
STAGE_ROOT='$STAGE_ROOT'
PACKAGE_DIR='$PACKAGE_DIR'
INSTALL_ROOT='$INSTALL_ROOT'
MULTIARCH='$MULTIARCH'
GNU_TYPE='$GNU_TYPE'
VERSION='$VERSION'
SAFE_ENABLE_UDEB='$SAFE_ENABLE_UDEB'
EOF

printf 'staged source tree: %s\n' "$STAGE_ROOT"
printf 'package outputs: %s\n' "$PACKAGE_DIR"
printf 'stage install root: %s\n' "$INSTALL_ROOT"
