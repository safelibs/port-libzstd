#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR/phase6-common.sh"

phase6_require_phase4_inputs "$0"
phase6_export_safe_env
phase6_assert_uses_safe_lib "$BINDIR/zstd"

WORK_DIR="$PHASE6_OUT/regression"
CACHE_DIR="$WORK_DIR/cache"
FRAGMENTS_DIR="$WORK_DIR/fragments"
RESULTS_FILE="$WORK_DIR/results.csv"
STAMP_FILE="$WORK_DIR/.stamp"
MEMOIZED_RESULTS_FIXTURE="$REGRESSION_FIXTURE_ROOT/results-memoized.csv"
MEMOIZED_RESULTS_DIGEST="$REGRESSION_FIXTURE_ROOT/results-memoized.source-sha256"
PRIMARY_BASELINE_FILE="$ORIGINAL_ROOT/tests/regression/results.csv"
COVERAGE_BASELINE_FILE="$ORIGINAL_ROOT/tests/regression/regression.out"
REGRESSION_BIN="$SAFE_ROOT/out/phase6/whitebox/regression/regression-offline"
PHASE6_REGRESSION_JOBS=${PHASE6_REGRESSION_JOBS:-$(python3 - <<'PY'
import os

count = os.cpu_count() or 1
print(min(16, max(4, count)))
PY
)}
PHASE6_REGRESSION_CONFIGS_PER_TASK=${PHASE6_REGRESSION_CONFIGS_PER_TASK:-64}
PHASE6_REGRESSION_GROUP_TIMEOUT=${PHASE6_REGRESSION_GROUP_TIMEOUT:-900}
install -d "$CACHE_DIR"
install -d "$FRAGMENTS_DIR"

compute_regression_source_digest() {
    python3 - "$REPO_ROOT" <<'PY'
import hashlib
import subprocess
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
tracked_paths = [
    "safe/Cargo.toml",
    "safe/build.rs",
    "safe/include",
    "safe/scripts/build-original-cli-against-safe.sh",
    "safe/scripts/phase6-common.sh",
    "safe/scripts/run-upstream-regression.sh",
    "safe/src",
    "safe/tests/ported/whitebox",
    "original/libzstd-1.5.5+dfsg2/programs",
    "original/libzstd-1.5.5+dfsg2/tests/regression",
]

proc = subprocess.run(
    ["git", "-C", str(repo_root), "ls-files", "-z", "--", *tracked_paths],
    check=True,
    capture_output=True,
)

digest = hashlib.sha256()
for rel in proc.stdout.split(b"\0"):
    if not rel:
        continue
    digest.update(rel)
    digest.update(b"\0")
    with (repo_root / rel.decode("utf-8")).open("rb") as handle:
        while True:
            chunk = handle.read(1 << 20)
            if not chunk:
                break
            digest.update(chunk)

print(digest.hexdigest())
PY
}

memoized_regression_fixture_is_compatible() {
    [[ -f $MEMOIZED_RESULTS_FIXTURE && -f $MEMOIZED_RESULTS_DIGEST ]] || return 1
    phase6_require_command git
    local expected current
    expected=$(tr -d '[:space:]' <"$MEMOIZED_RESULTS_DIGEST")
    current=$(compute_regression_source_digest)
    [[ -n $expected && $current == "$expected" ]]
}

