#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_command cmake
phase6_ensure_safe_install
phase6_export_safe_env

PZSTD_DIR="$ORIGINAL_ROOT/contrib/pzstd"
GTEST_SRC=/usr/src/googletest
if [[ ! -d $GTEST_SRC ]]; then
    printf 'missing system googletest source tree: %s\n' "$GTEST_SRC" >&2
    exit 1
fi

SHIM_DIR="$PHASE6_OUT/pzstd-shim"
install -d "$SHIM_DIR"
cat >"$SHIM_DIR/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ ${1:-} == clone && ${2:-} == https://github.com/google/googletest ]]; then
    dest=${3:-googletest}
    src=${PHASE6_GTEST_SRC:?}
    rm -rf "$dest"
    cp -a "$src" "$dest"
    exit 0
fi

exec /usr/bin/git "$@"
EOF
chmod +x "$SHIM_DIR/git"

phase6_have_pzstd_toolchain() {
    local variant=${1:-}
    local source="$PHASE6_OUT/pzstd-toolchain-check.cpp"
    local binary="$PHASE6_OUT/pzstd-toolchain-check"
    local -a flags=(-std=c++14)

    case "$variant" in
        thread)
            flags+=(-fsanitize=thread -fuse-ld=gold)
            ;;
        address)
            flags+=(-fsanitize=address -fuse-ld=gold)
            ;;
        '')
            ;;
        *)
            printf 'unsupported pzstd toolchain probe: %s\n' "$variant" >&2
            exit 2
            ;;
    esac

    install -d "$PHASE6_OUT"
    case "$variant" in
        address)
            cat >"$source" <<EOF
#include <dlfcn.h>

int main(void) {
    void* handle = dlopen("${ORIGINAL_ROOT}/lib/libzstd.so.1.5.5", RTLD_NOW | RTLD_DEEPBIND);
    if (handle == nullptr) {
        return 1;
    }
    dlclose(handle);
    return 0;
}
EOF
            flags+=(-ldl)
            ;;
        *)
            printf 'int main(void) { return 0; }\n' >"$source"
            ;;
    esac
    g++ "${flags[@]}" "$source" -o "$binary" >/dev/null 2>&1 || return 1
    "$binary" >/dev/null 2>&1
}

run_pzstd_target() {
    local target=$1
    phase6_log "running pzstd target: $target"
    PATH="$SHIM_DIR:$PATH" \
    PHASE6_GTEST_SRC="$GTEST_SRC" \
    make -C "$PZSTD_DIR" \
        "$target" \
        ZSTDDIR="$HELPER_LIB_ROOT" \
        PROGDIR="$ORIGINAL_ROOT/programs" \
        CXXFLAGS="-O3 -Wall -Wextra -pedantic" \
        PZSTD_LDFLAGS="-Wl,-rpath,$LIBDIR" \
        PZSTD_CXX_STD="-std=c++14"
}

run_pzstd_target test-pzstd
phase6_export_safe_env
phase6_assert_uses_safe_lib "$PZSTD_DIR/pzstd"
if [[ -x $PZSTD_DIR/test/PzstdTest ]]; then
    phase6_assert_uses_safe_lib "$PZSTD_DIR/test/PzstdTest"
fi
make -C "$PZSTD_DIR" \
    roundtripcheck \
    ZSTDDIR="$HELPER_LIB_ROOT" \
    PROGDIR="$ORIGINAL_ROOT/programs" \
    CXXFLAGS="-O3 -Wall -Wextra -pedantic" \
    PZSTD_LDFLAGS="-Wl,-rpath,$LIBDIR" \
    PZSTD_CXX_STD="-std=c++14"
phase6_export_safe_env
phase6_assert_uses_safe_lib \
    "$PZSTD_DIR/pzstd"

if phase6_have_pkg gcc-multilib && phase6_have_pkg g++-multilib; then
    run_pzstd_target test-pzstd32
fi

if phase6_have_pzstd_toolchain thread; then
    run_pzstd_target test-pzstd-tsan
fi

if phase6_have_pzstd_toolchain address; then
    run_pzstd_target test-pzstd-asan
fi
