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

import filecmp
import hashlib
import os
import subprocess
import tomllib
from pathlib import Path


def run(cmd, *, stdout=None):
    result = subprocess.run(cmd, stdout=stdout, stderr=subprocess.PIPE, check=False)
    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(cmd)}\n{stderr}")


def run_capture(cmd) -> bytes:
    result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(cmd)}\n{stderr}")
    return result.stdout


def require_file(path: Path) -> None:
    if not path.is_file():
        raise FileNotFoundError(f"missing required fixture: {path}")


def resolve_path(root: Path, value: str) -> Path:
    candidate = Path(value)
    if not candidate.is_absolute():
        candidate = (root / candidate).resolve()
    return candidate


def sha256_of_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1 << 20)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def verify_historical_release_fixture(
    fixture: dict,
    fixture_root: Path,
    work_dir: Path,
    zstd_bin: Path,
) -> None:
    plain = resolve_path(fixture_root, fixture["plain"])
    frame = resolve_path(fixture_root, fixture["frame"])
    require_file(plain)
    require_file(frame)
    actual_sha256 = sha256_of_file(frame)
    if actual_sha256 != fixture["sha256"]:
        raise RuntimeError(
            f"historical fixture hash drifted for {fixture['id']}: "
            f"expected {fixture['sha256']} got {actual_sha256}"
        )
    run([str(zstd_bin), "-q", "-t", str(frame)])
    decoded = work_dir / f"{fixture['id']}.out"
    with decoded.open("wb") as handle:
        run([str(zstd_bin), "-q", "-d", "-c", str(frame)], stdout=handle)
    if not filecmp.cmp(plain, decoded, shallow=False):
        raise RuntimeError(f"decoded output drifted for historical release {fixture['release']}")

    print(
        f"{fixture['id']}: validated {fixture['release']} "
        f"via {fixture['generator']}"
    )


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
    if manifest.get("schema_version") != 3:
        raise RuntimeError(f"unsupported fixture manifest schema: {manifest.get('schema_version')}")

    roundtrip = manifest["roundtrip"]
    modern = manifest["modern_compat"]
    historical_fixtures = manifest["historical_release_fixture"]

    work_dir.mkdir(parents=True, exist_ok=True)

    test_only = (
        resolve_path(fixture_root, modern["empty_block"]),
        resolve_path(fixture_root, modern["rle_first_block"]),
        resolve_path(fixture_root, modern["huffman_compressed_larger"]),
    )
    for fixture in test_only:
        require_file(fixture)
        run([str(zstd_bin), "-q", "-t", str(fixture)])

    roundtrip_pairs = (
        (roundtrip["hello_plain"], roundtrip["hello_frame"]),
        (roundtrip["helloworld_plain"], roundtrip["helloworld_frame"]),
    )
    for plain_name, compressed_name in roundtrip_pairs:
        plain = resolve_path(fixture_root, plain_name)
        compressed = resolve_path(fixture_root, compressed_name)
        require_file(plain)
        require_file(compressed)
        decoded = work_dir / f"{plain_name}.out"
        with decoded.open("wb") as handle:
            run([str(zstd_bin), "-q", "-d", "-c", str(compressed)], stdout=handle)
        if not filecmp.cmp(plain, decoded, shallow=False):
            raise RuntimeError(f"decoded output drifted for {compressed.name}")

    http_sample = resolve_path(fixture_root, modern["http_sample"])
    http_dict = resolve_path(fixture_root, modern["http_dict"])
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

    for fixture in historical_fixtures:
        verify_historical_release_fixture(fixture, fixture_root, work_dir, zstd_bin)

    print("offline version-compatibility fixtures passed")


if __name__ == "__main__":
    main()