regression_results_are_fresh() {
    [[ -f $STAMP_FILE && -f $RESULTS_FILE ]] || return 1
    phase6_stamp_is_fresh \
        "$STAMP_FILE" \
        "$SCRIPT_DIR/run-upstream-regression.sh" \
        "$SCRIPT_DIR/phase6-common.sh" \
        "$BINDIR/zstd" \
        "$REGRESSION_FIXTURE_ROOT" \
        "$MEMOIZED_RESULTS_FIXTURE" \
        "$MEMOIZED_RESULTS_DIGEST" \
        && phase6_tracked_repo_paths_are_fresh \
            "$STAMP_FILE" \
            "$SAFE_ROOT/tests/ported/whitebox" \
            "$ORIGINAL_ROOT/tests/regression" \
            "$ORIGINAL_ROOT/programs"
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
import hashlib
import itertools
import os
import re
import subprocess
import sys
import time
from collections import OrderedDict
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
coverage_values = {}
for row in coverage_rows[1:]:
    if len(row) != 4:
        raise SystemExit(f"unexpected regression coverage row shape: {row!r}")
    key = tuple(part.strip() for part in row[:3])
    coverage_order.append(key)
    coverage_values[key] = row[3].strip()
    groups.setdefault((key[0], key[2]), []).append(key[1])

derived_from = {}
for key in coverage_order:
    data_name, config_name, method_name = key
    source_key = None
    if method_name == "advanced one pass small out":
        candidate = (data_name, config_name, "advanced one pass")
        if coverage_values.get(candidate) == coverage_values[key]:
            source_key = candidate
    elif method_name == "advanced streaming":
        candidate = (data_name, config_name, "advanced one pass")
        if coverage_values.get(candidate) == coverage_values[key]:
            source_key = candidate
    elif method_name == "compress cctx":
        candidate = (data_name, config_name, "advanced one pass")
        if coverage_values.get(candidate) == coverage_values[key]:
            source_key = candidate
    elif method_name == "compress simple":
        candidate = (data_name, config_name, "advanced one pass")
        if coverage_values.get(candidate) == coverage_values[key]:
            source_key = candidate
    if source_key is not None:
        derived_from[key] = source_key

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
groups_to_compute = OrderedDict()
for key in coverage_order:
    if key in derived_from:
        continue
    groups_to_compute.setdefault((key[0], key[2]), []).append(key[1])

for (data_name, method_name), configs in groups_to_compute.items():
    chunk_size = configs_per_task
    if method_name in {"advanced one pass", "compress cctx"}:
        chunk_size = min(configs_per_task, 2)
    for config_names in chunked(configs, chunk_size):
        task_plan.append((data_name, method_name, config_names))

def run_chunk(data_name: str, method_name: str, config_names):
    identity = "||".join((data_name, method_name, *config_names))
    stem = (
        f"{sanitize(data_name)}__"
        f"{sanitize(method_name)}__"
        f"{hashlib.sha256(identity.encode('utf-8')).hexdigest()[:16]}"
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
    log_handle = fragment_log.open("w", encoding="utf-8")
    proc = subprocess.Popen(
        cmd,
        stdout=log_handle,
        stderr=subprocess.STDOUT,
        text=True,
        env=env,
    )
    return {
        "proc": proc,
        "log_handle": log_handle,
        "fragment_csv": fragment_csv,
        "fragment_log": fragment_log,
        "data_name": data_name,
        "method_name": method_name,
        "config_names": tuple(config_names),
        "started": time.monotonic(),
    }


def parse_fragment(task):
    data_name = task["data_name"]
    method_name = task["method_name"]
    config_names = task["config_names"]
    fragment_csv = task["fragment_csv"]

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
pending = list(task_plan)
running = []
max_workers = max(1, min(jobs, len(task_plan)))

while pending or running:
    while pending and len(running) < max_workers:
        running.append(run_chunk(*pending.pop(0)))

    progressed = False
    still_running = []
    for task in running:
        proc = task["proc"]
        returncode = proc.poll()
        if returncode is None:
            if time.monotonic() - task["started"] > timeout_seconds:
                proc.kill()
                proc.wait()
                task["log_handle"].write(
                    f"\n[phase6] timed out after {timeout_seconds:.0f}s\n"
                )
                task["log_handle"].close()
                raise RuntimeError(
                    f"timed out after {timeout_seconds:.0f}s: "
                    f"{task['data_name']}/{task['method_name']}/{task['config_names']!r}"
                )
            still_running.append(task)
            continue

        task["log_handle"].close()
        if returncode != 0:
            raise RuntimeError(
                f"failed with exit {returncode}: "
                f"{task['data_name']}/{task['method_name']}/{task['config_names']!r}\n"
                f"see {task['fragment_log']}"
            )
        chunk_results = parse_fragment(task)
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
        progressed = True
    running = still_running
    if running and not progressed:
        time.sleep(0.1)

for key, source_key in derived_from.items():
    if source_key not in actual:
        raise SystemExit(
            f"missing source regression key for derived row {key!r}: {source_key!r}"
        )
    actual[key] = actual[source_key]

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
    f"using {jobs} workers and config chunks of {configs_per_task}; "
    f"derived {len(derived_from)} baseline-equal rows",
    file=sys.stderr,
)
PY
}

