# Phase 1: Validator Checkout, Override Packaging, and Initial Run

## Phase Name

Validator Setup And Initial Evidence

## Implement Phase ID

`impl_validator_setup_initial_run`

## Preexisting Inputs

- Goal text in `.plan/goal.md`
- Existing safe implementation under `safe/`
- Existing upstream source snapshot under `original/libzstd-1.5.5+dfsg2/`
- Existing package scripts:
  - `safe/scripts/build-artifacts.sh`
  - `safe/scripts/build-original-cli-against-safe.sh`
  - `safe/scripts/build-deb.sh`
- Existing package metadata:
  - `safe/debian/control`
  - `safe/debian/rules`
  - `safe/debian/*.install`
- Existing release gates:
  - `safe/scripts/run-full-suite.sh`
  - `safe/scripts/verify-export-parity.sh`
  - `safe/scripts/verify-install-layout.sh`
- Existing `.gitignore`
- No required preexisting `validator/` checkout

## New Outputs

- `validator/` checkout at the latest fast-forwarded `main` commit, or the existing checkout updated in place
- generated `safe/out/validator/venv/` with PyYAML if host Python lacks `yaml`
- generated `safe/out/validator/override-debs/libzstd/*.deb`
- generated `safe/out/validator/artifacts/proof/port-04-test-debs-lock.json`
- generated `safe/out/validator/artifacts/port-04-test/results/libzstd/*.json`
- generated `safe/out/validator/artifacts/port-04-test/logs/libzstd/*.log`
- generated `safe/out/validator/artifacts/port-04-test/casts/libzstd/*.cast`
- generated `safe/out/validator/artifacts/proof/port-04-test-validation-proof.json` only if the initial run passes
- tracked `validator-report.md`
- tracked `.gitignore` update for `validator/`
- tracked `safe/scripts/run-validator-libzstd.sh` helper
- tracked `safe/scripts/check-validator-phase-results.py` helper
- tracked `safe/scripts/run-validator-regressions.sh` helper
- a git commit containing `.gitignore`, `validator-report.md`, and the tracked helper scripts created in this phase

## File Changes

- Add `validator/` to `.gitignore`.
- Create `validator-report.md` with sections:
  - `Phase 1 Base Commit: <40-hex commit captured before edits>`
  - validator URL and commit
  - local safe commit
  - Python setup path
  - package filenames and SHA256 hashes
  - generated port lock path, repository string, tag ref, commit, release tag, package architectures, sizes, and SHA256 hashes
  - exact commands run
  - matrix inventory counts: 5 source cases, 80 usage cases, 85 total cases
  - initial summary counts from `safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json`
  - failing testcase table with the exact schema `testcase_id`, `kind`, `client_application`, `exit_code`, `error`, `result_path`, `log_path`, `assigned_remediation_phase`, `remediation_status`, `regression_test`, `fix_commit`, and `notes`
  - skip list, initially empty
- Create `safe/scripts/run-validator-libzstd.sh` as the canonical rerun command. The script must:
  - use `#!/usr/bin/env bash` and `set -euo pipefail`
  - locate `REPO_ROOT`, `SAFE_ROOT`, and `VALIDATOR_ROOT`
  - create `safe/out/validator/venv` only when needed for `import yaml`
  - export `PYTHON`
  - build safe artifacts and `.deb` packages
  - run the Debian package build as `env -u DEB_BUILD_PROFILES bash safe/scripts/build-deb.sh`, so a caller's `DEB_BUILD_PROFILES` cannot redirect the build to a non-default package root
  - source and validate `safe/out/deb/default/metadata.env` before staging packages, requiring `BUILD_TAG=default`, empty `PROFILES`, `PACKAGE_DIR=$SAFE_ROOT/out/deb/default/packages`, and `INSTALL_ROOT=$SAFE_ROOT/out/deb/default/stage-root`
  - stage exactly `libzstd1_*.deb`, `libzstd-dev_*.deb`, and `zstd_*.deb`
  - clear stale generated files under `safe/out/validator/artifacts/port-04-test/` and stale generated proof/lock files under `safe/out/validator/artifacts/proof/` before each run
  - generate `safe/out/validator/artifacts/proof/port-04-test-debs-lock.json` from the staged `.deb` files before running the matrix
  - run `make unit` and `make check-testcases` in `validator`
  - run the full libzstd validator matrix with `--mode port-04-test`, `--port-deb-lock`, and `--record-casts`; do not implement partial `--source-only` or `--usage-only` modes
  - capture the validator matrix exit status so failed-result JSON remains available for report classification
  - read `safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json` after every matrix run and treat any nonzero `failed` count as a helper failure even when `validator/test.sh` exits zero
  - run proof generation only when the matrix command exits zero and the summary reports zero failed cases
  - default to `validator/tests` and proof thresholds 5 source, 80 usage, and 85 total cases
  - if generated `safe/out/validator/skip.env` exists, source it only after confirming `validator-report.md` mentions every `VALIDATOR_SKIPPED_CASES` entry, then require `VALIDATOR_TESTS_ROOT/libzstd/` and `VALIDATOR_TESTS_ROOT/tests/libzstd/` to exist, run `"$PYTHON" validator/tools/testcases.py --config "$PWD/validator/repositories.yml" --tests-root "$VALIDATOR_TESTS_ROOT" --library libzstd --check --min-source-cases "$VALIDATOR_MIN_SOURCE_CASES" --min-usage-cases "$VALIDATOR_MIN_USAGE_CASES" --min-cases "$VALIDATOR_MIN_CASES"`, and use `VALIDATOR_TESTS_ROOT`, `VALIDATOR_MIN_SOURCE_CASES`, `VALIDATOR_MIN_USAGE_CASES`, and `VALIDATOR_MIN_CASES` for the filtered final matrix and proof run
  - never delete `safe/out/validator/skip.env` or `safe/out/validator/tests-filtered/` during normal invocation
