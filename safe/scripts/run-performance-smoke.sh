#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env
phase6_assert_uses_safe_lib "$BINDIR/zstd"

STAMP_FILE=$(phase6_stamp_path run-performance-smoke)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$SAFE_ROOT/scripts" \
    "$VERSIONS_FIXTURE_ROOT" \
    "$REGRESSION_FIXTURE_ROOT" \
    "$BINDIR/zstd"
then
    phase6_log "performance smoke already fresh; skipping rerun"
    exit 0
fi

WORK_DIR="$PHASE6_OUT/performance-smoke"
CORPUS="$WORK_DIR/corpus.txt"
COMPRESSED="$WORK_DIR/corpus.txt.zst"
ROUNDTRIP="$WORK_DIR/corpus.roundtrip"

install -d "$WORK_DIR"

python3 - "$VERSIONS_FIXTURE_ROOT" "$REGRESSION_FIXTURE_ROOT" "$CORPUS" <<'PY'
from __future__ import annotations

import pathlib
import sys

versions_root = pathlib.Path(sys.argv[1])
regression_root = pathlib.Path(sys.argv[2])
output = pathlib.Path(sys.argv[3])

sources = [
    versions_root / "hello",
    versions_root / "helloworld",
    versions_root / "manifest.toml",
    regression_root / "README.md",
]

chunk = b"".join(path.read_bytes() for path in sources)
target_size = 16 * 1024 * 1024
repetitions = (target_size + len(chunk) - 1) // len(chunk)
payload = (chunk * repetitions)[:target_size]
output.write_bytes(payload)
PY

python3 - "$BINDIR/zstd" "$CORPUS" "$COMPRESSED" "$ROUNDTRIP" <<'PY'
from __future__ import annotations

import pathlib
import shutil
import subprocess
import sys
import time

zstd = pathlib.Path(sys.argv[1])
corpus = pathlib.Path(sys.argv[2])
compressed = pathlib.Path(sys.argv[3])
roundtrip = pathlib.Path(sys.argv[4])

max_compress = float(__import__("os").environ.get("PERF_SMOKE_MAX_COMPRESS_SECONDS", "15"))
max_decompress = float(__import__("os").environ.get("PERF_SMOKE_MAX_DECOMPRESS_SECONDS", "15"))
min_compress_mib = float(__import__("os").environ.get("PERF_SMOKE_MIN_COMPRESS_MIB_PER_SEC", "1.0"))
min_decompress_mib = float(__import__("os").environ.get("PERF_SMOKE_MIN_DECOMPRESS_MIB_PER_SEC", "2.0"))


def timed_run(*cmd: str, stdout=None) -> float:
    start = time.perf_counter()
    subprocess.run(list(cmd), check=True, stdout=stdout)
    return time.perf_counter() - start


size_mib = corpus.stat().st_size / (1024 * 1024)
compress = min(
    timed_run(str(zstd), "-q", "-f", "-3", "-o", str(compressed), str(corpus))
    for _ in range(3)
)
subprocess.run([str(zstd), "-q", "-t", str(compressed)], check=True)
decompress = min(
    timed_run(str(zstd), "-q", "-d", "-f", "-o", str(roundtrip), str(compressed))
    for _ in range(3)
)
with compressed.open("rb") as stream:
    stream_decode = timed_run(str(zstd), "-q", "-d", "-c", str(compressed), stdout=subprocess.DEVNULL)

if corpus.read_bytes() != roundtrip.read_bytes():
    raise SystemExit("performance smoke roundtrip mismatch")

compress_mib = size_mib / compress
decompress_mib = size_mib / max(decompress, stream_decode)

if compress > max_compress:
    raise SystemExit(f"compression smoke exceeded {max_compress}s: {compress:.3f}s")
if decompress > max_decompress:
    raise SystemExit(f"decompression smoke exceeded {max_decompress}s: {decompress:.3f}s")
if compress_mib < min_compress_mib:
    raise SystemExit(
        f"compression smoke throughput too low: {compress_mib:.2f} MiB/s < {min_compress_mib:.2f} MiB/s"
    )
if decompress_mib < min_decompress_mib:
    raise SystemExit(
        f"decompression smoke throughput too low: {decompress_mib:.2f} MiB/s < {min_decompress_mib:.2f} MiB/s"
    )

print(
    "performance smoke ok: "
    f"{size_mib:.1f} MiB corpus, "
    f"compress={compress:.3f}s ({compress_mib:.2f} MiB/s), "
    f"decompress={decompress:.3f}s, "
    f"stream-decode={stream_decode:.3f}s"
)
PY

phase6_touch_stamp "$STAMP_FILE"
