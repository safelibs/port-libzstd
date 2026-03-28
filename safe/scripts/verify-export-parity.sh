#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

cargo rustc --manifest-path "$SAFE_ROOT/Cargo.toml" --release --crate-type cdylib

python3 - "$SAFE_ROOT" <<'PY'
from __future__ import annotations

import pathlib
import re
import subprocess
import sys
import tomllib

safe_root = pathlib.Path(sys.argv[1])
export_map_path = safe_root / "abi/export_map.toml"
baseline_path = safe_root / "abi/original.exports.txt"
soname_path = safe_root / "abi/original.soname.txt"
shared_object = safe_root / "target/release/libzstd.so"

export_map = tomllib.loads(export_map_path.read_text(encoding="utf-8"))
symbols = export_map["symbol"]
statuses = {entry["name"]: entry.get("status", "") for entry in symbols}
unresolved = sorted(name for name, status in statuses.items() if status != "implemented")
if unresolved:
    raise SystemExit(f"export_map.toml still has unresolved entries: {unresolved}")

expected_rows = []
for line in baseline_path.read_text(encoding="utf-8").splitlines():
    if not line or line.startswith("#"):
        continue
    name, bind, type_class, section, version_class, size_hex, value_hex = line.split("\t")
    expected_rows.append(
        {
            "name": name,
            "bind": bind,
            "type_class": type_class,
            "section": section,
            "version_class": version_class,
            "size_hex": size_hex,
            "value_hex": value_hex,
        }
    )

expected_by_name = {row["name"]: row for row in expected_rows}
map_names = [entry["name"] for entry in symbols]
expected_names = [row["name"] for row in expected_rows]
if map_names != expected_names:
    raise SystemExit("export_map.toml names are out of sync with original.exports.txt")

for entry in symbols:
    expected = expected_by_name[entry["name"]]
    for key in ("binding", "type_class", "version_class"):
        lhs = entry[key]
        rhs = expected["bind" if key == "binding" else key]
        if lhs != rhs:
            raise SystemExit(
                f"export_map.toml mismatch for {entry['name']} field {key}: {lhs!r} != {rhs!r}"
            )

expected_soname = soname_path.read_text(encoding="utf-8").strip()
if export_map["upstream_soname"] != expected_soname:
    raise SystemExit(
        f"export_map upstream_soname mismatch: {export_map['upstream_soname']!r} != {expected_soname!r}"
    )

readelf_output = subprocess.check_output(["readelf", "-d", str(shared_object)], text=True)
actual_soname = None
for line in readelf_output.splitlines():
    if "SONAME" not in line:
        continue
    match = re.search(r"\[(?P<soname>[^\]]+)\]", line)
    if match:
        actual_soname = match.group("soname")
        break
if actual_soname != expected_soname:
    raise SystemExit(
        f"{shared_object.name} SONAME mismatch: {actual_soname!r} != {expected_soname!r}"
    )

objdump_output = subprocess.check_output(["objdump", "-T", str(shared_object)], text=True)
line_re = re.compile(
    r"^(?P<value>[0-9a-fA-F]+)\s+(?P<bind>\S+)\s+(?P<type>\S+)\s+(?P<section>\S+)\s+"
    r"(?P<size>[0-9a-fA-F]+)\s+(?P<version>\S+)\s+(?P<name>\S+)$"
)
actual_rows = []
for line in objdump_output.splitlines():
    match = line_re.match(line.strip())
    if not match:
        continue
    row = match.groupdict()
    if row["section"] == "*UND*":
        continue
    actual_rows.append(row)

actual_by_name = {row["name"]: row for row in actual_rows}
missing = [name for name in expected_names if name not in actual_by_name]
extra = sorted(name for name in actual_by_name if name not in expected_by_name)
if missing:
    raise SystemExit(f"missing public exports: {missing}")
if extra:
    raise SystemExit(f"unexpected extra public exports: {extra}")

for name, expected in expected_by_name.items():
    actual = actual_by_name[name]
    if actual["bind"] != expected["bind"]:
        raise SystemExit(f"{name} binding mismatch: {actual['bind']} != {expected['bind']}")
    if actual["type"] != expected["type_class"]:
        raise SystemExit(f"{name} type mismatch: {actual['type']} != {expected['type_class']}")
    actual_version = actual["version"].strip("()")
    if actual_version != expected["version_class"]:
        raise SystemExit(
            f"{name} version mismatch: {actual_version} != {expected['version_class']}"
        )

print(
    f"verified export parity for {len(expected_rows)} symbols against {shared_object.name}"
)
PY
