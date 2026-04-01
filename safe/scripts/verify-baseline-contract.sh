#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)

bash "$SAFE_ROOT/scripts/verify-header-identity.sh"
bash "$SAFE_ROOT/scripts/capture-upstream-abi.sh" --check

python3 - "$SAFE_ROOT" "$REPO_ROOT" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys
import tomllib

safe_root = pathlib.Path(sys.argv[1])
repo_root = pathlib.Path(sys.argv[2])

required_files = [
    safe_root / "Cargo.toml",
    safe_root / "build.rs",
    safe_root / "rust-toolchain.toml",
    safe_root / "src/lib.rs",
    safe_root / "src/ffi/mod.rs",
    safe_root / "src/ffi/types.rs",
    safe_root / "src/ffi/symbols.rs",
    safe_root / "include/zstd.h",
    safe_root / "include/zdict.h",
    safe_root / "include/zstd_errors.h",
    safe_root / "abi/original.exports.txt",
    safe_root / "abi/original.soname.txt",
    safe_root / "abi/export_map.toml",
    safe_root / "tests/upstream_test_matrix.toml",
    safe_root / "tests/dependents/dependent_matrix.toml",
    safe_root / "tests/layout/public_types.c",
    safe_root / "tests/layout/public_types.rs",
    safe_root / "scripts/capture-upstream-abi.sh",
    safe_root / "scripts/verify-header-identity.sh",
    safe_root / "scripts/verify-baseline-contract.sh",
]
missing = [str(path) for path in required_files if not path.exists()]
if missing:
    raise SystemExit(f"missing planned outputs: {missing}")

for forbidden in ("vendor", "upstream"):
    if (safe_root / forbidden).exists():
        raise SystemExit(f"forbidden vendored tree detected: {safe_root / forbidden}")

export_map = tomllib.loads((safe_root / "abi/export_map.toml").read_text(encoding="utf-8"))
symbols = export_map["symbol"]
if len(symbols) != 185:
    raise SystemExit(f"expected 185 baseline exports, found {len(symbols)}")
if any(entry["owning_phase"] > 3 for entry in symbols):
    raise SystemExit("export_map.toml still contains entries above Phase 3")

export_names = []
for line in (safe_root / "abi/original.exports.txt").read_text(encoding="utf-8").splitlines():
    if not line or line.startswith("#"):
        continue
    export_names.append(line.split("\t", 1)[0])

map_names = [entry["name"] for entry in symbols]
if export_names != map_names:
    raise SystemExit("export_map.toml names do not match original.exports.txt")

phase_expectations = {
    "ZSTD_decompress": 1,
    "ZSTD_decompressDCtx": 1,
    "ZSTD_DCtx_reset": 1,
    "ZSTD_getDictID_fromFrame": 1,
    "ZSTD_compressBound": 2,
    "ZSTD_copyCCtx": 2,
    "ZSTD_flushStream": 2,
    "ZSTD_endStream": 2,
    "ZSTD_createThreadPool": 3,
    "ZSTD_freeThreadPool": 3,
    "ZSTD_CCtx_refThreadPool": 3,
    "ZSTD_estimateCStreamSize_usingCParams": 3,
    "ZDICT_addEntropyTablesFromBuffer": 3,
}
phase_lookup = {entry["name"]: entry["owning_phase"] for entry in symbols}
for symbol, expected_phase in phase_expectations.items():
    if phase_lookup.get(symbol) != expected_phase:
        raise SystemExit(f"{symbol} assigned to phase {phase_lookup.get(symbol)} instead of {expected_phase}")

owner_expectations = {
    "ZSTD_flushStream": "crate::compress::cstream",
    "ZSTD_endStream": "crate::compress::cstream",
    "ZSTD_copyCCtx": "crate::compress::cctx",
    "ZSTD_copyDCtx": "crate::decompress::dctx",
}
owner_lookup = {entry["name"]: entry["owner_module"] for entry in symbols}
for symbol, expected_owner in owner_expectations.items():
    if owner_lookup.get(symbol) != expected_owner:
        raise SystemExit(f"{symbol} assigned to owner module {owner_lookup.get(symbol)} instead of {expected_owner}")

