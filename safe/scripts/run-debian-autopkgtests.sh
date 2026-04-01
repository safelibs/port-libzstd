#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
METADATA_FILE="$SAFE_ROOT/out/deb/default/metadata.env"
UPSTREAM_CONTROL="$REPO_ROOT/original/libzstd-1.5.5+dfsg2/debian/tests/control"
LEGACY_AUTOPKGTEST_VENV="$SAFE_ROOT/out/deb/default/autopkgtest-venv"
source "$SAFE_ROOT/scripts/phase6-common.sh"

rm -rf "$LEGACY_AUTOPKGTEST_VENV"
phase6_require_phase4_inputs "$0"

DEB_STAGE_ROOT=$PHASE6_DEB_STAGE_ROOT
DEB_INSTALL_ROOT=$PHASE6_DEB_INSTALL_ROOT
DEB_MULTIARCH=$MULTIARCH

STAMP_FILE=$(phase6_stamp_path run-debian-autopkgtests)
if phase6_stamp_is_fresh \
    "$STAMP_FILE" \
    "$0" \
    "$SCRIPT_DIR/phase6-common.sh" \
    "$METADATA_FILE" \
    "$UPSTREAM_CONTROL" \
    "$DEB_STAGE_ROOT/debian/tests/control" \
    "$DEB_STAGE_ROOT/debian/tests/requirements/install.txt" \
    "$DEB_INSTALL_ROOT/usr/bin/zstd"
then
    phase6_log "Debian autopkgtests already fresh; skipping rerun"
    exit 0
fi

if rg -n "original/libzstd-1.5.5\\+dfsg2" "$DEB_STAGE_ROOT/debian/tests" >/dev/null; then
    printf 'safe/debian/tests still reference ../original\n' >&2
    exit 1
fi

python3 - "$DEB_STAGE_ROOT/debian/tests/control" "$DEB_STAGE_ROOT" <<'PY'
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

python3 - "$UPSTREAM_CONTROL" "$DEB_STAGE_ROOT/debian/tests/control" <<'PY'
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

if ! python3 - <<'PY' >/dev/null 2>&1
import importlib.util
raise SystemExit(0 if importlib.util.find_spec("click") and importlib.util.find_spec("typedload") else 1)
PY
then
    AUTOPKGTEST_VENV_TMPDIR=$(mktemp -d "${TMPDIR:-/tmp}/libzstd-autopkgtest-venv.XXXXXX")
    trap 'rm -rf "$AUTOPKGTEST_VENV_TMPDIR"' EXIT
    AUTOPKGTEST_VENV="$AUTOPKGTEST_VENV_TMPDIR/venv"
    python3 -m venv "$AUTOPKGTEST_VENV"
    "$AUTOPKGTEST_VENV/bin/pip" install -r "$DEB_STAGE_ROOT/debian/tests/requirements/install.txt"
    export PATH="$AUTOPKGTEST_VENV/bin:$PATH"
fi

export PATH="$DEB_INSTALL_ROOT/usr/bin:$PATH"
export LD_LIBRARY_PATH="$DEB_INSTALL_ROOT/usr/lib/$DEB_MULTIARCH${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PKG_CONFIG_SYSROOT_DIR="$DEB_INSTALL_ROOT"
export PKG_CONFIG_LIBDIR="$DEB_INSTALL_ROOT/usr/lib/$DEB_MULTIARCH/pkgconfig"
export CMAKE_PREFIX_PATH="$DEB_INSTALL_ROOT/usr"

assert_binary_uses_safe_package_lib() {
    local binary=$1
    local expected="$DEB_INSTALL_ROOT/usr/lib/$DEB_MULTIARCH/libzstd.so.1"
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

assert_binary_uses_safe_package_lib "$DEB_INSTALL_ROOT/usr/bin/zstd"

python3 - "$DEB_STAGE_ROOT/debian/tests/control" <<'PY' |
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
        cd "$DEB_STAGE_ROOT"
        sh -ec "$command" </dev/null
    ) || {
        printf 'autopkgtest failed: %s\n' "$feature" >&2
        exit 1
    }
done

phase6_touch_stamp "$STAMP_FILE"