- Create `safe/scripts/run-validator-regressions.sh` as the stable gate for local reproductions derived from validator failures. The script must:
  - use `#!/usr/bin/env bash` and `set -euo pipefail`
  - locate and export `REPO_ROOT`, `SAFE_ROOT`, and `VALIDATOR_ARTIFACT_ROOT="$REPO_ROOT/safe/out/validator/artifacts"`
  - exit zero with a clear message when `safe/tests/validator/` does not exist
  - discover executable `*.sh` files directly under `safe/tests/validator/` in lexical order
  - fail if `safe/tests/validator/` exists but no executable `*.sh` tests are present
  - run each discovered script from `REPO_ROOT`
  - pass through any extra command-line arguments to each regression script
- Create `safe/scripts/check-validator-phase-results.py` for Phase 1-5 and final checkers. The script must verify initial classification, completed work, and final table closure without blocking on failures assigned to current or later phases. The script must:
  - use `#!/usr/bin/env python3`
  - accept `--results-root`, `--report`, repeated `--completed-phase`, and repeated `--allow-remaining-phase`
  - accept zero `--completed-phase` arguments for Phase 1 initial-classification verification; in that mode, the set of table rows with `remediation_status` set to `open` must exactly equal the set of currently failing testcase IDs, and every open row must be assigned to one of the provided `--allow-remaining-phase` values
  - read `VALIDATOR_RUNNER_STATUS` from the environment, defaulting to `0` when absent
  - parse result JSON files under `safe/out/validator/artifacts/port-04-test/results/libzstd/`, excluding `summary.json`
  - parse the strict `validator-report.md` failure table and map each `testcase_id` to exactly one `assigned_remediation_phase`, `remediation_status`, `regression_test`, `fix_commit`, and `notes`
  - fail if the failure table header or separator row differs from the schema required by this plan
  - fail if any table row has an invalid `assigned_remediation_phase`, invalid `remediation_status`, or duplicate `testcase_id`
  - fail if any currently failing testcase ID was not present in the Phase 1 failure table
  - fail when any failed testcase has no assignment
  - fail when any failed testcase is assigned to a completed phase
  - fail when any failed testcase is assigned to a phase outside `--allow-remaining-phase`
  - fail when any row assigned to a completed phase still has `remediation_status` set to `open`
  - allow an `open` row that was originally assigned to a completed phase only after the report reassigns it to an `--allow-remaining-phase` value; when the reassignment is for suspected validator-bug triage, require `notes` to contain `suspected_validator_bug_deferred_to_phase5:<source_phase_id>`
  - fail when a `fixed` row lacks `regression_test` or `fix_commit`
  - fail when a `skipped_validator_bug` row lacks `notes` or mentions no generated skip artifact
  - fail when failed testcases remain but `VALIDATOR_RUNNER_STATUS` is zero, because the strict helper must exit nonzero while validator findings remain
  - fail when no failed testcase remains but `VALIDATOR_RUNNER_STATUS` is nonzero, because proof and helper infrastructure must pass on a clean matrix
  - accept zero `--allow-remaining-phase` arguments for Phase 5 and Phase 6 all-completed verification; in that mode, fail if any current result JSON is failing, if any table row still has `remediation_status` set to `open`, or if any completed row is missing its required regression, fix commit, or generated skip-artifact evidence
  - print passed completed phases and any allowed remaining failed testcase IDs

## Implementation Details

### Workflow Artifact Contract

- This plan remains the authoritative validator plan. Existing `.plan/workflow-structure.yaml`, `.plan/phases/*.md`, and `workflow.yaml` may describe prior safe-port phases and must be replaced in place from this validator plan when the final workflow is materialized. Do not load prompts or checks from stale phase files.
- The validator repository reference discovered during planning is `1319bb0374ef66428a42dd71e49553c6d057feaf` on 2026-04-28; record the actual checked-out commit after clone or fast-forward in `validator-report.md`.

