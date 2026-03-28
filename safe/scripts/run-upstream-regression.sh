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
PRIMARY_BASELINE_FILE="$ORIGINAL_ROOT/tests/regression/results.csv"
COVERAGE_BASELINE_FILE="$ORIGINAL_ROOT/tests/regression/regression.out"
REGRESSION_BIN="$SAFE_ROOT/out/phase6/whitebox/regression/regression-offline"
PHASE6_REGRESSION_JOBS=${PHASE6_REGRESSION_JOBS:-$(python3 - <<'PY'
import os

count = os.cpu_count() or 1
print(min(32, max(4, count)))
PY
)}
PHASE6_REGRESSION_CONFIGS_PER_TASK=${PHASE6_REGRESSION_CONFIGS_PER_TASK:-4}
PHASE6_REGRESSION_GROUP_TIMEOUT=${PHASE6_REGRESSION_GROUP_TIMEOUT:-900}
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
    rm -rf "$FRAGMENTS_DIR"
    install -d "$FRAGMENTS_DIR"
}

compute_regression_results() {
    python3 - \
        "$REGRESSION_BIN" \
        "$CACHE_DIR" \
        "$BINDIR/zstd" \
        "$FRAGMENTS_DIR" \
        "$COVERAGE_BASELINE_FILE" \
        "$RESULTS_FILE" \
        "$PHASE6_REGRESSION_JOBS" \
        "$PHASE6_REGRESSION_GROUP_TIMEOUT" \
        "$PHASE6_REGRESSION_CONFIGS_PER_TASK" <<'PY'
import csv
import itertools
import os
import re
import subprocess
import sys
from collections import OrderedDict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

regression_bin = Path(sys.argv[1])
cache_dir = Path(sys.argv[2])
zstd_bin = Path(sys.argv[3])
fragments_dir = Path(sys.argv[4])
coverage_path = Path(sys.argv[5])
results_path = Path(sys.argv[6])
jobs = int(sys.argv[7])
timeout_seconds = float(sys.argv[8])
configs_per_task = int(sys.argv[9])

with coverage_path.open(newline="", encoding="utf-8") as handle:
    coverage_rows = list(csv.reader(handle))
if not coverage_rows:
    raise SystemExit("regression coverage baseline is empty")

expected_header = [part.strip() for part in coverage_rows[0]]
coverage_order = []
groups = OrderedDict()
for row in coverage_rows[1:]:
    if len(row) != 4:
        raise SystemExit(f"unexpected regression coverage row shape: {row!r}")
    key = tuple(part.strip() for part in row[:3])
    coverage_order.append(key)
    groups.setdefault((key[0], key[2]), []).append(key[1])

def sanitize(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", name)

def chunked(items, size):
    iterator = iter(items)
    while True:
        chunk = list(itertools.islice(iterator, size))
        if not chunk:
            return
        yield chunk

task_plan = []
for (data_name, method_name), configs in groups.items():
    for config_names in chunked(configs, configs_per_task):
        task_plan.append((data_name, method_name, config_names))

def run_chunk(data_name: str, method_name: str, config_names):
    stem = (
        f"{sanitize(data_name)}__"
        f"{sanitize('__'.join(config_names))}__"
        f"{sanitize(method_name)}"
    )
    fragment_csv = fragments_dir / f"{stem}.csv"
    fragment_log = fragments_dir / f"{stem}.log"
    cmd = [
        str(regression_bin),
        "--cache",
        str(cache_dir),
        "--zstd",
        str(zstd_bin),
        "--data",
        data_name,
        "--method",
        method_name,
        "--output",
        str(fragment_csv),
    ]
    env = dict(os.environ)
    env["PHASE6_REGRESSION_CONFIGS"] = ",".join(config_names)
    try:
        completed = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            check=False,
            env=env,
        )
    except subprocess.TimeoutExpired as exc:
        fragment_log.write_text(
            (exc.stdout or "") + (exc.stderr or ""),
            encoding="utf-8",
        )
        raise RuntimeError(
            f"timed out after {timeout_seconds:.0f}s: "
            f"{data_name}/{method_name}/{config_names!r}"
        ) from exc

    fragment_log.write_text(completed.stdout + completed.stderr, encoding="utf-8")
    if completed.returncode != 0:
        raise RuntimeError(
            f"failed with exit {completed.returncode}: "
            f"{data_name}/{method_name}/{config_names!r}\n"
            f"see {fragment_log}"
        )

    with fragment_csv.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle))
    if not rows:
        raise RuntimeError(f"empty regression fragment: {fragment_csv}")
    if [part.strip() for part in rows[0]] != expected_header:
        raise RuntimeError(f"regression fragment header drifted: {fragment_csv}")

    results = {}
    expected_keys = {(data_name, config_name, method_name) for config_name in config_names}
    for row in rows[1:]:
        if len(row) != 4:
            raise RuntimeError(f"unexpected regression row shape in {fragment_csv}: {row!r}")
        key = tuple(part.strip() for part in row[:3])
        if key not in expected_keys:
            raise RuntimeError(
                f"unexpected regression fragment key {key!r} in {fragment_csv}"
            )
        value = row[3].strip()
        try:
            int(value)
        except ValueError as exc:
            raise RuntimeError(
                f"non-numeric regression result for {key!r}: {value!r}"
            ) from exc
        if key in results:
            raise RuntimeError(f"duplicate regression row for {key!r}")
        results[key] = value
    if set(results) != expected_keys:
        missing = sorted(expected_keys - set(results))
        extra = sorted(set(results) - expected_keys)
        raise RuntimeError(
            f"regression fragment coverage drifted for {data_name}/{method_name}: "
            f"missing={missing!r} extra={extra!r}"
        )
    return results

