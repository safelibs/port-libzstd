#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
source "$SAFE_ROOT/scripts/phase6-common.sh"

phase6_require_phase4_inputs "$0"

bash "$SCRIPT_DIR/build-artifacts.sh" --release
bash "$SCRIPT_DIR/build-artifacts.sh" --release --variant mt
bash "$SCRIPT_DIR/build-artifacts.sh" --release --variant nomt

DEFAULT_ARTIFACT_STAMP="$SAFE_ROOT/out/obj/release-default/.build-artifacts.signature"
MT_ARTIFACT_STAMP="$SAFE_ROOT/out/obj/release-mt/.build-artifacts.signature"
NOMT_ARTIFACT_STAMP="$SAFE_ROOT/out/obj/release-nomt/.build-artifacts.signature"
STAMP_FILE=$(phase6_stamp_path run-build-variant-tests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$DEFAULT_ARTIFACT_STAMP" \
    "$MT_ARTIFACT_STAMP" \
    "$NOMT_ARTIFACT_STAMP" \
    && phase6_tracked_repo_paths_are_fresh \
        "$STAMP_FILE" \
        "$SAFE_ROOT/Cargo.toml" \
        "$SAFE_ROOT/include" \
        "$SAFE_ROOT/src" \
        "$SAFE_ROOT/tests"
then
    phase6_log "build variant verification already fresh; skipping rerun"
    exit 0
fi

DEFAULT_LIBDIR="$SAFE_ROOT/out/install/release-default/usr/lib/$MULTIARCH"
MT_LIBDIR="$SAFE_ROOT/out/install/release-mt/usr/lib/$MULTIARCH"
NOMT_LIBDIR="$SAFE_ROOT/out/install/release-nomt/usr/lib/$MULTIARCH"

assert_archive_ready() {
    local archive=$1
    local lookup="dl""sym"
    local loader="dl""open"

    ar t "$archive" >/dev/null || {
        printf 'build variant static library is not a readable archive: %s\n' "$archive" >&2
        exit 1
    }
    if LC_ALL=C strings -a "$archive" | LC_ALL=C grep -aE "${lookup}|${loader}" >/dev/null; then
        printf 'build variant static library carries loader lookup token: %s\n' "$archive" >&2
        exit 1
    fi
}

static_link_smoke() {
    local archive=$1
    local include_root=$2
    local work_root=$3
    local cc_bin=${CC:-cc}

    rm -rf "$work_root"
    install -d "$work_root"
    cat >"$work_root/static-link-smoke.c" <<'EOF'
#include <zstd.h>

int main(void) {
    return ZSTD_versionNumber() == 10505 ? 0 : 1;
}
EOF
    "$cc_bin" -static -I "$include_root" \
        "$work_root/static-link-smoke.c" "$archive" -lm \
        -o "$work_root/static-link-smoke"
    "$work_root/static-link-smoke"
}

for candidate in "$DEFAULT_LIBDIR" "$MT_LIBDIR" "$NOMT_LIBDIR"; do
    if [[ ! -d $candidate ]]; then
        candidate=${candidate%/$MULTIARCH}
    fi
    phase6_require_path "$candidate/libzstd.so.1.5.5" "prepared build variant shared object"
    phase6_require_path "$candidate/libzstd.a" "prepared build variant static archive"
    phase6_require_path "$candidate/pkgconfig/libzstd.pc" "prepared build variant pkg-config metadata"
    assert_archive_ready "$candidate/libzstd.a"
    readelf -lW "$candidate/libzstd.so.1.5.5" | grep -q 'GNU_STACK' || {
        printf 'missing GNU_STACK program header: %s\n' "$candidate/libzstd.so.1.5.5" >&2
        exit 1
    }
    if readelf -lW "$candidate/libzstd.so.1.5.5" | grep 'GNU_STACK' | grep -q 'RWE'; then
        printf 'shared object requests an executable stack: %s\n' "$candidate/libzstd.so.1.5.5" >&2
        exit 1
    fi
done

grep -q 'Libs.private: -pthread -lm' "$MT_LIBDIR/pkgconfig/libzstd.pc" || {
    printf 'mt pkg-config metadata lost private linkage flags\n' >&2
    exit 1
}
grep -q 'Libs.private: -lm' "$DEFAULT_LIBDIR/pkgconfig/libzstd.pc" || {
    printf 'default pkg-config metadata lost libm linkage\n' >&2
    exit 1
}
grep -q 'Libs.private: -lm' "$NOMT_LIBDIR/pkgconfig/libzstd.pc" || {
    printf 'nomt pkg-config metadata lost libm linkage\n' >&2
    exit 1
}

if grep -q 'Libs.private: -pthread' "$DEFAULT_LIBDIR/pkgconfig/libzstd.pc"; then
    printf 'default pkg-config metadata still advertises pthread linkage\n' >&2
    exit 1
fi
if grep -q 'Libs.private: -pthread' "$NOMT_LIBDIR/pkgconfig/libzstd.pc"; then
    printf 'nomt pkg-config metadata still advertises pthread linkage\n' >&2
    exit 1
fi

static_link_smoke \
    "$DEFAULT_LIBDIR/libzstd.a" \
    "$SAFE_ROOT/out/install/release-default/usr/include" \
    "$SAFE_ROOT/out/obj/run-build-variant-static-smoke"

if cmp -s "$MT_LIBDIR/libzstd.a" "$NOMT_LIBDIR/libzstd.a"; then
    printf 'mt and nomt static archives are unexpectedly identical\n' >&2
    exit 1
fi
if cmp -s "$MT_LIBDIR/libzstd.so.1.5.5" "$NOMT_LIBDIR/libzstd.so.1.5.5"; then
    printf 'mt and nomt shared objects are unexpectedly identical\n' >&2
    exit 1
fi

phase6_touch_stamp "$STAMP_FILE"
