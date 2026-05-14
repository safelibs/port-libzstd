#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

VERSION=1.5.5
SONAME=1
PROFILE=debug
VARIANT=default
PREFIX=/usr
LIBDIR=
INCLUDEDIR=/usr/include
MULTIARCH=
DESTDIR=
OBJDIR=
INSTALL_CMAKE=1

compute_build_signature() {
    python3 - "$SAFE_ROOT" "$VERSION" "$SONAME" "$PROFILE" "$VARIANT" "$PREFIX" "$LIBDIR" "$INCLUDEDIR" "$MULTIARCH" "$INSTALL_CMAKE" <<'PY'
from __future__ import annotations

import hashlib
import pathlib
import subprocess
import sys

safe_root = pathlib.Path(sys.argv[1])
params = sys.argv[2:]

h = hashlib.sha256()
for value in params:
    h.update(value.encode("utf-8"))
    h.update(b"\0")

for command in (["rustc", "-vV"], ["cargo", "-V"]):
    try:
        h.update(subprocess.check_output(command))
    except Exception as exc:
        h.update(f"{command!r}: {exc}".encode("utf-8"))
    h.update(b"\0")

paths = [
    safe_root / "Cargo.toml",
    safe_root / "build.rs",
    safe_root / "src",
    safe_root / "third_party",
    safe_root / "include",
    safe_root / "pkgconfig",
    safe_root / "cmake",
    safe_root / "scripts" / "build-artifacts.sh",
]

for optional in ("Cargo.lock", "rust-toolchain.toml"):
    path = safe_root / optional
    if path.exists():
        paths.append(path)

for path in paths:
    if path.is_dir():
        files = sorted(entry for entry in path.rglob("*") if entry.is_file())
    else:
        files = [path]
    for entry in files:
        rel = entry.relative_to(safe_root)
        h.update(str(rel).encode("utf-8"))
        h.update(b"\0")
        h.update(entry.read_bytes())
        h.update(b"\0")

print(h.hexdigest())
PY
}

artifacts_are_current() {
    local signature=$1
    local stamp_file=$2
    local -a required=(
        "$DESTDIR$LIBDIR/libzstd.so.$VERSION"
        "$DESTDIR$LIBDIR/libzstd.so.$SONAME"
        "$DESTDIR$LIBDIR/libzstd.so"
        "$DESTDIR$LIBDIR/libzstd.a"
        "$DESTDIR$LIBDIR/pkgconfig/libzstd.pc"
        "$DESTDIR$INCLUDEDIR/zstd.h"
        "$DESTDIR$INCLUDEDIR/zdict.h"
        "$DESTDIR$INCLUDEDIR/zstd_errors.h"
    )

    local path
    for path in "${required[@]}"; do
        [[ -e $path ]] || return 1
    done

    [[ $(readlink "$DESTDIR$LIBDIR/libzstd.so.$SONAME") == "libzstd.so.$VERSION" ]] || return 1
    [[ $(readlink "$DESTDIR$LIBDIR/libzstd.so") == "libzstd.so.$VERSION" ]] || return 1

    if [[ $INSTALL_CMAKE -eq 1 ]]; then
        required+=(
            "$DESTDIR$LIBDIR/cmake/zstd/zstdConfig.cmake"
            "$DESTDIR$LIBDIR/cmake/zstd/zstdConfigVersion.cmake"
            "$DESTDIR$LIBDIR/cmake/zstd/zstdTargets.cmake"
            "$DESTDIR$LIBDIR/cmake/zstd/zstdTargets-noconfig.cmake"
        )
        for path in "${required[@]:8}"; do
            [[ -e $path ]] || return 1
        done
    fi

    [[ -f $stamp_file ]] || return 1
    [[ $(<"$stamp_file") == "$signature" ]]
}

usage() {
    cat <<'EOF'
usage: build-artifacts.sh [--release|--debug] [--variant default|mt|nomt]
                          [--destdir PATH] [--objdir PATH]
                          [--prefix PATH] [--libdir PATH]
                          [--includedir PATH] [--multiarch TRIPLET]
                          [--no-install-cmake]
EOF
}