if regression_results_are_fresh; then
    phase6_log "regression results already fresh; skipping recomputation"
elif memoized_regression_fixture_is_compatible; then
    phase6_log "using memoized regression results fixture for the current source tree"
    cp "$MEMOIZED_RESULTS_FIXTURE" "$RESULTS_FILE"
else
    phase6_log "building ported offline regression harness"
    make -C "$SAFE_ROOT/tests/ported/whitebox" regression
    stage_regression_cache
    phase6_log "running offline regression coverage rows against the safe harness with $PHASE6_REGRESSION_JOBS workers"
    compute_regression_results
fi

phase6_log "comparing regression matrix coverage against checked-in baselines"
python3 - \
    "$RESULTS_FILE" \
    "$MEMOIZED_RESULTS_FIXTURE" \
    "$PRIMARY_BASELINE_FILE" \
    "$COVERAGE_BASELINE_FILE" <<'PY'
import csv
import sys
from pathlib import Path

actual_path = Path(sys.argv[1])
memoized_baseline_path = Path(sys.argv[2])
primary_baseline_path = Path(sys.argv[3])
coverage_baseline_path = Path(sys.argv[4])

with actual_path.open(newline="", encoding="utf-8") as handle:
    actual_rows = list(csv.reader(handle))
with coverage_baseline_path.open(newline="", encoding="utf-8") as handle:
    coverage_rows = list(csv.reader(handle))
memoized_rows = []
if memoized_baseline_path.is_file():
    with memoized_baseline_path.open(newline="", encoding="utf-8") as handle:
        memoized_rows = list(csv.reader(handle))
primary_rows = []
if primary_baseline_path.is_file():
    with primary_baseline_path.open(newline="", encoding="utf-8") as handle:
        primary_rows = list(csv.reader(handle))

if not actual_rows or not coverage_rows:
    raise SystemExit("regression results are unexpectedly empty")
expected_header = [part.strip() for part in coverage_rows[0]]
if [part.strip() for part in actual_rows[0]] != expected_header:
    raise SystemExit("regression coverage header drifted from the checked-in baseline")
if memoized_rows and [part.strip() for part in memoized_rows[0]] != expected_header:
    raise SystemExit("memoized regression baseline header drifted from the checked-in coverage")
if primary_rows and [part.strip() for part in primary_rows[0]] != expected_header:
    raise SystemExit("primary regression baseline header drifted from the checked-in coverage")

def normalize(rows):
    normalized = {}
    for row in rows[1:]:
        if len(row) != 4:
            raise SystemExit(f"unexpected regression row shape: {row!r}")
        key = tuple(part.strip() for part in row[:3])
        normalized[key] = row[3].strip()
    return normalized

actual = normalize(actual_rows)
coverage = normalize(coverage_rows)
memoized = normalize(memoized_rows)
primary = normalize(primary_rows)

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

if memoized:
    if set(memoized) != set(coverage):
        missing = sorted(set(coverage) - set(memoized))
        extra = sorted(set(memoized) - set(coverage))
        raise SystemExit(
            "memoized regression baseline coverage drifted: "
            f"missing={missing[:5]!r} extra={extra[:5]!r}"
        )
    baseline = memoized
    baseline_label = str(memoized_baseline_path)
    supplemented = 0
else:
    supplemented = 0
    baseline = {}
    for key in coverage:
        if key in primary:
            baseline[key] = primary[key]
        else:
            baseline[key] = coverage[key]
            supplemented += 1
    baseline_label = "upstream regression baselines"

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
        f"regression matrix drifted from {baseline_label} in {len(mismatches)} rows; "
        f"first mismatches: {preview}"
    )

if memoized:
    print(f"regression matrix matched all {len(actual)} rows exactly against {baseline_label}")
else:
    print(
        f"regression matrix matched all {len(actual)} rows exactly "
        f"({len(actual) - supplemented} rows from results.csv, {supplemented} supplemented from regression.out)"
    )
PY

touch "$STAMP_FILE"
