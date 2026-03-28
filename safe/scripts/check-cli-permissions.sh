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

source_file="$WORK_DIR/source.bin"
compressed_file="$WORK_DIR/source.bin.zst"
decompressed_file="$WORK_DIR/source.bin.out"

dd if=/dev/zero of="$source_file" bs=1M count=8 status=none
chmod 0400 "$source_file"

phase6_log "checking CVE-2021-24031 creation mode on compression output"
(
    cd "$WORK_DIR"
    umask 0000
    strace -ff -qq \
        -e trace=open,openat,creat,chmod,fchmod,fchmodat \
        -o "$WORK_DIR/compress.trace" \
        "$BINDIR/zstd" -q -f "$source_file" -o "$compressed_file"
)

[[ $(stat -c %a "$compressed_file") == 400 ]] || {
    printf 'unexpected final mode for compressed file: %s\n' "$(stat -c %a "$compressed_file")" >&2
    exit 1
}
grep -E 'open(at)?\(.*source\.bin\.zst.*0600' "$WORK_DIR"/compress.trace* >/dev/null
if grep -E '([cf]h?mod(at)?\(.*source\.bin\.zst.*0600)' "$WORK_DIR"/compress.trace* >/dev/null; then
    printf 'detected a post-open restrictive chmod on compression output\n' >&2
    exit 1
fi

chmod 0400 "$compressed_file"

phase6_log "checking CVE-2021-24032 creation mode on decompression output"
(
    cd "$WORK_DIR"
    umask 0000
    strace -ff -qq \
        -e trace=open,openat,creat,chmod,fchmod,fchmodat \
        -o "$WORK_DIR/decompress.trace" \
        "$BINDIR/zstd" -q -f -d "$compressed_file" -o "$decompressed_file"
)

[[ $(stat -c %a "$decompressed_file") == 400 ]] || {
    printf 'unexpected final mode for decompressed file: %s\n' "$(stat -c %a "$decompressed_file")" >&2
    exit 1
}
grep -E 'open(at)?\(.*source\.bin\.out.*0600' "$WORK_DIR"/decompress.trace* >/dev/null
if grep -E '([cf]h?mod(at)?\(.*source\.bin\.out.*0600)' "$WORK_DIR"/decompress.trace* >/dev/null; then
    printf 'detected a post-open restrictive chmod on decompression output\n' >&2
    exit 1
fi
