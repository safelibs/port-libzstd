#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_command strace
phase6_ensure_safe_install
phase6_export_safe_env

WORK_DIR="$PHASE6_OUT/cli-permissions"
rm -rf "$WORK_DIR"
install -d "$WORK_DIR"

require_atomic_mode() {
    local trace_prefix=${1:?missing trace prefix}
    local target_regex=${2:?missing target regex}
    local final_mode=${3:?missing final mode}
    local description=${4:?missing description}

    if ! grep -E "(open(at)?|creat)\\(.*${target_regex}.*0${final_mode}" "$WORK_DIR"/"${trace_prefix}".trace* >/dev/null; then
        printf 'missing atomic create mode 0%s for %s\n' "$final_mode" "$description" >&2
        exit 1
    fi
    if grep -E "(open(at)?|creat)\\(.*${target_regex}.*0600" "$WORK_DIR"/"${trace_prefix}".trace* >/dev/null; then
        printf 'detected temporary 0600 create mode for %s\n' "$description" >&2
        exit 1
    fi
    if grep -E "(fchmod|fchmodat|chmod)\\(.*${target_regex}.*0[0-7]{3}" "$WORK_DIR"/"${trace_prefix}".trace* >/dev/null; then
        printf 'detected a post-open chmod on %s\n' "$description" >&2
        exit 1
    fi
}

source_file="$WORK_DIR/source.bin"
compressed_file="$WORK_DIR/source.bin.zst"
decompressed_file="$WORK_DIR/source.bin.out"

dd if=/dev/zero of="$source_file" bs=1M count=8 status=none
chmod 0400 "$source_file"

phase6_log "checking CVE-2021-24031 creation mode on compression output"
(
    cd "$WORK_DIR"
    umask 0000
    strace -ff -qq -y \
        -e trace=open,openat,creat,chmod,fchmod,fchmodat \
        -o "$WORK_DIR/compress.trace" \
        "$BINDIR/zstd" -q -f "$source_file" -o "$compressed_file"
)

[[ $(stat -c %a "$compressed_file") == 400 ]] || {
    printf 'unexpected final mode for compressed file: %s\n' "$(stat -c %a "$compressed_file")" >&2
    exit 1
}
require_atomic_mode compress 'source\.bin\.zst' 400 'compression output'

chmod 0400 "$compressed_file"

phase6_log "checking CVE-2021-24032 creation mode on decompression output"
(
    cd "$WORK_DIR"
    umask 0000
    strace -ff -qq -y \
        -e trace=open,openat,creat,chmod,fchmod,fchmodat \
        -o "$WORK_DIR/decompress.trace" \
        "$BINDIR/zstd" -q -f -d "$compressed_file" -o "$decompressed_file"
)

[[ $(stat -c %a "$decompressed_file") == 400 ]] || {
    printf 'unexpected final mode for decompressed file: %s\n' "$(stat -c %a "$decompressed_file")" >&2
    exit 1
}
require_atomic_mode decompress 'source\.bin\.out' 400 'decompression output'