actual = {}
total_rows = len(coverage_order)
completed_rows = 0
with ThreadPoolExecutor(max_workers=jobs) as executor:
    futures = {
        executor.submit(run_chunk, data_name, method_name, config_names): (
            data_name,
            method_name,
            tuple(config_names),
        )
        for data_name, method_name, config_names in task_plan
    }
    for future in as_completed(futures):
        chunk_results = future.result()
        for key, value in chunk_results.items():
            if key in actual:
                raise SystemExit(f"duplicate regression key across runs: {key!r}")
            actual[key] = value
        completed_rows += len(chunk_results)
        if completed_rows % 25 == 0 or completed_rows == total_rows:
            print(
                f"[phase6] regression rows {completed_rows}/{total_rows}",
                file=sys.stderr,
            )

expected_keys = set(coverage_order)
actual_keys = set(actual)
if actual_keys != expected_keys:
    missing = sorted(expected_keys - actual_keys)
    extra = sorted(actual_keys - expected_keys)
    raise SystemExit(
        "regression matrix coverage drifted: "
        f"missing={missing[:5]!r} extra={extra[:5]!r}"
    )

tmp_path = results_path.with_suffix(".csv.tmp")
with tmp_path.open("w", newline="", encoding="utf-8") as handle:
    writer = csv.writer(handle)
    writer.writerow(coverage_rows[0])
    for data_name, config_name, method_name in coverage_order:
        value = actual[(data_name, config_name, method_name)]
        writer.writerow([data_name, config_name, method_name, value])
tmp_path.replace(results_path)
print(
    f"[phase6] computed regression results for {len(actual)} rows "
    f"using {jobs} workers and config chunks of {configs_per_task}",
    file=sys.stderr,
)
PY
}

if regression_results_are_fresh; then
    phase6_log "regression results already fresh; skipping recomputation"
else
    stage_regression_cache
    phase6_log "running offline regression coverage rows against the safe harness with $PHASE6_REGRESSION_JOBS workers"
    compute_regression_results
fi

phase6_log "comparing regression matrix coverage against checked-in baselines"
python3 - \
    "$RESULTS_FILE" \
    "$PRIMARY_BASELINE_FILE" \
    "$COVERAGE_BASELINE_FILE" <<'PY'
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
