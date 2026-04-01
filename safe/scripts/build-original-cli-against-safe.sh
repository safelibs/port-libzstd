#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)

SOURCE_ROOT="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"
ARTIFACT_ROOT="$SAFE_ROOT/out/install/release-default"
WORK_ROOT="$SAFE_ROOT/out/original-cli"
DESTDIR=
PREFIX=/usr
LIBDIR=
INCLUDEDIR=/usr/include
MULTIARCH=
VERSION=1.5.5
SONAME=1

usage() {
    cat <<'EOF'
usage: build-original-cli-against-safe.sh [--source-root PATH]
                                          [--artifact-root PATH]
                                          [--work-root PATH]
                                          [--destdir PATH]
                                          [--prefix PATH]
                                          [--libdir PATH]
                                          [--includedir PATH]
                                          [--multiarch TRIPLET]
EOF
}

relpath() {
    python3 - "$1" "$2" <<'PY'
import os
import sys
print(os.path.relpath(sys.argv[1], sys.argv[2]))
PY
}

compute_build_signature() {
    python3 - \
        "$REPO_ROOT" \
        "$SOURCE_ROOT" \
        "$ARTIFACT_ROOT" \
        "$SCRIPT_DIR/build-original-cli-against-safe.sh" \
        "$VERSION" \
        "$SONAME" \
        "$PREFIX" \
        "$LIBDIR" \
        "$INCLUDEDIR" \
        "$DESTDIR" \
        <<'PY'
from __future__ import annotations

import hashlib
import pathlib
import subprocess
import sys

repo_root = pathlib.Path(sys.argv[1])
source_root = pathlib.Path(sys.argv[2])
artifact_root = pathlib.Path(sys.argv[3])
script_path = pathlib.Path(sys.argv[4])
params = sys.argv[5:]

libdir = pathlib.Path(params[3].lstrip("/"))
includedir = pathlib.Path(params[4].lstrip("/"))

def iter_signature_entries(root: pathlib.Path, path: pathlib.Path) -> list[pathlib.Path]:
    if not path.exists():
        raise SystemExit(f"missing input for helper build signature: {path}")
    if not path.is_dir():
        return [path]
    if root != source_root:
        return sorted(entry for entry in path.rglob("*") if entry.is_file())

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

paths: list[tuple[pathlib.Path, pathlib.Path]] = [
    (script_path.parent, script_path),
    (source_root, source_root / "lib" / "common"),
    (source_root, source_root / "lib" / "legacy"),
    (source_root, source_root / "lib" / "libzstd.mk"),
    (source_root, source_root / "programs"),
    (source_root, source_root / "contrib" / "pzstd"),
]

for rel in (
    includedir / "zstd.h",
    includedir / "zdict.h",
    includedir / "zstd_errors.h",
    libdir / "libzstd.so.1.5.5",
    libdir / "libzstd.so.1",
    libdir / "libzstd.so",
    libdir / "libzstd.a",
):
    paths.append((artifact_root, artifact_root / rel))

for root, path in paths:
    entries = iter_signature_entries(root, path)
    for entry in entries:
        rel = entry.relative_to(root)
        h.update(str(rel).encode("utf-8"))
        h.update(b"\0")
        if entry.is_symlink():
            h.update(b"symlink\0")
            h.update(pathlib.Path(entry.readlink()).as_posix().encode("utf-8"))
        else:
            h.update(entry.read_bytes())
        h.update(b"\0")

print(h.hexdigest())
PY
}

artifact_tree_ready() {
    local include_root=$1
    local lib_root=$2
    local path

    for path in \
        "$include_root/zstd.h" \
        "$include_root/zdict.h" \
        "$include_root/zstd_errors.h" \
        "$lib_root/libzstd.so.$VERSION" \
        "$lib_root/libzstd.so.$SONAME" \
        "$lib_root/libzstd.so" \
        "$lib_root/libzstd.a"
    do
        [[ -e $path ]] || return 1
    done
}

