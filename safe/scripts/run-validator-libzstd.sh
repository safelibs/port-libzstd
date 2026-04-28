#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
VALIDATOR_ROOT="$REPO_ROOT/validator"
export REPO_ROOT SAFE_ROOT VALIDATOR_ROOT

if [[ ! -d "$VALIDATOR_ROOT/.git" ]]; then
    printf 'missing validator checkout at %s\n' "$VALIDATOR_ROOT" >&2
    exit 1
fi

cd "$REPO_ROOT"

VALIDATOR_OUT="$SAFE_ROOT/out/validator"
mkdir -p "$VALIDATOR_OUT"

if python3 -c 'import yaml' >/dev/null 2>&1; then
    PYTHON=python3
else
    VENV="$VALIDATOR_OUT/venv"
    if [[ ! -x "$VENV/bin/python" ]]; then
        python3 -m venv "$VENV"
    fi
    if ! "$VENV/bin/python" -c 'import yaml' >/dev/null 2>&1; then
        "$VENV/bin/python" -m pip install --upgrade pip PyYAML
    fi
    PYTHON="$VENV/bin/python"
fi
export PYTHON

bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
bash "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh"
env -u DEB_BUILD_PROFILES bash "$SAFE_ROOT/scripts/build-deb.sh"

metadata="$SAFE_ROOT/out/deb/default/metadata.env"
package_dir="$SAFE_ROOT/out/deb/default/packages"
install_root="$SAFE_ROOT/out/deb/default/stage-root"
test -f "$metadata"
(
    set -euo pipefail
    # shellcheck source=/dev/null
    source "$metadata"
    test "${BUILD_TAG:-}" = default
    test -z "${PROFILES:-}"
    test "${PACKAGE_DIR:-}" = "$package_dir"
    test "${INSTALL_ROOT:-}" = "$install_root"
)

