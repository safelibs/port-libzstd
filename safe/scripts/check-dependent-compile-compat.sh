#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
BUILD_DIR=${DEPENDENT_BUILD_DIR:-$SAFE_ROOT/out/dependents/compile-compat}

python3 - "$SAFE_ROOT" "$REPO_ROOT" "$BUILD_DIR" <<'PY'
from __future__ import annotations

import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tomllib

safe_root = pathlib.Path(sys.argv[1])
repo_root = pathlib.Path(sys.argv[2])
build_dir = pathlib.Path(sys.argv[3])
matrix_path = safe_root / "tests/dependents/dependent_matrix.toml"
dependents_path = repo_root / "dependents.json"

pkg_config = shutil.which("pkg-config") or shutil.which("pkgconf")
if pkg_config is None:
    raise SystemExit("missing pkg-config or pkgconf")

compiler = os.environ.get("CC", "cc")
common_flags = [
    "-std=c11",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-Wno-deprecated-declarations",
]

matrix = tomllib.loads(matrix_path.read_text(encoding="utf-8"))
dependents = json.loads(dependents_path.read_text(encoding="utf-8"))
expected_sources = [
    "apt",
    "dpkg",
    "rsync",
    "systemd",
    "libarchive",
    "btrfs-progs",
    "squashfs-tools",
    "qemu",
    "curl",
    "tiff",
    "rpm",
    "zarchive",
]
expected_runtime = {
    "apt": "test_apt",
    "dpkg": "test_dpkg",
    "rsync": "test_rsync",
    "systemd": "test_systemd",
    "libarchive": "test_libarchive",
    "btrfs-progs": "test_btrfs",
    "squashfs-tools": "test_squashfs",
    "qemu": "test_qemu",
    "curl": "test_curl",
    "tiff": "test_tiff",
    "rpm": "test_rpm",
    "zarchive": "test_zarchive",
}

matrix_list = matrix["dependent"]
matrix_entries = {entry["source_package"]: entry for entry in matrix_list}
source_order = [entry["source_package"] for entry in dependents["packages"]]
matrix_order = [entry["source_package"] for entry in matrix_list]
runtime_lookup = {entry["source_package"]: entry["runtime_test"] for entry in matrix_list}
if source_order != expected_sources:
    raise SystemExit(f"dependents.json source packages drifted: {source_order}")
if matrix_order != expected_sources:
    raise SystemExit(f"dependent_matrix.toml source packages drifted: {matrix_order}")
if runtime_lookup != expected_runtime:
    raise SystemExit("dependent runtime test mapping drifted from the frozen Phase 6 matrix")

libzstd_dev = subprocess.run(
    ["dpkg-query", "-W", "-f=${Version}", "libzstd-dev"],
    check=False,
    capture_output=True,
    text=True,
)
if libzstd_dev.returncode != 0:
    raise SystemExit("libzstd-dev is not installed")
installed_version = libzstd_dev.stdout.strip()
if "safelibs" not in installed_version:
    raise SystemExit(f"libzstd-dev version does not look safe: {installed_version}")

multiarch = ""
if shutil.which("dpkg-architecture") is not None:
    multiarch = subprocess.check_output(
        ["dpkg-architecture", "-qDEB_HOST_MULTIARCH"],
        text=True,
    ).strip()
expected_lib = pathlib.Path(f"/usr/lib/{multiarch}/libzstd.so.1") if multiarch else None

build_dir.mkdir(parents=True, exist_ok=True)


def pkg_config_tokens(flag: str, modules: list[str]) -> list[str]:
    return subprocess.check_output([pkg_config, flag, *modules], text=True).split()


def resolve_linked_lib(binary: pathlib.Path) -> pathlib.Path | None:
    ldd_output = subprocess.check_output(["ldd", str(binary)], text=True)
    match = re.search(r"libzstd\.so(?:\.1)? => (\S+)", ldd_output)
    if match is None:
        return None
    return pathlib.Path(os.path.realpath(match.group(1)))


for source_package in source_order:
    entry = matrix_entries[source_package]
    json_entry = next(
        item for item in dependents["packages"] if item["source_package"] == source_package
    )
    if entry["binary_package"] != json_entry["binary_package"]:
        raise SystemExit(f"binary package mismatch for {source_package}")

    probe = repo_root / entry["compile_probe"]
    if not probe.is_file():
        raise SystemExit(f"missing compile probe: {probe}")

    modules = entry.get("pkg_config_modules", ["libzstd"])
    if entry["compile_mode"] != "pkg-config-c":
        raise SystemExit(
            f"unsupported compile_mode for {source_package}: {entry['compile_mode']}"
        )

    out_dir = build_dir / source_package
    out_dir.mkdir(parents=True, exist_ok=True)
    binary = out_dir / probe.stem
    cmd = [
        compiler,
        *common_flags,
        *pkg_config_tokens("--cflags", modules),
        str(probe),
        "-o",
        str(binary),
        *pkg_config_tokens("--libs", modules),
    ]
    subprocess.run(cmd, check=True)

    if expected_lib is not None:
        linked_lib = resolve_linked_lib(binary)
        if linked_lib is None:
            raise SystemExit(f"{binary} did not link against libzstd.so.1")
        if linked_lib != pathlib.Path(os.path.realpath(expected_lib)):
            raise SystemExit(
                f"{binary} resolved {linked_lib} instead of {expected_lib}"
            )

    print(
        f"compiled {source_package}: {probe.relative_to(repo_root)} "
        f"against libzstd-dev {installed_version}"
    )

print(f"compiled {len(source_order)} dependent probes into {build_dir}")
PY
