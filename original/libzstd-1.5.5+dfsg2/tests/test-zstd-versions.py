#!/usr/bin/env python3
"""Test zstd interoperability using checked-in offline fixtures."""

# ################################################################
# Copyright (c) Meta Platforms, Inc. and affiliates.
# All rights reserved.
#
# This source code is licensed under both the BSD-style license (found in the
# LICENSE file in the root directory of this source tree) and the GPLv2 (found
# in the COPYING file in the root directory of this source tree).
# You may select, at your option, one of the above-listed licenses.
# ################################################################

from pathlib import Path
import filecmp
import os
import subprocess
import tomllib


def run(cmd, *, stdout=None):
    result = subprocess.run(cmd, stdout=stdout, stderr=subprocess.PIPE, check=False)
    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(cmd)}\n{stderr}")


def require_file(path: Path) -> None:
    if not path.is_file():
        raise FileNotFoundError(f"missing required fixture: {path}")


def main() -> None:
    repo_root = Path(__file__).resolve().parents[3]
    fixture_root = Path(
        os.environ.get(
            "PHASE6_VERSION_FIXTURE_ROOT",
            repo_root / "safe" / "tests" / "fixtures" / "versions",
        )
    )
    work_dir = Path(
        os.environ.get(
            "PHASE6_VERSION_WORK_DIR",
            repo_root / "safe" / "out" / "phase6" / "version-compat",
        )
    )
    zstd_bin = Path(
        os.environ.get(
            "ZSTD_VERSION_BIN",
            repo_root / "safe" / "out" / "install" / "release-default" / "usr" / "bin" / "zstd",
        )
    )

    require_file(zstd_bin)
    manifest_path = fixture_root / "manifest.toml"
    require_file(manifest_path)
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    fixtures = manifest["fixtures"]

    work_dir.mkdir(parents=True, exist_ok=True)

    test_only = (
        fixture_root / fixtures["empty_block"],
        fixture_root / fixtures["rle_first_block"],
        fixture_root / fixtures["huffman_compressed_larger"],
    )
    for fixture in test_only:
        require_file(fixture)
        run([str(zstd_bin), "-q", "-t", str(fixture)])

    roundtrip_pairs = (
        ("hello", fixtures["hello"]),
        ("helloworld", fixtures["helloworld"]),
    )
    for plain_name, compressed_name in roundtrip_pairs:
        plain = fixture_root / plain_name
        compressed = fixture_root / compressed_name
        require_file(plain)
        require_file(compressed)
        decoded = work_dir / f"{plain_name}.out"
        with decoded.open("wb") as handle:
            run([str(zstd_bin), "-q", "-d", "-c", str(compressed)], stdout=handle)
        if not filecmp.cmp(plain, decoded, shallow=False):
            raise RuntimeError(f"decoded output drifted for {compressed.name}")

    http_sample = fixture_root / fixtures["http_sample"]
    http_dict = fixture_root / fixtures["http_dict"]
    require_file(http_sample)
    require_file(http_dict)
    http_zst = work_dir / "http.zst"
    http_out = work_dir / "http.out"
    run([
        str(zstd_bin),
        "-q",
        "-f",
        "-D",
        str(http_dict),
        str(http_sample),
        "-o",
        str(http_zst),
    ])
    with http_out.open("wb") as handle:
        run([
            str(zstd_bin),
            "-q",
            "-d",
            "-c",
            "-D",
            str(http_dict),
            str(http_zst),
        ], stdout=handle)
    if not filecmp.cmp(http_sample, http_out, shallow=False):
        raise RuntimeError("decoded output drifted for dictionary fixture")

    print("offline version-compatibility fixtures passed")


if __name__ == "__main__":
    main()
