#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
UPSTREAM_ROOT="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"

VERSION=1.5.5
SOURCE_DIR_NAME=libzstd-1.5.5+dfsg2
CANONICAL_INSTALL_ROOT="$SAFE_ROOT/out/install/release-default"
CANONICAL_HELPER_ROOT="$SAFE_ROOT/out/original-cli/lib"
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

compute_build_signature() {
    python3 - "$REPO_ROOT" "$SAFE_ROOT" "$UPSTREAM_ROOT" "$BUILD_TAG" "$SAFE_ENABLE_UDEB" "$MULTIARCH" "$VERSION" <<'PY'
from __future__ import annotations

import hashlib
import pathlib
import subprocess
import sys

repo_root = pathlib.Path(sys.argv[1])
safe_root = pathlib.Path(sys.argv[2])
upstream_root = pathlib.Path(sys.argv[3])
params = sys.argv[4:]

def iter_signature_entries(root: pathlib.Path, path: pathlib.Path) -> list[pathlib.Path]:
    if not path.exists():
        raise SystemExit(f"missing input for deb build signature: {path}")
    if not path.is_dir():
        return [path]

    rel = path.relative_to(repo_root)
    output = subprocess.check_output(
        ["git", "-C", str(repo_root), "ls-files", "-z", "--", rel.as_posix()]
    )
    entries = []
    for item in output.split(b"\0"):
        if not item:
            continue
        entry = repo_root / item.decode("utf-8")
        if entry.is_file():
            entries.append(entry)
    return sorted(entries)

h = hashlib.sha256()
for value in params:
    h.update(value.encode("utf-8"))
    h.update(b"\0")

paths = [
    safe_root / "Cargo.toml",
    safe_root / "build.rs",
    safe_root / "include",
    safe_root / "src",
    safe_root / "third_party",
    safe_root / "scripts",
    safe_root / "pkgconfig",
    safe_root / "cmake",
    safe_root / "debian",
    upstream_root / "lib",
    upstream_root / "programs",
    upstream_root / "zlibWrapper",
    upstream_root / "examples",
    upstream_root / "contrib" / "pzstd",
    upstream_root / "doc" / "educational_decoder",
]

for optional in (
    safe_root / "Cargo.lock",
    safe_root / "rust-toolchain.toml",
    upstream_root / "CHANGELOG",
    upstream_root / "CODE_OF_CONDUCT.md",
    upstream_root / "CONTRIBUTING.md",
    upstream_root / "COPYING",
    upstream_root / "LICENSE",
    upstream_root / "README.md",
    upstream_root / "TESTING.md",
):
    if optional.exists():
        paths.append(optional)

for path in paths:
    root = safe_root if safe_root in path.parents or path == safe_root else upstream_root
    for entry in iter_signature_entries(root, path):
        rel = entry.relative_to(root)
        h.update(root.name.encode("utf-8"))
        h.update(b":")
        h.update(str(rel).encode("utf-8"))
        h.update(b"\0")
        h.update(entry.read_bytes())
        h.update(b"\0")

print(h.hexdigest())
PY
}

build_outputs_present() {
    local package_dir=$1
    local install_root=$2
    local enable_udeb=$3
    local pkg

    [[ -d $package_dir ]] || return 1
    [[ -d $install_root ]] || return 1

    for pkg in libzstd1 libzstd-dev zstd; do
        compgen -G "$package_dir/${pkg}_*.deb" >/dev/null || return 1
    done

    if [[ $enable_udeb -eq 1 ]]; then
        compgen -G "$package_dir/libzstd1-udeb_*.udeb" >/dev/null || return 1
    fi
}