upstream_matrix = tomllib.loads((safe_root / "tests/upstream_test_matrix.toml").read_text(encoding="utf-8"))
entries = upstream_matrix["entry"]
if any(entry["owning_phase"] > 5 for entry in entries):
    raise SystemExit("upstream_test_matrix.toml still contains entries above Phase 5")
entry_ids = {entry["id"] for entry in entries}
required_entry_ids = {
    "tests:all32",
    "tests:allnothread",
    "tests:fuzzer",
    "tests:fuzzer32",
    "tests:zstreamtest",
    "tests:zstreamtest32",
    "tests:zstreamtest_asan",
    "tests:zstreamtest_tsan",
    "tests:zstreamtest_ubsan",
    "tests:paramgrill",
    "tests:decodecorpus",
    "tests:poolTests",
    "tests:external_matchfinder",
    "tests:legacy",
    "tests:longmatch",
    "tests:bigdict",
    "tests:invalidDictionaries",
    "tests:roundTripCrash",
    "tests:fullbench",
    "tests:fullbench32",
    "tests:datagen",
    "tests:zstd-nolegacy",
    "tests:playTests.sh",
    "tests:test-variants.sh",
    "tests:test-zstd-versions.py",
    "tests:versionsTest",
    "tests:check_size.py",
    "tests:test-license.py",
    "tests:cli-tests",
    "tests:gzip",
    "tests:regression",
    "tests:fuzz",
    "zlibWrapper:test",
    "zlibWrapper:test-valgrind",
    "educational_decoder:test",
    "contrib/pzstd:test-pzstd",
    "contrib/pzstd:test-pzstd32",
    "contrib/pzstd:test-pzstd-tsan",
    "contrib/pzstd:test-pzstd-asan",
    "debian:zstd-selftest",
    "debian:build-pkg-config",
    "debian:build-cmake",
    "examples:test",
    "contrib/seekable_format:test",
}
missing_ids = sorted(required_entry_ids - entry_ids)
if missing_ids:
    raise SystemExit(f"upstream_test_matrix.toml missing required entries: {missing_ids}")

entry_lookup = {entry["id"]: entry for entry in entries}
make_target_expectations = {
    "tests:all32": "all32",
    "tests:allnothread": "allnothread",
    "tests:fuzzer32": "fuzzer32",
    "tests:fullbench32": "fullbench32",
    "tests:longmatch": "longmatch",
    "tests:versionsTest": "versionsTest",
    "tests:zstd-nolegacy": "zstd-nolegacy",
    "tests:zstreamtest32": "zstreamtest32",
    "tests:zstreamtest_asan": "zstreamtest_asan",
    "tests:zstreamtest_tsan": "zstreamtest_tsan",
    "tests:zstreamtest_ubsan": "zstreamtest_ubsan",
}
for entry_id, make_target in make_target_expectations.items():
    entry = entry_lookup[entry_id]
    if entry["kind"] != "make-target" or entry.get("make_target") != make_target:
        raise SystemExit(f"{entry_id} must stay mapped to make target {make_target}")

for entry_id in (
    "tests:all32",
    "tests:fuzzer32",
    "tests:fullbench32",
    "tests:zstreamtest32",
    "tests:zstreamtest_asan",
    "tests:zstreamtest_tsan",
    "tests:zstreamtest_ubsan",
):
    entry = entry_lookup[entry_id]
    if entry["owning_phase"] != 5 or entry["release_gate"]:
        raise SystemExit(f"{entry_id} must be a non-gating phase-5 preserved variant")

phase_matrix_expectations = {
    "tests:decodecorpus": (1, True),
    "tests:legacy": (1, True),
    "tests:paramgrill": (2, True),
    "tests:external_matchfinder": (2, True),
    "tests:bigdict": (2, True),
    "tests:invalidDictionaries": (2, True),
    "tests:roundTripCrash": (2, True),
    "tests:fullbench": (2, False),
    "tests:datagen": (2, False),
    "tests:longmatch": (2, True),
    "tests:fuzzer": (3, True),
    "tests:zstreamtest": (3, True),
    "tests:poolTests": (3, True),
    "debian:zstd-selftest": (4, True),
    "debian:build-pkg-config": (4, True),
    "debian:build-cmake": (4, True),
    "examples:test": (5, True),
    "contrib/seekable_format:test": (5, True),
}
for entry_id, (expected_phase, expected_gate) in phase_matrix_expectations.items():
    entry = entry_lookup[entry_id]
    if entry["owning_phase"] != expected_phase or entry["release_gate"] != expected_gate:
        raise SystemExit(
            f"{entry_id} must stay at phase {expected_phase} with release_gate={expected_gate}"
        )

