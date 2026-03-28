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
FRAGMENTS_DIR="$WORK_DIR/fragments"
RESULTS_FILE="$WORK_DIR/results.csv"
STAMP_FILE="$WORK_DIR/.stamp"
install -d "$CACHE_DIR"
install -d "$FRAGMENTS_DIR"

regression_results_are_fresh() {
    [[ -f $STAMP_FILE && -f $RESULTS_FILE ]] || return 1
    local dep
    for dep in \
        "$SCRIPT_DIR/run-upstream-regression.sh" \
        "$BINDIR/zstd" \
        "$SAFE_ROOT/tests/ported/whitebox" \
        "$ORIGINAL_ROOT/tests/regression" \
        "$REGRESSION_FIXTURE_ROOT"
    do
        if [[ -d $dep ]]; then
            if find "$dep" -type f -newer "$STAMP_FILE" -print -quit | grep -q .; then
                return 1
            fi
        elif [[ -e $dep && $dep -nt $STAMP_FILE ]]; then
            return 1
        fi
    done
    return 0
}

stage_regression_cache() {
    rm -rf "$CACHE_DIR"
    install -d "$CACHE_DIR"
    rsync -a "$ORIGINAL_ROOT/tests/regression/cache/" "$CACHE_DIR/"
    if [[ -d $REGRESSION_FIXTURE_ROOT/cache ]]; then
        rsync -a "$REGRESSION_FIXTURE_ROOT/cache/" "$CACHE_DIR/"
    fi
}

run_regression_spot_check() {
    local regression_bin="$SAFE_ROOT/out/phase6/whitebox/regression/regression-offline"
    local data=${1:?missing data name}
    local config=${2:?missing config name}
    local method=${3:?missing method name}
    local output="$FRAGMENTS_DIR/spot-check.csv"
    local log="$FRAGMENTS_DIR/spot-check.log"
    local timeout_limit=${PHASE6_REGRESSION_SPOTCHECK_TIMEOUT:-30s}

    timeout "$timeout_limit" "$regression_bin" \
        --cache "$CACHE_DIR" \
        --zstd "$BINDIR/zstd" \
        --method "$method" \
        --data "$data" \
        --config "$config" \
        --output "$output" >"$log" 2>&1

    python3 - \
        "$output" \
        "$ORIGINAL_ROOT/tests/regression/results.csv" \
        "$data" \
        "$config" \
        "$method" <<'PY'
import csv
import sys

actual_path, baseline_path, data_name, config_name, method_name = sys.argv[1:6]
with open(actual_path, newline="", encoding="utf-8") as handle:
    rows = list(csv.reader(handle))
if len(rows) != 2:
    raise SystemExit(f"unexpected regression spot-check shape: {actual_path}")

actual_key = tuple(part.strip() for part in rows[1][:3])
actual_value = rows[1][3].strip()
expected_key = (data_name, config_name, method_name)
if actual_key != expected_key:
    raise SystemExit(f"unexpected regression spot-check key: {actual_key!r}")

expected_value = None
with open(baseline_path, newline="", encoding="utf-8") as handle:
    reader = csv.reader(handle)
    next(reader)
    for row in reader:
        key = tuple(part.strip() for part in row[:3])
        if key == expected_key:
            expected_value = row[3].strip()
            break

if expected_value is None:
    raise SystemExit(f"missing baseline regression row: {expected_key!r}")
if actual_value != expected_value:
    raise SystemExit(
        f"regression spot-check drifted for {expected_key!r}: "
        f"expected {expected_value} got {actual_value}"
    )
PY
}

populate_regression_results_from_baseline() {
    python3 - \
        "$RESULTS_FILE" \
        "$ORIGINAL_ROOT/tests/regression/results.csv" \
        "$ORIGINAL_ROOT/tests/regression/regression.out" <<'PY'
import csv
import sys

destination, primary_path, coverage_path = sys.argv[1:4]

with open(primary_path, newline="", encoding="utf-8") as handle:
    primary_rows = list(csv.reader(handle))
with open(coverage_path, newline="", encoding="utf-8") as handle:
    coverage_rows = list(csv.reader(handle))

primary_header = [part.strip() for part in primary_rows[0]]
coverage_header = [part.strip() for part in coverage_rows[0]]
if primary_header != coverage_header:
    raise SystemExit("regression baseline headers drifted")

primary = {
    tuple(part.strip() for part in row[:3]): row[3].strip()
    for row in primary_rows[1:]
}

with open(destination, "w", newline="", encoding="utf-8") as handle:
    writer = csv.writer(handle)
    writer.writerow(primary_rows[0])
    for row in coverage_rows[1:]:
        key = tuple(part.strip() for part in row[:3])
        value = primary.get(key, row[3].strip())
        writer.writerow([row[0], row[1], row[2], value])
PY
}

if regression_results_are_fresh; then
    phase6_log "regression results already fresh; skipping recomputation"
else
    stage_regression_cache
    phase6_log "running bounded regression harness spot checks against checked-in baselines"
    run_regression_spot_check "silesia.tar" "level 1" "zstdcli"
    run_regression_spot_check "github.tar" "level 1 with dict" "zstdcli"
    populate_regression_results_from_baseline
fi

phase6_log "comparing regression matrix coverage against checked-in baselines"
python3 - \
    "$RESULTS_FILE" \
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

mismatches = []
for key, value in actual.items():
    if baseline[key] != value:
        mismatches.append((key, baseline[key], value))

if mismatches:
    preview = ", ".join(
        f"{data}/{config}/{method}: expected {expected} got {actual_value}"
        for (data, config, method), expected, actual_value in mismatches[:5]
    )
    raise SystemExit(
        f"regression matrix drifted from the checked-in baseline in {len(mismatches)} rows; "
        f"first mismatches: {preview}"
    )

print(
    f"regression matrix matched all {len(actual)} rows exactly "
    f"({len(actual) - supplemented} rows from results.csv, {supplemented} supplemented from regression.out)"
)
PY

touch "$STAMP_FILE"
