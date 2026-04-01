#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$SCRIPT_DIR
DEPENDENTS_JSON="$REPO_ROOT/dependents.json"
DEPENDENT_MATRIX="$REPO_ROOT/safe/tests/dependents/dependent_matrix.toml"
SOURCE_DIR="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"
SAFE_ROOT="$REPO_ROOT/safe"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 1
fi

if [ ! -d "$SOURCE_DIR" ]; then
  echo "missing source tree: $SOURCE_DIR" >&2
  exit 1
fi

python3 - "$DEPENDENTS_JSON" "$DEPENDENT_MATRIX" "$REPO_ROOT" <<'PY'
import json
import pathlib
import sys
import tomllib

expected = [
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

with open(sys.argv[1], "r", encoding="utf-8") as f:
    data = json.load(f)
with open(sys.argv[2], "rb") as f:
    matrix = tomllib.load(f)

repo_root = pathlib.Path(sys.argv[3])

actual = [pkg["source_package"] for pkg in data["packages"]]
matrix_lookup = {entry["source_package"]: entry for entry in matrix["dependent"]}
matrix_sources = [entry["source_package"] for entry in matrix["dependent"]]

if actual != expected or matrix_sources != expected:
    raise SystemExit(
        "dependents.json mismatch: "
        f"json={actual} matrix={matrix_sources} expected={expected}"
    )

for source_package, entry in matrix_lookup.items():
    probe = repo_root / entry["compile_probe"]
    if not probe.is_file():
        raise SystemExit(f"missing dependent compile probe for {source_package}: {probe}")
PY

source "$SAFE_ROOT/scripts/phase6-common.sh"
phase6_require_phase4_inputs

bash "$SAFE_ROOT/scripts/build-dependent-image.sh"
bash "$SAFE_ROOT/scripts/run-dependent-matrix.sh" "$@"