- Clone or update validator:
```bash
if [ -d validator/.git ]; then
  test -z "$(git -C validator status --porcelain --untracked-files=no)"
  test "$(git -C validator remote get-url origin)" = "https://github.com/safelibs/validator"
  git -C validator fetch origin main
  current_branch=$(git -C validator branch --show-current)
  if [ "$current_branch" != "main" ]; then
    git -C validator switch main || git -C validator switch --track origin/main
  fi
  git -C validator merge --ff-only origin/main
else
  git clone https://github.com/safelibs/validator validator
fi
```

- Set up Python:

```bash
if python3 -c 'import yaml' >/dev/null 2>&1; then
  PYTHON=python3
else
  mkdir -p safe/out/validator
  python3 -m venv safe/out/validator/venv
  safe/out/validator/venv/bin/python -m pip install --upgrade pip PyYAML
  PYTHON="$PWD/safe/out/validator/venv/bin/python"
fi
export PYTHON

# Initial Phase 1 setup only. Do not embed this cleanup in
# safe/scripts/run-validator-libzstd.sh; later final runs must preserve a
# Phase 5-generated skip.env and filtered test root.
rm -f safe/out/validator/skip.env
rm -rf safe/out/validator/tests-filtered
rm -rf safe/out/validator/artifacts
```

- Build and stage override packages:

```bash
bash safe/scripts/build-artifacts.sh --release
bash safe/scripts/build-original-cli-against-safe.sh
env -u DEB_BUILD_PROFILES bash safe/scripts/build-deb.sh
metadata="$PWD/safe/out/deb/default/metadata.env"
package_dir="$PWD/safe/out/deb/default/packages"
test -f "$metadata"
(
  set -euo pipefail
  source "$metadata"
  test "${BUILD_TAG:-}" = default
  test -z "${PROFILES:-}"
  test "${PACKAGE_DIR:-}" = "$package_dir"
  test "${INSTALL_ROOT:-}" = "$PWD/safe/out/deb/default/stage-root"
)
rm -rf safe/out/validator/override-debs/libzstd
mkdir -p safe/out/validator/override-debs/libzstd
shopt -s nullglob
for pkg in libzstd1 libzstd-dev zstd; do
  matches=("$package_dir"/${pkg}_*.deb)
  if [ "${#matches[@]}" -ne 1 ]; then
    printf 'expected exactly one %s .deb, found %d\n' "$pkg" "${#matches[@]}" >&2
    exit 1
  fi
  cp -a "${matches[0]}" safe/out/validator/override-debs/libzstd/
done
shopt -u nullglob
if find safe/out/validator/override-debs/libzstd -maxdepth 1 -type f -name 'libzstd1-udeb_*' | grep -q .; then
  printf 'validator override leaf must not include libzstd1-udeb\n' >&2
  exit 1
fi
```

- Generate the local port lock from the staged override packages:

```bash
mkdir -p safe/out/validator/artifacts/proof
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
```

- Run validator tooling:

```bash
PYTHON="$PYTHON" make -C validator unit
PYTHON="$PYTHON" make -C validator check-testcases
validator_status=0
PYTHON="$PYTHON" bash validator/test.sh \
  --config "$PWD/validator/repositories.yml" \
  --tests-root "$PWD/validator/tests" \
  --artifact-root "$PWD/safe/out/validator/artifacts" \
  --mode port-04-test \
  --library libzstd \
  --override-deb-root "$PWD/safe/out/validator/override-debs" \
  --port-deb-lock "$PWD/safe/out/validator/artifacts/proof/port-04-test-debs-lock.json" \
  --record-casts || validator_status=$?
```

- Read the mode-specific summary and convert validator findings into a strict helper failure:

```bash
summary_path="$PWD/safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json"
summary_failed=1
if [ -f "$summary_path" ]; then
  summary_failed=$("$PYTHON" -c 'import json,sys; print(int(json.load(open(sys.argv[1]))["failed"]))' "$summary_path")
fi
strict_status="$validator_status"
if [ "$summary_failed" -ne 0 ]; then
  strict_status=1
fi
```

- If the matrix command succeeds and the summary has zero failures, generate proof:

```bash
if [ "$validator_status" -eq 0 ] && [ "$summary_failed" -eq 0 ]; then
  "$PYTHON" validator/tools/verify_proof_artifacts.py \
    --config "$PWD/validator/repositories.yml" \
    --tests-root "$PWD/validator/tests" \
    --artifact-root "$PWD/safe/out/validator/artifacts" \
    --proof-output "$PWD/safe/out/validator/artifacts/proof/port-04-test-validation-proof.json" \
    --mode port-04-test \
    --library libzstd \
    --min-source-cases 5 \
    --min-usage-cases 80 \
    --min-cases 85 \
    --require-casts
fi
```

