#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path


TABLE_COLUMNS = [
    "testcase_id",
    "kind",
    "client_application",
    "exit_code",
    "error",
    "result_path",
    "log_path",
    "assigned_remediation_phase",
    "remediation_status",
    "regression_test",
    "fix_commit",
    "notes",
]
HEADER_ROW = "| " + " | ".join(TABLE_COLUMNS) + " |"
SEPARATOR_ROW = "| " + " | ".join(["---"] * len(TABLE_COLUMNS)) + " |"

VALID_REMEDIATION_PHASES = {
    "impl_validator_source_cli_regressions",
    "impl_validator_streaming_capi_regressions",
    "impl_validator_libarchive_usage_regressions",
    "impl_validator_remaining_burn_down",
}
VALID_STATUSES = {"open", "fixed", "skipped_validator_bug"}
SUSPECTED_VALIDATOR_BUG_MARKER = "suspected_validator_bug_deferred_to_phase5:"
OBSERVED_FLAKE_MARKER = "observed_in_phase1_rerun_flake"


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--results-root", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--completed-phase", action="append", default=[])
    parser.add_argument("--allow-remaining-phase", action="append", default=[])
    return parser.parse_args()


def runner_status_from_env() -> int:
    raw = os.environ.get("VALIDATOR_RUNNER_STATUS", "0")
    try:
        return int(raw)
    except ValueError:
        fail(f"VALIDATOR_RUNNER_STATUS must be an integer, got {raw!r}")


def load_current_failures(results_root: Path) -> dict[str, dict[str, object]]:
    if not results_root.is_dir():
        fail(f"results root does not exist: {results_root}")

    failures: dict[str, dict[str, object]] = {}
    result_paths = sorted(path for path in results_root.glob("*.json") if path.name != "summary.json")
    if not result_paths:
        fail(f"no testcase result JSON files found under {results_root}")

    for path in result_paths:
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            fail(f"invalid JSON in {path}: {exc}")
        if not isinstance(payload, dict):
            fail(f"result JSON must be an object: {path}")
        testcase_id = payload.get("testcase_id")
        if not isinstance(testcase_id, str) or not testcase_id:
            fail(f"result JSON lacks testcase_id: {path}")
        if payload.get("status") == "failed":
            failures[testcase_id] = payload
    return failures


def parse_table_row(line: str, *, line_number: int) -> list[str]:
    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        fail(f"invalid failure table row at line {line_number}")
    cells = [cell.strip() for cell in stripped.strip("|").split("|")]
    if len(cells) != len(TABLE_COLUMNS):
        fail(
            f"failure table row at line {line_number} has {len(cells)} columns, "
            f"expected {len(TABLE_COLUMNS)}"
        )
    return cells


def load_failure_table(report: Path) -> dict[str, dict[str, str]]:
    if not report.is_file():
        fail(f"report does not exist: {report}")
    lines = report.read_text(encoding="utf-8").splitlines()

    header_index = None
    for index, line in enumerate(lines):
        if line.strip() == HEADER_ROW:
            header_index = index
            break
    if header_index is None:
        fail(f"failure table header differs from required schema: {HEADER_ROW}")
    if header_index + 1 >= len(lines) or lines[header_index + 1].strip() != SEPARATOR_ROW:
        fail(f"failure table separator differs from required schema: {SEPARATOR_ROW}")

    rows: dict[str, dict[str, str]] = {}
    for index in range(header_index + 2, len(lines)):
        line = lines[index]
        if not line.strip():
            break
        if not line.lstrip().startswith("|"):
            break
        values = parse_table_row(line, line_number=index + 1)
        row = dict(zip(TABLE_COLUMNS, values, strict=True))
        testcase_id = row["testcase_id"]
        if not testcase_id:
            fail(f"failure table row at line {index + 1} lacks testcase_id")
        if testcase_id in rows:
            fail(f"duplicate testcase_id in failure table: {testcase_id}")
        assigned = row["assigned_remediation_phase"]
        if assigned not in VALID_REMEDIATION_PHASES:
            fail(f"invalid assigned_remediation_phase for {testcase_id}: {assigned!r}")
        status = row["remediation_status"]
        if status not in VALID_STATUSES:
            fail(f"invalid remediation_status for {testcase_id}: {status!r}")
        rows[testcase_id] = row
    return rows


def validate_evidence(rows: dict[str, dict[str, str]]) -> None:
    for testcase_id, row in rows.items():
        status = row["remediation_status"]
        regression_test = row["regression_test"]
        fix_commit = row["fix_commit"]
        notes = row["notes"]
        if status == "fixed":
            if not regression_test or not fix_commit:
                fail(f"fixed row lacks regression_test or fix_commit: {testcase_id}")
        if status == "skipped_validator_bug":
            if not notes:
                fail(f"skipped_validator_bug row lacks notes: {testcase_id}")
            if "safe/out/validator/skip.env" not in notes and "safe/out/validator/tests-filtered" not in notes:
                fail(f"skipped_validator_bug row mentions no generated skip artifact: {testcase_id}")