if entry_lookup["tests:versionsTest"]["owning_phase"] != 5 or not entry_lookup["tests:versionsTest"]["release_gate"]:
    raise SystemExit("tests:versionsTest must be a release-gating phase-5 entry")

for autopkgtest_id in ("debian:zstd-selftest", "debian:build-pkg-config", "debian:build-cmake"):
    entry = entry_lookup[autopkgtest_id]
    if entry["owning_phase"] != 4 or not entry["release_gate"]:
        raise SystemExit(f"{autopkgtest_id} must be phase 4 release-gating")
    if autopkgtest_id == "debian:zstd-selftest":
        if entry["helper_paths"] != []:
            raise SystemExit("debian:zstd-selftest should not require helper paths")
    else:
        for helper in entry["helper_paths"]:
            if not helper.startswith("safe/debian/tests/"):
                raise SystemExit(f"{autopkgtest_id} helper path must stay under safe/debian/tests/: {helper}")

for entry_id in ("zlibWrapper:test", "zlibWrapper:test-valgrind", "educational_decoder:test", "contrib/pzstd:test-pzstd"):
    entry = entry_lookup[entry_id]
    if entry["owning_phase"] != 5 or not entry["release_gate"]:
        raise SystemExit(f"{entry_id} must be a release-gating phase-5 entry")

for entry_id in ("contrib/pzstd:test-pzstd32", "contrib/pzstd:test-pzstd-tsan", "contrib/pzstd:test-pzstd-asan"):
    entry = entry_lookup[entry_id]
    if entry["owning_phase"] != 5 or entry["release_gate"]:
        raise SystemExit(f"{entry_id} must be a non-gating phase-5 preserved entry")

dependent_matrix = tomllib.loads((safe_root / "tests/dependents/dependent_matrix.toml").read_text(encoding="utf-8"))
with (repo_root / "dependents.json").open("r", encoding="utf-8") as handle:
    dependents_json = json.load(handle)

matrix_entries = dependent_matrix["dependent"]
expected_sources = {
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
}
matrix_sources = {entry["source_package"] for entry in matrix_entries}
if matrix_sources != expected_sources:
    raise SystemExit(f"dependent matrix source packages mismatch: {sorted(matrix_sources)}")

runtime_lookup = {entry["source_package"]: entry["runtime_test"] for entry in matrix_entries}
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
if runtime_lookup != expected_runtime:
    raise SystemExit("dependent matrix runtime test mapping is out of sync with test-original.sh")

json_lookup = {entry["source_package"]: entry for entry in dependents_json["packages"]}
for entry in matrix_entries:
    json_entry = json_lookup[entry["source_package"]]
    if entry["binary_package"] != json_entry["binary_package"]:
        raise SystemExit(f"binary package mismatch for {entry['source_package']}")
    if not entry["compile_probe"].startswith("safe/tests/dependents/src/"):
        raise SystemExit(f"compile probe path must stay under safe/tests/dependents/src/: {entry['compile_probe']}")

layout_c = (safe_root / "tests/layout/public_types.c").read_text(encoding="utf-8")
layout_rs = (safe_root / "tests/layout/public_types.rs").read_text(encoding="utf-8")
for required_type in (
    "ZSTD_inBuffer",
    "ZSTD_outBuffer",
    "ZSTD_customMem",
    "ZSTD_frameHeader",
    "ZSTD_Sequence",
    "ZSTD_bounds",
    "ZSTD_frameProgression",
    "ZSTD_CCtx",
    "ZSTD_DCtx",
    "ZSTD_CDict",
    "ZSTD_DDict",
    "ZSTD_threadPool",
):
    if required_type not in layout_c or required_type not in layout_rs:
        raise SystemExit(f"layout probe missing type {required_type}")
PY