- `safe/scripts/run-validator-libzstd.sh` must exit nonzero if `strict_status` is nonzero or if proof generation fails. Phase 1 evidence collection may invoke the helper under `set +e` so `validator-report.md` can still be written and committed when the initial validator run exposes failures.

- If the matrix command fails or the summary reports failed cases, parse result JSON files and classify failures into:
  - source CLI/dictionary/multiframe/corruption failures -> Phase 2
  - streaming C API failures -> Phase 3
  - libarchive usage failures -> Phase 4
  - packaging, validator infrastructure, or mixed residual failures -> Phase 5
- End Phase 1 by committing `.gitignore`, `validator-report.md`, `safe/scripts/run-validator-libzstd.sh`, `safe/scripts/check-validator-phase-results.py`, and `safe/scripts/run-validator-regressions.sh`; the commit must exist even when the initial validator matrix fails.

## Verification Phases

- `check_validator_setup_software_tester` - type: `check`; fixed `bounce_target: impl_validator_setup_initial_run`; purpose: verify that validator setup, package override staging, initial matrix execution, and failure classification are reproducible. Commands to run:
- `git -C validator rev-parse HEAD`
  - `test -x safe/scripts/run-validator-libzstd.sh`
  - `test -x safe/scripts/check-validator-phase-results.py`
  - `test -x safe/scripts/run-validator-regressions.sh`
  - `rg -n 'env -u DEB_BUILD_PROFILES[[:space:]]+bash[[:space:]]+.*build-deb.sh' safe/scripts/run-validator-libzstd.sh`
  - `bash -lc 'set +e; DEB_BUILD_PROFILES=noudeb bash safe/scripts/run-validator-libzstd.sh; status=$?; set -e; VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --allow-remaining-phase impl_validator_source_cli_regressions --allow-remaining-phase impl_validator_streaming_capi_regressions --allow-remaining-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down'`
  - `bash -lc 'source safe/out/deb/default/metadata.env; test "${BUILD_TAG:-}" = default; test -z "${PROFILES:-}"; test "${PACKAGE_DIR:-}" = "$PWD/safe/out/deb/default/packages"; test "${INSTALL_ROOT:-}" = "$PWD/safe/out/deb/default/stage-root"'`
  - `test -f validator-report.md`
  - `find safe/out/validator/override-debs/libzstd -maxdepth 1 -type f -name '*.deb' -printf '%f\n' | sort`
  - `test -f safe/out/validator/artifacts/proof/port-04-test-debs-lock.json`
  - `python3 -m json.tool safe/out/validator/artifacts/proof/port-04-test-debs-lock.json >/dev/null`
  - `test -f safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json`
  - `python3 -m json.tool safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json >/dev/null`
  - `rg -n 'Validator Commit|Initial Run|Failure Classification|assigned_remediation_phase|libzstd' validator-report.md`
- `check_validator_setup_senior_tester` - type: `check`; fixed `bounce_target: impl_validator_setup_initial_run`; purpose: review artifact flow and ensure the validator checkout was not modified or committed. Commands to run:
  - `bash -lc 'base=$(awk "/Phase 1 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'`
  - `git status --short`
  - `git check-ignore -v validator`
  - `git -C validator status --short --branch`
  - `rg -n 'validator/|safe/out/validator|override-debs|proof' .gitignore validator-report.md`
  - `test ! -e .gitmodules || ! rg -n 'validator' .gitmodules`

## Verification

- Run the two check phases above.
- Confirm `validator-report.md` includes the validator commit and initial run summary.
- Confirm `validator/` is ignored and not staged for commit.
- Confirm the phase commit exists with `git log --oneline -1`.

## Success Criteria

- `validator/` exists, is on the fast-forwarded validator `main`, and is ignored by the parent repository.
- The known validator reference commit `1319bb0374ef66428a42dd71e49553c6d057feaf` is preserved as context and the actual checked-out commit is recorded in `validator-report.md`.
- The helper scripts are tracked, executable, rerunnable, and use `port-04-test` with the generated local port lock.
- The override package leaf contains exactly `libzstd1`, `libzstd-dev`, and `zstd` `.deb` files from the verified default package root, never `libzstd1-udeb`.
- The local port lock and summary JSON are valid and match the staged package artifacts.
- `validator-report.md` contains the validator commit, initial summary, exact failure table schema, skip list, and assigned remediation phases for every current failure.
- The generated workflow artifacts replace stale prior safe-port workflow descriptions in place, including `workflow.yaml` when the final workflow is materialized from these phase files.

## Git Commit Requirement

The implementer must commit all Phase 1 work to git before yielding to the verifier phases.
