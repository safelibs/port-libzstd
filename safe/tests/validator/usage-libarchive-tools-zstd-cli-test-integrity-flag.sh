#!/usr/bin/env bash
# Standalone reproducer for the validator testcase
# `usage-libarchive-tools-zstd-cli-test-integrity-flag`. Mirrors the same
# corruption pattern (one byte flipped at offset 10 of a freshly compressed
# zstd frame) and asserts the safe `zstd -t` CLI accepts the valid frame and
# rejects the corrupted one.

set -euo pipefail

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

src="$tmpdir/payload.txt"
printf 'integrity-flag payload\n' >"$src"

zstd -q -o "$tmpdir/good.zst" "$src"
[[ -s "$tmpdir/good.zst" ]] || {
  echo "expected zstd to produce $tmpdir/good.zst" >&2
  exit 1
}

magic=$(od -An -N4 -tx1 "$tmpdir/good.zst" | tr -d ' \n')
[[ "$magic" == "28b52ffd" ]] || {
  echo "unexpected zstd magic: $magic" >&2
  exit 1
}

zstd -tq "$tmpdir/good.zst"

cp "$tmpdir/good.zst" "$tmpdir/bad.zst"
size=$(stat -c %s "$tmpdir/bad.zst")
[[ "$size" -gt 12 ]]
printf '\xff' | dd of="$tmpdir/bad.zst" bs=1 seek=10 count=1 conv=notrunc status=none

if zstd -tq "$tmpdir/bad.zst" >/dev/null 2>&1; then
  echo "zstd -t accepted a corrupted frame at $tmpdir/bad.zst" >&2
  exit 1
fi