pc_path_expr() {
    local base_var=$1
    local base=$2
    local path=$3

    if [[ $path == "$base" ]]; then
        printf '${%s}' "$base_var"
    elif [[ $path == "$base"/* ]]; then
        printf '${%s}%s' "$base_var" "${path#$base}"
    else
        printf '%s' "$path"
    fi
}

cmake_path_expr() {
    local base=$1
    local path=$2

    if [[ $path == "$base" ]]; then
        printf '${_IMPORT_PREFIX}'
    elif [[ $path == "$base"/* ]]; then
        printf '${_IMPORT_PREFIX}%s' "${path#$base}"
    else
        printf '%s' "$path"
    fi
}

host_triple() {
    rustc -vV | awk '/^host:/ { host = $2 } END { print host }'
}

prepare_static_rust_sysroot() {
    local original_sysroot
    local cache_base
    local cache_key
    local cache_root
    local patched_sysroot
    local stamp_file

    original_sysroot=$(rustc --print sysroot)
    cache_base="${XDG_CACHE_HOME:-$HOME/.cache}/safelibs/libzstd-static-rust-sysroot"
    cache_key=$(python3 - "$original_sysroot" <<'PY'
from __future__ import annotations

import hashlib
import pathlib
import subprocess
import sys

root = pathlib.Path(sys.argv[1])
paths = [
    root / "lib/rustlib/src/rust/library/std/src/sys/thread/unix.rs",
    root / "lib/rustlib/src/rust/library/std/src/sys/pal/unix/weak/mod.rs",
]

h = hashlib.sha256()
h.update(b"libzstd-static-rust-sysroot-v2\0")
for command in (["rustc", "-vV"], ["cargo", "-V"]):
    h.update(subprocess.check_output(command))
    h.update(b"\0")
for path in paths:
    h.update(str(path.relative_to(root)).encode("utf-8"))
    h.update(b"\0")
    h.update(path.read_bytes())
    h.update(b"\0")
print(h.hexdigest()[:24])
PY
)
    cache_root="$cache_base/$cache_key"
    patched_sysroot="$cache_root/sysroot"
    stamp_file="$cache_root/.prepared"

    if [[ -x $patched_sysroot/bin/rustc && -x $patched_sysroot/bin/cargo && -f $stamp_file ]]; then
        printf '%s\n' "$patched_sysroot"
        return 0
    fi

    rm -rf "$cache_root"
    install -d "$cache_root"
    if ! cp -a -l "$original_sysroot" "$patched_sysroot" 2>/dev/null; then
        cp -a "$original_sysroot" "$patched_sysroot"
    fi

    python3 - "$patched_sysroot" <<'PY'
from __future__ import annotations

import pathlib
import sys

root = pathlib.Path(sys.argv[1]) / "lib/rustlib/src/rust/library/std/src/sys"
lookup = "dl" + "sym"

thread = root / "thread/unix.rs"
thread_source = thread.read_text()
thread_import = (
    '#[cfg(all(target_os = "linux", target_env = "gnu"))]\n'
    "use crate::sys::weak::" + lookup + ";\n"
)
if thread_import not in thread_source:
    raise SystemExit("static std patch could not find the thread import marker")
thread_source = thread_source.replace(thread_import, "")

start_marker = (
    '#[cfg(all(target_os = "linux", target_env = "gnu"))]\n'
    "unsafe fn min_stack_size(attr: *const libc::pthread_attr_t) -> usize {"
)
next_marker = "\n// No point in looking up __pthread_get_minstack() on non-glibc platforms."
start = thread_source.find(start_marker)
if start < 0:
    raise SystemExit("static std patch could not find the stack-size helper")
end = thread_source.find(next_marker, start)
if end < 0:
    raise SystemExit("static std patch could not find the stack-size helper end")
replacement = (
    '#[cfg(all(target_os = "linux", target_env = "gnu"))]\n'
    "unsafe fn min_stack_size(_: *const libc::pthread_attr_t) -> usize {\n"
    "    libc::PTHREAD_STACK_MIN\n"
    "}\n"
)
thread.unlink()
thread.write_text(thread_source[:start] + replacement + thread_source[end + 1 :])

weak = root / "pal/unix/weak/mod.rs"
weak_source = weak.read_text()
module_block = (
    "// GNU/Linux needs the `" + lookup + "` variant to avoid linking to private glibc symbols.\n"
    '#[cfg(all(target_os = "linux", target_env = "gnu"))]\n'
    "mod " + lookup + ";\n"
    '#[cfg(all(target_os = "linux", target_env = "gnu"))]\n'
    "pub(crate) use " + lookup + "::weak as " + lookup + ";\n\n"
)
if module_block not in weak_source:
    raise SystemExit("static std patch could not find the weak module marker")
weak.unlink()
weak.write_text(weak_source.replace(module_block, ""))
PY
    printf '%s\n' "$cache_key" >"$stamp_file"
    printf '%s\n' "$patched_sysroot"
}

assert_static_archive_clean() {
    local archive=$1
    local lookup="dl""sym"
    local loader="dl""open"

    ar t "$archive" >/dev/null || {
        printf 'static archive is not a readable archive: %s\n' "$archive" >&2
        exit 1
    }
    if LC_ALL=C strings -a "$archive" | LC_ALL=C grep -aE "${lookup}|${loader}" >/dev/null; then
        printf 'static archive still carries loader lookup token: %s\n' "$archive" >&2
        exit 1
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            PROFILE=release
            ;;
        --debug)
            PROFILE=debug
            ;;
        --variant)
            VARIANT=${2:?missing variant}
            shift
            ;;
        --destdir)
            DESTDIR=${2:?missing destdir}
            shift
            ;;
        --objdir)
            OBJDIR=${2:?missing objdir}
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
        --no-install-cmake)
            INSTALL_CMAKE=0
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
    DESTDIR="$SAFE_ROOT/out/install/${PROFILE}-${VARIANT}"
fi

if [[ -z $OBJDIR ]]; then
    OBJDIR="$SAFE_ROOT/out/obj/${PROFILE}-${VARIANT}"
fi

PROFILE_FLAG=
if [[ $PROFILE == release ]]; then
    PROFILE_FLAG=--release
fi

case "$VARIANT" in
    default)
        SHARED_FEATURES=build-shared-default
        STATIC_FEATURES=build-static-default
        LIBS_PRIVATE=-lm
        ;;
    mt)
        SHARED_FEATURES=variant-mt
        STATIC_FEATURES=variant-mt
        LIBS_PRIVATE="-pthread -lm"
        ;;
    nomt)
        SHARED_FEATURES=variant-nomt
        STATIC_FEATURES=variant-nomt
        LIBS_PRIVATE=-lm
        ;;
    *)
        printf 'unsupported variant: %s\n' "$VARIANT" >&2
        exit 2
        ;;
esac

BUILD_ROOT="$SAFE_ROOT/out/cargo/${PROFILE}-${VARIANT}"
SHARED_TARGET_DIR="$BUILD_ROOT/shared"
STATIC_TARGET_DIR="$BUILD_ROOT/static"
STATIC_TARGET=$(host_triple)
STAMP_FILE="$OBJDIR/.build-artifacts.signature"
BUILD_SIGNATURE=$(compute_build_signature)

if artifacts_are_current "$BUILD_SIGNATURE" "$STAMP_FILE"; then
    printf 'reusing up-to-date build artifacts: %s\n' "$DESTDIR"
    exit 0
fi

rm -rf "$DESTDIR" "$OBJDIR"
rm -rf "$BUILD_ROOT"
install -d "$DESTDIR$LIBDIR" "$DESTDIR$INCLUDEDIR" \
    "$DESTDIR$LIBDIR/pkgconfig" "$OBJDIR/CMakeFiles/Export/libzstd-safe"
if [[ $INSTALL_CMAKE -eq 1 ]]; then
    install -d "$DESTDIR$LIBDIR/cmake/zstd"
fi

CARGO_BASE=(cargo rustc --manifest-path "$SAFE_ROOT/Cargo.toml" --no-default-features)
if [[ -n $PROFILE_FLAG ]]; then
    CARGO_BASE+=("$PROFILE_FLAG")
fi

CARGO_TARGET_DIR="$SHARED_TARGET_DIR" \
    "${CARGO_BASE[@]}" --features "$SHARED_FEATURES" -- --crate-type=cdylib

STATIC_SYSROOT=$(prepare_static_rust_sysroot)
STATIC_CARGO=(
    "$STATIC_SYSROOT/bin/cargo" -Z build-std=std
    rustc --manifest-path "$SAFE_ROOT/Cargo.toml" --no-default-features
)
if [[ -n $PROFILE_FLAG ]]; then
    STATIC_CARGO+=("$PROFILE_FLAG")
fi
STATIC_CARGO+=(
    --target "$STATIC_TARGET" --features "$STATIC_FEATURES" -- --crate-type=staticlib
)
RUSTC_BOOTSTRAP=1 \
RUSTC="$STATIC_SYSROOT/bin/rustc" \
CARGO_TARGET_DIR="$STATIC_TARGET_DIR" \
    "${STATIC_CARGO[@]}"

SHARED_OUT_DIR="$SHARED_TARGET_DIR/$PROFILE"
SHARED_SRC="$SHARED_OUT_DIR/libzstd.so"
SHARED_BASENAME="libzstd.so.$VERSION"
STATIC_OUT_DIR="$STATIC_TARGET_DIR/$STATIC_TARGET/$PROFILE"
STATIC_SRC="$STATIC_OUT_DIR/libzstd.a"
if [[ ! -f $STATIC_SRC ]]; then
    STATIC_SRC=$(find "$STATIC_OUT_DIR/deps" -maxdepth 1 -name libzstd.a -print -quit)
fi
[[ -n ${STATIC_SRC:-} && -f $STATIC_SRC ]] || {
    printf 'cargo did not produce libzstd.a under %s\n' "$STATIC_OUT_DIR" >&2
    exit 1
}

install -m 755 "$SHARED_SRC" "$DESTDIR$LIBDIR/$SHARED_BASENAME"
ln -sfn "$SHARED_BASENAME" "$DESTDIR$LIBDIR/libzstd.so.$SONAME"
ln -sfn "$SHARED_BASENAME" "$DESTDIR$LIBDIR/libzstd.so"
install -m 644 "$STATIC_SRC" "$OBJDIR/libzstd.a"
assert_static_archive_clean "$OBJDIR/libzstd.a"
install -m 644 "$OBJDIR/libzstd.a" "$DESTDIR$LIBDIR/libzstd.a"

install -m 644 "$SAFE_ROOT/include/zstd.h" "$DESTDIR$INCLUDEDIR/zstd.h"
install -m 644 "$SAFE_ROOT/include/zdict.h" "$DESTDIR$INCLUDEDIR/zdict.h"
install -m 644 "$SAFE_ROOT/include/zstd_errors.h" "$DESTDIR$INCLUDEDIR/zstd_errors.h"

PC_EXEC_PREFIX='${prefix}'
PC_LIBDIR=$(pc_path_expr exec_prefix "$PREFIX" "$LIBDIR")
PC_INCLUDEDIR=$(pc_path_expr prefix "$PREFIX" "$INCLUDEDIR")

sed \
    -e "s|@PREFIX@|$PREFIX|g" \
    -e "s|@EXEC_PREFIX@|$PC_EXEC_PREFIX|g" \
    -e "s|@INCLUDEDIR@|$PC_INCLUDEDIR|g" \
    -e "s|@LIBDIR@|$PC_LIBDIR|g" \
    -e "s|@VERSION@|$VERSION|g" \
    -e "s|@LIBS_PRIVATE@|$LIBS_PRIVATE|g" \
    "$SAFE_ROOT/pkgconfig/libzstd.pc.in" >"$OBJDIR/libzstd.pc"
install -m 644 "$OBJDIR/libzstd.pc" "$DESTDIR$LIBDIR/pkgconfig/libzstd.pc"

cp "$SAFE_ROOT/cmake/zstdConfig.cmake.in" "$OBJDIR/zstdConfig.cmake"
if [[ $INSTALL_CMAKE -eq 1 ]]; then
    install -m 644 "$OBJDIR/zstdConfig.cmake" "$DESTDIR$LIBDIR/cmake/zstd/zstdConfig.cmake"
fi

cat >"$OBJDIR/zstdConfigVersion.cmake" <<EOF
set(PACKAGE_VERSION "$VERSION")

if(PACKAGE_FIND_VERSION VERSION_GREATER PACKAGE_VERSION)
  set(PACKAGE_VERSION_COMPATIBLE FALSE)
else()
  string(REGEX MATCH "^[0-9]+" PACKAGE_VERSION_MAJOR "\${PACKAGE_VERSION}")
  string(REGEX MATCH "^[0-9]+" PACKAGE_FIND_VERSION_MAJOR "\${PACKAGE_FIND_VERSION}")
  if(PACKAGE_FIND_VERSION_MAJOR STREQUAL PACKAGE_VERSION_MAJOR)
    set(PACKAGE_VERSION_COMPATIBLE TRUE)
    if(PACKAGE_FIND_VERSION VERSION_EQUAL PACKAGE_VERSION)
      set(PACKAGE_VERSION_EXACT TRUE)
    endif()
  else()
    set(PACKAGE_VERSION_COMPATIBLE FALSE)
  endif()
endif()
EOF
if [[ $INSTALL_CMAKE -eq 1 ]]; then
    install -m 644 "$OBJDIR/zstdConfigVersion.cmake" \
        "$DESTDIR$LIBDIR/cmake/zstd/zstdConfigVersion.cmake"
fi

CMAKE_LIBDIR_EXPR=$(cmake_path_expr "$PREFIX" "$LIBDIR")
CMAKE_INCLUDEDIR_EXPR=$(cmake_path_expr "$PREFIX" "$INCLUDEDIR")
TARGETS_NOCONFIG="$OBJDIR/CMakeFiles/Export/libzstd-safe/zstdTargets-noconfig.cmake"

cat >"$TARGETS_NOCONFIG" <<EOF
get_filename_component(_IMPORT_PREFIX "\${CMAKE_CURRENT_LIST_DIR}/../../../.." ABSOLUTE)

if(NOT TARGET zstd::libzstd_shared)
  add_library(zstd::libzstd_shared SHARED IMPORTED)
  set_target_properties(zstd::libzstd_shared PROPERTIES
    IMPORTED_LOCATION_NOCONFIG "$CMAKE_LIBDIR_EXPR/$SHARED_BASENAME"
    IMPORTED_SONAME_NOCONFIG "libzstd.so.$SONAME"
    INTERFACE_INCLUDE_DIRECTORIES "$CMAKE_INCLUDEDIR_EXPR"
  )
endif()

if(NOT TARGET zstd::libzstd_static)
  add_library(zstd::libzstd_static STATIC IMPORTED)
  set_target_properties(zstd::libzstd_static PROPERTIES
    IMPORTED_LOCATION_NOCONFIG "$CMAKE_LIBDIR_EXPR/libzstd.a"
    INTERFACE_INCLUDE_DIRECTORIES "$CMAKE_INCLUDEDIR_EXPR"
  )
endif()

unset(_IMPORT_PREFIX)
EOF

cp "$SAFE_ROOT/cmake/zstdTargets.cmake" \
    "$OBJDIR/CMakeFiles/Export/libzstd-safe/zstdTargets.cmake"
if [[ $INSTALL_CMAKE -eq 1 ]]; then
    install -m 644 "$OBJDIR/CMakeFiles/Export/libzstd-safe/zstdTargets.cmake" \
        "$DESTDIR$LIBDIR/cmake/zstd/zstdTargets.cmake"
    install -m 644 "$TARGETS_NOCONFIG" \
        "$DESTDIR$LIBDIR/cmake/zstd/zstdTargets-noconfig.cmake"
fi

printf '%s\n' "$BUILD_SIGNATURE" >"$STAMP_FILE"
