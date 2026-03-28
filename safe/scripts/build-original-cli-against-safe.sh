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
INCLUDE_ROOT="$SAFE_ROOT/include"
MULTIARCH=

usage() {
    cat <<'EOF'
usage: build-original-cli-against-safe.sh [--source-root PATH]
                                          [--artifact-root PATH]
                                          [--work-root PATH]
                                          [--destdir PATH]
                                          [--prefix PATH]
                                          [--libdir PATH]
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
install -m 644 "$INCLUDE_ROOT/zstd.h" "$HELPER_LIB_ROOT/zstd.h"
install -m 644 "$INCLUDE_ROOT/zdict.h" "$HELPER_LIB_ROOT/zdict.h"
install -m 644 "$INCLUDE_ROOT/zstd_errors.h" "$HELPER_LIB_ROOT/zstd_errors.h"
install -m 755 "$ARTIFACT_ROOT$LIBDIR/libzstd.so.1.5.5" "$HELPER_LIB_ROOT/libzstd.so.1.5.5"
ln -sfn "libzstd.so.1.5.5" "$HELPER_LIB_ROOT/libzstd.so.1"
ln -sfn "libzstd.so.1.5.5" "$HELPER_LIB_ROOT/libzstd.so"
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