override_leaf="$VALIDATOR_OUT/override-debs/libzstd"
rm -rf "$override_leaf"
mkdir -p "$override_leaf"
shopt -s nullglob
for pkg in libzstd1 libzstd-dev zstd; do
    matches=("$package_dir"/${pkg}_*.deb)
    if [[ ${#matches[@]} -ne 1 ]]; then
        printf 'expected exactly one %s .deb, found %d\n' "$pkg" "${#matches[@]}" >&2
        exit 1
    fi
    cp -a "${matches[0]}" "$override_leaf/"
done
shopt -u nullglob
deb_count=$(find "$override_leaf" -maxdepth 1 -type f -name '*.deb' | wc -l)
if [[ "$deb_count" -ne 3 ]]; then
    printf 'validator override leaf must contain exactly three .deb files, found %s\n' "$deb_count" >&2
    exit 1
fi
if find "$override_leaf" -maxdepth 1 -type f -name 'libzstd1-udeb_*' | grep -q .; then
    printf 'validator override leaf must not include libzstd1-udeb\n' >&2
    exit 1
fi

artifact_root="$VALIDATOR_OUT/artifacts"
proof_root="$artifact_root/proof"
rm -rf "$artifact_root/port-04-test"
mkdir -p "$proof_root"
rm -f \
    "$proof_root/port-04-test-debs-lock.json" \
    "$proof_root/port-04-test-validation-proof.json"

"$PYTHON" - <<'PY'
from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
from datetime import datetime, timezone

repo_root = pathlib.Path.cwd()
override_leaf = repo_root / "safe/out/validator/override-debs/libzstd"
lock_path = repo_root / "safe/out/validator/artifacts/proof/port-04-test-debs-lock.json"
packages = ["libzstd1", "libzstd-dev", "zstd"]
commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
if len(commit) != 40 or any(ch not in "0123456789abcdef" for ch in commit):
    raise SystemExit(f"git HEAD must be a 40-character lowercase hex commit, got {commit!r}")

debs = []
for package in packages:
    matches = sorted(override_leaf.glob(f"{package}_*.deb"))
    if len(matches) != 1:
        raise SystemExit(f"expected exactly one staged {package} .deb, found {len(matches)}")
    path = matches[0]
    architecture = subprocess.check_output(["dpkg-deb", "-f", str(path), "Architecture"], text=True).strip()
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    debs.append(
        {
            "package": package,
            "filename": path.name,
            "architecture": architecture,
            "sha256": digest,
            "size": path.stat().st_size,
        }
    )

lock = {
    "schema_version": 1,
    "mode": "port-04-test",
    "generated_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "source_config": "validator/repositories.yml",
    "source_inventory": "local override packages from safe/out/validator/override-debs/libzstd",
    "libraries": [
        {
            "library": "libzstd",
            "repository": "local/port-libzstd",
            "tag_ref": "refs/tags/libzstd/04-test-local",
            "commit": commit,
            "release_tag": f"build-{commit[:12]}",
            "debs": debs,
            "unported_original_packages": [],
        }
    ],
}
lock_path.parent.mkdir(parents=True, exist_ok=True)
lock_path.write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

PYTHON="$PYTHON" make -C "$VALIDATOR_ROOT" unit
PYTHON="$PYTHON" make -C "$VALIDATOR_ROOT" check-testcases

VALIDATOR_TESTS_ROOT="$VALIDATOR_ROOT/tests"
VALIDATOR_MIN_SOURCE_CASES=5
VALIDATOR_MIN_USAGE_CASES=80
VALIDATOR_MIN_CASES=85
skip_env="$VALIDATOR_OUT/skip.env"
if [[ -f "$skip_env" ]]; then
    "$PYTHON" - "$skip_env" "$REPO_ROOT/validator-report.md" <<'PY'
from __future__ import annotations

import pathlib
import re
import shlex
import sys

skip_env = pathlib.Path(sys.argv[1])
report = pathlib.Path(sys.argv[2])
text = skip_env.read_text(encoding="utf-8")
match = re.search(r"(?m)^VALIDATOR_SKIPPED_CASES=(.*)$", text)
if match is None:
    raise SystemExit("skip.env exists but does not define VALIDATOR_SKIPPED_CASES")
try:
    parts = shlex.split(match.group(1))
except ValueError as exc:
    raise SystemExit(f"cannot parse VALIDATOR_SKIPPED_CASES from skip.env without sourcing: {exc}") from exc
skipped_cases = []
for part in parts:
    skipped_cases.extend(case for case in part.split() if case)
if not skipped_cases:
    raise SystemExit("skip.env defines no VALIDATOR_SKIPPED_CASES entries")
report_text = report.read_text(encoding="utf-8")
missing = [case for case in skipped_cases if case not in report_text]
if missing:
    raise SystemExit("validator-report.md does not mention skipped case(s): " + ", ".join(missing))
PY
    # shellcheck source=/dev/null
    source "$skip_env"
    : "${VALIDATOR_TESTS_ROOT:?}"
    : "${VALIDATOR_MIN_SOURCE_CASES:?}"
    : "${VALIDATOR_MIN_USAGE_CASES:?}"
    : "${VALIDATOR_MIN_CASES:?}"
    test -d "$VALIDATOR_TESTS_ROOT/libzstd"
    test -d "$VALIDATOR_TESTS_ROOT/tests/libzstd"
    "$PYTHON" "$VALIDATOR_ROOT/tools/testcases.py" \
        --config "$VALIDATOR_ROOT/repositories.yml" \
        --tests-root "$VALIDATOR_TESTS_ROOT" \
        --library libzstd \
        --check \
        --min-source-cases "$VALIDATOR_MIN_SOURCE_CASES" \
        --min-usage-cases "$VALIDATOR_MIN_USAGE_CASES" \
        --min-cases "$VALIDATOR_MIN_CASES"
fi

validator_status=0
PYTHON="$PYTHON" bash "$VALIDATOR_ROOT/test.sh" \
    --config "$VALIDATOR_ROOT/repositories.yml" \
    --tests-root "$VALIDATOR_TESTS_ROOT" \
    --artifact-root "$artifact_root" \
    --mode port-04-test \
    --library libzstd \
    --override-deb-root "$VALIDATOR_OUT/override-debs" \
    --port-deb-lock "$proof_root/port-04-test-debs-lock.json" \
    --record-casts || validator_status=$?

summary_path="$artifact_root/port-04-test/results/libzstd/summary.json"
summary_failed=1
if [[ -f "$summary_path" ]]; then
    summary_failed=$("$PYTHON" -c 'import json,sys; print(int(json.load(open(sys.argv[1], encoding="utf-8"))["failed"]))' "$summary_path")
else
    printf 'missing validator summary: %s\n' "$summary_path" >&2
fi

strict_status="$validator_status"
if [[ "$summary_failed" -ne 0 ]]; then
    strict_status=1
fi

if [[ "$validator_status" -eq 0 && "$summary_failed" -eq 0 ]]; then
    "$PYTHON" "$VALIDATOR_ROOT/tools/verify_proof_artifacts.py" \
        --config "$VALIDATOR_ROOT/repositories.yml" \
        --tests-root "$VALIDATOR_TESTS_ROOT" \
        --artifact-root "$artifact_root" \
        --proof-output "$proof_root/port-04-test-validation-proof.json" \
        --mode port-04-test \
        --library libzstd \
        --min-source-cases "$VALIDATOR_MIN_SOURCE_CASES" \
        --min-usage-cases "$VALIDATOR_MIN_USAGE_CASES" \
        --min-cases "$VALIDATOR_MIN_CASES" \
        --require-casts
fi

exit "$strict_status"
