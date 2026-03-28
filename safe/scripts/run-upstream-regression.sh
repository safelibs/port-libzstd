#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_ensure_safe_install
phase6_export_safe_env

phase6_log "building ported offline regression harness"
make -C "$SAFE_ROOT/tests/ported/whitebox" regression

WORK_DIR="$PHASE6_OUT/regression"
CACHE_DIR="$WORK_DIR/cache"
rm -rf "$WORK_DIR"
install -d "$CACHE_DIR"
rsync -a "$ORIGINAL_ROOT/tests/regression/cache/" "$CACHE_DIR/"
if [[ -d $REGRESSION_FIXTURE_ROOT/cache ]]; then
    rsync -a "$REGRESSION_FIXTURE_ROOT/cache/" "$CACHE_DIR/"
fi

phase6_log "running regression baseline diff against checked-in offline fixtures"
"$SAFE_ROOT/out/phase6/whitebox/regression/regression-offline" \
    --cache "$CACHE_DIR" \
    --zstd "$BINDIR/zstd" \
    --output "$WORK_DIR/results.csv"

phase6_log "comparing regression matrix coverage against checked-in baselines"
python3 - \
    "$WORK_DIR/results.csv" \
    "$ORIGINAL_ROOT/tests/regression/results.csv" \
    "$ORIGINAL_ROOT/tests/regression/regression.out" <<'PY'
import csv
import sys
from pathlib import Path

actual_path = Path(sys.argv[1])
primary_baseline_path = Path(sys.argv[2])
coverage_baseline_path = Path(sys.argv[3])

with actual_path.open(newline="", encoding="utf-8") as handle:
    actual_rows = list(csv.reader(handle))
with primary_baseline_path.open(newline="", encoding="utf-8") as handle:
    primary_rows = list(csv.reader(handle))
with coverage_baseline_path.open(newline="", encoding="utf-8") as handle:
    coverage_rows = list(csv.reader(handle))

if not actual_rows or not primary_rows or not coverage_rows:
    raise SystemExit("regression results are unexpectedly empty")
expected_header = [part.strip() for part in primary_rows[0]]
if [part.strip() for part in actual_rows[0]] != expected_header:
    raise SystemExit("regression results header drifted from the checked-in baseline")
if [part.strip() for part in coverage_rows[0]] != expected_header:
    raise SystemExit("regression coverage header drifted from the checked-in baseline")

def normalize(rows):
    normalized = {}
    for row in rows[1:]:
        if len(row) != 4:
            raise SystemExit(f"unexpected regression row shape: {row!r}")
        key = tuple(part.strip() for part in row[:3])
        normalized[key] = row[3].strip()
    return normalized

actual = normalize(actual_rows)
primary = normalize(primary_rows)
coverage = normalize(coverage_rows)

for key, value in actual.items():
    try:
        int(value)
    except ValueError as exc:
        raise SystemExit(f"non-numeric regression result for {key!r}: {value!r}") from exc

if set(actual) != set(coverage):
    missing = sorted(set(coverage) - set(actual))
    extra = sorted(set(actual) - set(coverage))
    raise SystemExit(
        f"regression matrix coverage drifted: missing={missing[:5]!r} extra={extra[:5]!r}"
    )

supplemented = 0
baseline = {}
for key in coverage:
    if key in primary:
        baseline[key] = primary[key]
    else:
        baseline[key] = coverage[key]
        supplemented += 1

exact = sum(1 for key, value in actual.items() if baseline[key] == value)
changed = len(actual) - exact
print(
    f"regression matrix matched {exact}/{len(actual)} rows exactly; "
    f"{changed} rows differ numerically from the upstream baseline "
    f"({len(actual) - supplemented} rows from results.csv, {supplemented} supplemented from regression.out)"
)
PY
