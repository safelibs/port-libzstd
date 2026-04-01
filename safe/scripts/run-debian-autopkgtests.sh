#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
METADATA_FILE="$SAFE_ROOT/out/deb/default/metadata.env"
UPSTREAM_CONTROL="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/debian/tests/control"

ensure_default_phase4_roots() {
    bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
    bash "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh"
    bash "$SAFE_ROOT/scripts/build-deb.sh"
}

ensure_default_phase4_roots

source "$METADATA_FILE"

if rg -n "original/libzstd-1.5.5\\+dfsg2" "$STAGE_ROOT/debian/tests" >/dev/null; then
    printf 'safe/debian/tests still reference ../original\n' >&2
    exit 1
fi

python3 - "$STAGE_ROOT/debian/tests/control" "$STAGE_ROOT" <<'PY'
from __future__ import annotations

import pathlib
import re
import sys

control = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
root = pathlib.Path(sys.argv[2])

for rel in sorted(set(re.findall(r"debian/tests/[A-Za-z0-9_./-]+", control))):
    if not (root / rel).exists():
        raise SystemExit(f"missing autopkgtest path: {rel}")
PY

python3 - "$UPSTREAM_CONTROL" "$STAGE_ROOT/debian/tests/control" <<'PY'
from __future__ import annotations

import pathlib
import sys


def features(path: pathlib.Path) -> list[str]:
    return [
        next(
            (line.split(": ", 1)[1] for line in block.splitlines() if line.startswith("Features: ")),
            "unknown",
        )
        for block in path.read_text(encoding="utf-8").strip().split("\n\n")
    ]


upstream = features(pathlib.Path(sys.argv[1]))
safe = features(pathlib.Path(sys.argv[2]))
if upstream != safe:
    raise SystemExit(
        "autopkgtest identities diverged: "
        f"upstream={upstream!r} safe={safe!r}"
    )
PY

AUTOPKGTEST_VENV="$SAFE_ROOT/out/deb/default/autopkgtest-venv"
if ! python3 - <<'PY' >/dev/null 2>&1
import importlib.util
raise SystemExit(0 if importlib.util.find_spec("click") and importlib.util.find_spec("typedload") else 1)
PY
then
    python3 -m venv "$AUTOPKGTEST_VENV"
    "$AUTOPKGTEST_VENV/bin/pip" install -r "$STAGE_ROOT/debian/tests/requirements/install.txt"
    export PATH="$AUTOPKGTEST_VENV/bin:$PATH"
fi

export PATH="$INSTALL_ROOT/usr/bin:$PATH"
export LD_LIBRARY_PATH="$INSTALL_ROOT/usr/lib/$MULTIARCH${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PKG_CONFIG_SYSROOT_DIR="$INSTALL_ROOT"
export PKG_CONFIG_LIBDIR="$INSTALL_ROOT/usr/lib/$MULTIARCH/pkgconfig"
export CMAKE_PREFIX_PATH="$INSTALL_ROOT/usr"

assert_binary_uses_safe_package_lib() {
    local binary=$1
    local expected="$INSTALL_ROOT/usr/lib/$MULTIARCH/libzstd.so.1"
    local resolved

    resolved=$(
        env LD_LIBRARY_PATH="$LD_LIBRARY_PATH" ldd "$binary" |
            awk '/libzstd\.so\.1 => / { print $3; exit }'
    )

    if [[ -z $resolved ]]; then
        printf 'unable to resolve libzstd for %s\n' "$binary" >&2
        exit 1
    fi
    if [[ $resolved != "$expected" ]]; then
        printf 'expected %s to load %s, resolved %s\n' "$binary" "$expected" "$resolved" >&2
        exit 1
    fi
}

assert_binary_uses_safe_package_lib "$INSTALL_ROOT/usr/bin/zstd"

python3 - "$STAGE_ROOT/debian/tests/control" <<'PY' |
from __future__ import annotations

import pathlib
import sys

paragraphs = [
    dict(
        line.split(": ", 1)
        for line in block.splitlines()
        if ": " in line
    )
    for block in pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").strip().split("\n\n")
]
for paragraph in paragraphs:
    feature = paragraph.get("Features", "unknown")
    command = paragraph["Test-Command"]
    print(f"{feature}\t{command}")
PY
while IFS=$'\t' read -r feature command; do
    printf 'running autopkgtest: %s\n' "$feature"
    (
        cd "$STAGE_ROOT"
        sh -ec "$command" </dev/null
    ) || {
        printf 'autopkgtest failed: %s\n' "$feature" >&2
        exit 1
    }
done