helper_outputs_are_current() {
    local signature=$1
    local helper_root=$2
    local stamp_file=$3
    local bin_root=$4
    local man_root=$5
    local path

    for path in \
        "$helper_root/libzstd.mk" \
        "$helper_root/common/xxhash.c" \
        "$helper_root/common/threading.h" \
        "$helper_root/legacy/zstd_legacy.h" \
        "$helper_root/zstd.h" \
        "$helper_root/zdict.h" \
        "$helper_root/zstd_errors.h" \
        "$helper_root/libzstd.so.$VERSION" \
        "$helper_root/libzstd.a" \
        "$bin_root/zstd" \
        "$bin_root/zstdcat" \
        "$bin_root/unzstd" \
        "$bin_root/zstdmt" \
        "$bin_root/zstdgrep" \
        "$bin_root/zstdless" \
        "$bin_root/pzstd" \
        "$man_root/zstd.1" \
        "$man_root/zstdcat.1" \
        "$man_root/unzstd.1" \
        "$man_root/zstdgrep.1" \
        "$man_root/zstdless.1"
    do
        [[ -e $path ]] || return 1
    done

    [[ -L $helper_root/libzstd.so.$SONAME ]] || return 1
    [[ -L $helper_root/libzstd.so ]] || return 1
    [[ $(readlink "$helper_root/libzstd.so.$SONAME") == "libzstd.so.$VERSION" ]] || return 1
    [[ $(readlink "$helper_root/libzstd.so") == "libzstd.so.$VERSION" ]] || return 1
    grep -Eq '^INPUT[[:space:]]*\([[:space:]]*libzstd\.so[[:space:]]*\)$' "$helper_root/libzstd.a" || return 1

    [[ -f $stamp_file ]] || return 1
    [[ $(<"$stamp_file") == "$signature" ]]
}

assert_binary_uses_helper_lib() {
    local helper_root=$1
    local binary=$2
    local resolved=

    resolved=$(
        env LD_LIBRARY_PATH="$helper_root" ldd "$binary" 2>/dev/null |
            awk '/libzstd\.so/ {print $3; exit}'
    )

    [[ -n $resolved ]] || {
        printf 'could not resolve libzstd for %s via helper root %s\n' "$binary" "$helper_root" >&2
        exit 1
    }
    [[ $resolved == "$helper_root/libzstd.so.$SONAME" || $resolved == "$helper_root/libzstd.so.$VERSION" ]] || {
        printf 'binary %s did not resolve libzstd from helper root: %s\n' "$binary" "$resolved" >&2
        exit 1
    }
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --source-root)
            SOURCE_ROOT=${2:?missing source root}
            shift
            ;;
        --artifact-root)
            ARTIFACT_ROOT=${2:?missing artifact root}
            shift
            ;;
        --work-root)
            WORK_ROOT=${2:?missing work root}
            shift
            ;;
        --destdir)
            DESTDIR=${2:?missing destdir}
            shift
            ;;
        --prefix)
            PREFIX=${2:?missing prefix}
            shift
            ;;
        --libdir)
            LIBDIR=${2:?missing libdir}
            shift
            ;;
        --includedir)
            INCLUDEDIR=${2:?missing includedir}
            shift
            ;;
        --multiarch)
            MULTIARCH=${2:?missing multiarch}
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

if [[ -z $MULTIARCH ]] && command -v dpkg-architecture >/dev/null 2>&1; then
    MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
fi

if [[ -z $LIBDIR ]]; then
    if [[ -n $MULTIARCH ]]; then
        LIBDIR="$PREFIX/lib/$MULTIARCH"
    else
        LIBDIR="$PREFIX/lib"
    fi
fi

if [[ -z $DESTDIR ]]; then
    DESTDIR="$ARTIFACT_ROOT"
fi

HELPER_LIB_ROOT="$WORK_ROOT/lib"
ARTIFACT_LIB_ROOT="$ARTIFACT_ROOT$LIBDIR"
ARTIFACT_INCLUDE_ROOT="$ARTIFACT_ROOT$INCLUDEDIR"
STAMP_FILE="$WORK_ROOT/.build-original-cli.signature"
BIN_ROOT="$DESTDIR$PREFIX/bin"
MAN_ROOT="$DESTDIR$PREFIX/share/man/man1"
DEFAULT_ARTIFACT_ROOT="$SAFE_ROOT/out/install/release-default"

if ! artifact_tree_ready "$ARTIFACT_INCLUDE_ROOT" "$ARTIFACT_LIB_ROOT"; then
    if [[ $ARTIFACT_ROOT == "$DEFAULT_ARTIFACT_ROOT" ]]; then
        bash "$SCRIPT_DIR/build-artifacts.sh" --release
    fi
fi