reuse_existing_build() {
    local desired_signature=$1
    local -a meta=()

    [[ -f $METADATA_FILE ]] || return 1

    mapfile -t meta < <(
        bash -c '
            set -euo pipefail
            source "$1"
            printf "%s\n%s\n%s\n%s\n%s\n%s\n%s\n" \
                "${BUILD_SIGNATURE:-}" \
                "$STAGE_ROOT" \
                "$PACKAGE_DIR" \
                "$INSTALL_ROOT" \
                "$SAFE_ENABLE_UDEB" \
                "${CANONICAL_INSTALL_ROOT:-}" \
                "${CANONICAL_HELPER_ROOT:-}"
        ' bash "$METADATA_FILE"
    )

    [[ ${#meta[@]} -eq 7 ]] || return 1
    [[ ${meta[0]} == "$desired_signature" ]] || return 1
    [[ ${meta[1]} == "$STAGE_ROOT" ]] || return 1
    [[ ${meta[2]} == "$PACKAGE_DIR" ]] || return 1
    [[ ${meta[3]} == "$INSTALL_ROOT" ]] || return 1
    [[ ${meta[5]} == "$CANONICAL_INSTALL_ROOT" ]] || return 1
    [[ ${meta[6]} == "$CANONICAL_HELPER_ROOT" ]] || return 1
    [[ -d ${meta[1]} ]] || return 1
    build_outputs_present "${meta[2]}" "${meta[3]}" "${meta[4]}" || return 1

    if [[ $BUILD_TAG == default ]]; then
        rm -f \
            "$SAFE_ROOT/out"/libzstd-dev_*.deb \
            "$SAFE_ROOT/out"/libzstd1_*.deb \
            "$SAFE_ROOT/out"/libzstd1-udeb_*.udeb \
            "$SAFE_ROOT/out"/zstd_*.deb

        link_latest_package "${meta[2]}/libzstd-dev_*.deb" "$SAFE_ROOT/out"
        link_latest_package "${meta[2]}/libzstd1_*.deb" "$SAFE_ROOT/out"
        link_latest_package "${meta[2]}/zstd_*.deb" "$SAFE_ROOT/out"
        if [[ ${meta[4]} -eq 1 ]]; then
            link_latest_package "${meta[2]}/libzstd1-udeb_*.udeb" "$SAFE_ROOT/out"
        fi
    fi

    printf 'reusing up-to-date deb build: %s\n' "${meta[1]}"
    printf 'staged source tree: %s\n' "${meta[1]}"
    printf 'package outputs: %s\n' "${meta[2]}"
    printf 'stage install root: %s\n' "${meta[3]}"
    return 0
}

rsync_tree() {
    local src=$1
    local dest=$2
    shift 2
    rsync -a --delete "$@" "$src" "$dest"
}

link_latest_package() {
    local pattern=$1
    local output_dir=$2
    local -a matches=()

    shopt -s nullglob
    matches=($pattern)
    shopt -u nullglob

    if [[ ${#matches[@]} -ne 1 ]]; then
        printf 'expected exactly one package matching %s, found %d\n' \
            "$pattern" "${#matches[@]}" >&2
        exit 1
    fi

    ln -sfn "$(realpath --relative-to="$output_dir" "${matches[0]}")" \
        "$output_dir/$(basename "${matches[0]}")"
}

BUILD_SIGNATURE=$(compute_build_signature)
if reuse_existing_build "$BUILD_SIGNATURE"; then
    exit 0
fi

rm -rf "$STAGE_PARENT" "$BUILD_ROOT"
install -d "$STAGE_ROOT" "$PACKAGE_DIR" "$INSTALL_ROOT"
DPKG_WRAPPER_DIR="$BUILD_ROOT/dpkg-bin"
install -d "$DPKG_WRAPPER_DIR"
ln -sfn /usr/bin/dpkg-genbuildinfo "$DPKG_WRAPPER_DIR/dpkg-genbuildinfo"
ln -sfn /usr/bin/dpkg-genchanges "$DPKG_WRAPPER_DIR/dpkg-genchanges"

rsync_tree "$SAFE_ROOT/include/" "$STAGE_ROOT/include/"
rsync_tree "$SAFE_ROOT/src/" "$STAGE_ROOT/src/"
rsync_tree "$SAFE_ROOT/third_party/" "$STAGE_ROOT/third_party/"
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

rsync_tree "$UPSTREAM_ROOT/lib/" "$STAGE_ROOT/lib/" \
    --exclude='*.o' \
    --exclude='*.a' \
    --exclude='*.so' \
    --exclude='*.so.*' \
    --exclude='obj'
install -m 644 "$SAFE_ROOT/include/zstd.h" "$STAGE_ROOT/lib/zstd.h"
install -m 644 "$SAFE_ROOT/include/zdict.h" "$STAGE_ROOT/lib/zdict.h"
install -m 644 "$SAFE_ROOT/include/zstd_errors.h" "$STAGE_ROOT/lib/zstd_errors.h"

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
    PATH="$DPKG_WRAPPER_DIR:$PATH" dpkg-buildpackage -d -b -us -uc
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

if [[ $BUILD_TAG == default ]]; then
    rm -f \
        "$SAFE_ROOT/out"/libzstd-dev_*.deb \
        "$SAFE_ROOT/out"/libzstd1_*.deb \
        "$SAFE_ROOT/out"/libzstd1-udeb_*.udeb \
        "$SAFE_ROOT/out"/zstd_*.deb

    link_latest_package "$PACKAGE_DIR/libzstd-dev_*.deb" "$SAFE_ROOT/out"
    link_latest_package "$PACKAGE_DIR/libzstd1_*.deb" "$SAFE_ROOT/out"
    link_latest_package "$PACKAGE_DIR/zstd_*.deb" "$SAFE_ROOT/out"
    if [[ $SAFE_ENABLE_UDEB -eq 1 ]]; then
        link_latest_package "$PACKAGE_DIR/libzstd1-udeb_*.udeb" "$SAFE_ROOT/out"
    fi
fi

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
BUILD_SIGNATURE='$BUILD_SIGNATURE'
CANONICAL_INSTALL_ROOT='$CANONICAL_INSTALL_ROOT'
CANONICAL_HELPER_ROOT='$CANONICAL_HELPER_ROOT'
EOF

printf 'staged source tree: %s\n' "$STAGE_ROOT"
printf 'package outputs: %s\n' "$PACKAGE_DIR"
printf 'stage install root: %s\n' "$INSTALL_ROOT"