def validate_phase_args(completed: set[str], allowed: set[str]) -> None:
    invalid_completed = sorted(completed - VALID_REMEDIATION_PHASES)
    invalid_allowed = sorted(allowed - VALID_REMEDIATION_PHASES)
    if invalid_completed:
        fail(f"invalid --completed-phase value(s): {', '.join(invalid_completed)}")
    if invalid_allowed:
        fail(f"invalid --allow-remaining-phase value(s): {', '.join(invalid_allowed)}")
    overlap = sorted(completed & allowed)
    if overlap:
        fail(f"phases cannot be both completed and allowed remaining: {', '.join(overlap)}")


def validate_reassignment_notes(rows: dict[str, dict[str, str]], completed: set[str], allowed: set[str]) -> None:
    for testcase_id, row in rows.items():
        notes = row["notes"]
        if SUSPECTED_VALIDATOR_BUG_MARKER not in notes:
            continue
        assigned = row["assigned_remediation_phase"]
        if assigned not in allowed:
            fail(f"suspected validator-bug deferral is not assigned to an allowed remaining phase: {testcase_id}")
        source = notes.split(SUSPECTED_VALIDATOR_BUG_MARKER, 1)[1].split()[0].strip(".,;")
        if source not in completed:
            fail(f"suspected validator-bug deferral source phase is not completed for {testcase_id}: {source!r}")


def main() -> int:
    args = parse_args()
    completed = set(args.completed_phase)
    allowed = set(args.allow_remaining_phase)
    validate_phase_args(completed, allowed)

    runner_status = runner_status_from_env()
    current_failures = load_current_failures(args.results_root)
    current_failed_ids = set(current_failures)
    rows = load_failure_table(args.report)
    validate_evidence(rows)
    validate_reassignment_notes(rows, completed, allowed)

    open_ids = {testcase_id for testcase_id, row in rows.items() if row["remediation_status"] == "open"}

    if not completed:
        missing_open = current_failed_ids - open_ids
        extra_open = open_ids - current_failed_ids
        unmarked_extra = sorted(
            testcase_id
            for testcase_id in extra_open
            if OBSERVED_FLAKE_MARKER not in rows[testcase_id]["notes"]
        )
        if missing_open or unmarked_extra:
            fail(
                "open failure table rows must cover current failures for initial classification, "
                "and any extra open rows must be marked as previously observed rerun flakes; "
                f"missing_open={sorted(missing_open)!r}, unmarked_extra={unmarked_extra!r}, "
                f"open={sorted(open_ids)!r}, current={sorted(current_failed_ids)!r}"
            )
        for testcase_id in sorted(open_ids):
            assigned = rows[testcase_id]["assigned_remediation_phase"]
            if assigned not in allowed:
                fail(f"open testcase is assigned outside allowed remaining phases: {testcase_id} -> {assigned}")

    for testcase_id in sorted(current_failed_ids):
        row = rows.get(testcase_id)
        if row is None:
            fail(f"currently failing testcase was not present in the Phase 1 failure table: {testcase_id}")
        assigned = row["assigned_remediation_phase"]
        if not assigned:
            fail(f"failed testcase has no assignment: {testcase_id}")
        if assigned in completed:
            fail(f"failed testcase is still assigned to a completed phase: {testcase_id} -> {assigned}")
        if assigned not in allowed:
            fail(f"failed testcase is assigned outside allowed remaining phases: {testcase_id} -> {assigned}")

    for testcase_id, row in rows.items():
        if row["assigned_remediation_phase"] in completed and row["remediation_status"] == "open":
            fail(f"row assigned to completed phase is still open: {testcase_id}")

    if not allowed:
        if current_failed_ids:
            fail(f"current failed testcases remain after all completed phases: {sorted(current_failed_ids)!r}")
        if open_ids:
            fail(f"failure table still contains open rows after all completed phases: {sorted(open_ids)!r}")

    if current_failed_ids and runner_status == 0:
        fail("failed testcases remain but VALIDATOR_RUNNER_STATUS is zero")
    if not current_failed_ids and runner_status != 0:
        fail("no failed testcase remains but VALIDATOR_RUNNER_STATUS is nonzero")

    allowed_remaining = sorted(
        testcase_id
        for testcase_id in current_failed_ids
        if rows[testcase_id]["assigned_remediation_phase"] in allowed
    )
    print("completed phases passed:", ", ".join(sorted(completed)) if completed else "(none)")
    print("allowed remaining failed testcase IDs:", ", ".join(allowed_remaining) if allowed_remaining else "(none)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