artifact_tree_ready "$ARTIFACT_INCLUDE_ROOT" "$ARTIFACT_LIB_ROOT" || {
    printf 'safe artifact root is missing headers or libraries: %s\n' "$ARTIFACT_ROOT" >&2
    exit 1
}

BUILD_SIGNATURE=$(compute_build_signature)
if helper_outputs_are_current "$BUILD_SIGNATURE" "$HELPER_LIB_ROOT" "$STAMP_FILE" "$BIN_ROOT" "$MAN_ROOT"; then
    printf 'reusing up-to-date original CLI helper tree: %s\n' "$HELPER_LIB_ROOT"
    exit 0
fi

rm -rf "$HELPER_LIB_ROOT"
install -d "$HELPER_LIB_ROOT/common" \
    "$HELPER_LIB_ROOT/compress" \
    "$HELPER_LIB_ROOT/decompress" \
    "$HELPER_LIB_ROOT/dictBuilder" \
    "$HELPER_LIB_ROOT/deprecated" \
    "$HELPER_LIB_ROOT/legacy"

rsync -a --delete "$SOURCE_ROOT/lib/common/" "$HELPER_LIB_ROOT/common/"
if [[ -d $SOURCE_ROOT/lib/legacy ]]; then
    rsync -a --delete "$SOURCE_ROOT/lib/legacy/" "$HELPER_LIB_ROOT/legacy/"
fi
install -m 644 "$SOURCE_ROOT/lib/libzstd.mk" "$HELPER_LIB_ROOT/libzstd.mk"
cat >"$HELPER_LIB_ROOT/Makefile" <<'EOF'
.PHONY: clean libzstd.a libzstd

clean:

libzstd.a:

libzstd:
EOF
install -m 644 "$ARTIFACT_INCLUDE_ROOT/zstd.h" "$HELPER_LIB_ROOT/zstd.h"
install -m 644 "$ARTIFACT_INCLUDE_ROOT/zdict.h" "$HELPER_LIB_ROOT/zdict.h"
install -m 644 "$ARTIFACT_INCLUDE_ROOT/zstd_errors.h" "$HELPER_LIB_ROOT/zstd_errors.h"
install -m 755 "$ARTIFACT_LIB_ROOT/libzstd.so.$VERSION" "$HELPER_LIB_ROOT/libzstd.so.$VERSION"
ln -sfn "libzstd.so.$VERSION" "$HELPER_LIB_ROOT/libzstd.so.$SONAME"
ln -sfn "libzstd.so.$VERSION" "$HELPER_LIB_ROOT/libzstd.so"
cat >"$HELPER_LIB_ROOT/libzstd.a" <<'EOF'
INPUT ( libzstd.so )
EOF

PROGRAMS_DIR="$SOURCE_ROOT/programs"
PZSTD_DIR="$SOURCE_ROOT/contrib/pzstd"
HELPER_FROM_PROGRAMS=$(relpath "$HELPER_LIB_ROOT" "$PROGRAMS_DIR")
HELPER_FROM_PZSTD=$(relpath "$HELPER_LIB_ROOT" "$PZSTD_DIR")
PROGRAMS_FROM_PZSTD=$(relpath "$PROGRAMS_DIR" "$PZSTD_DIR")

make -C "$PROGRAMS_DIR" clean LIBZSTD="$HELPER_FROM_PROGRAMS" >/dev/null || true
make -C "$PZSTD_DIR" clean ZSTDDIR="$HELPER_FROM_PZSTD" PROGDIR="$PROGRAMS_FROM_PZSTD" >/dev/null || true

make -C "$PROGRAMS_DIR" zstd-dll LIBZSTD="$HELPER_FROM_PROGRAMS"
make -C "$PROGRAMS_DIR" install \
    DESTDIR="$DESTDIR" \
    PREFIX="$PREFIX" \
    LIBZSTD="$HELPER_FROM_PROGRAMS"

make -C "$PZSTD_DIR" install \
    DESTDIR="$DESTDIR" \
    PREFIX="$PREFIX" \
    ZSTDDIR="$HELPER_FROM_PZSTD" \
    PROGDIR="$PROGRAMS_FROM_PZSTD"

assert_binary_uses_helper_lib "$HELPER_LIB_ROOT" "$BIN_ROOT/zstd"
assert_binary_uses_helper_lib "$HELPER_LIB_ROOT" "$BIN_ROOT/pzstd"

printf '%s\n' "$BUILD_SIGNATURE" >"$STAMP_FILE"
