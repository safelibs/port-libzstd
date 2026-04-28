# Phase 5: Remaining Failures, Validator-Bug Triage, and Report Consolidation

## Phase Name

Remaining Validator Burn Down

## Implement Phase ID

`impl_validator_remaining_burn_down`

## Preexisting Inputs

- Outputs from Phases 1-4
- Full validator results under `safe/out/validator/artifacts/`
- `validator-report.md`
- All local regression tests added in earlier phases

## New Outputs

- Fixes for any residual validator failures not covered by Phases 2-4
- Generated filtered validator test root under `safe/out/validator/tests-filtered/` only if a validator bug is clearly proven
- Generated skip metadata under `safe/out/validator/skip.env` only if a validator bug is clearly proven
- Finalized failure classification in `validator-report.md`
- A pre-validator code/test/package commit when safe code, packaging, or regression tests changed, plus a final Phase 5 report commit

## File Changes

- Update only the safe-side files needed for true residual compatibility failures.
- If a validator bug is proven:
  - leave `validator/` unchanged
  - document the bug in `validator-report.md`
  - generate any filtered test root under `safe/out/validator/tests-filtered/`
  - generate `safe/out/validator/skip.env` with adjusted proof thresholds and skipped testcase IDs
  - ensure only the invalid check is omitted from the filtered run
- Update `safe/scripts/run-validator-libzstd.sh` only if needed to support generated skip roots or better report generation.

## Implementation Details

- Capture `Phase 5 Base Commit: $(git rev-parse HEAD)` before making tracked changes and record that value in the Phase 5 report section.
- If no Phase 1 failure row is assigned to `impl_validator_remaining_burn_down`, run the strict full validator helper, then update `validator-report.md` with the exact note `No remaining failures assigned to impl_validator_remaining_burn_down`, the phase base commit, commands run, and result paths before making one report-only commit.
- If residual rows are assigned to this phase, add regressions and fixes first for true libzstd-safe failures, commit code/test/package changes before running `safe/scripts/run-validator-libzstd.sh`, then update the failure table rows from `open` to `fixed` with `regression_test`, `fix_commit`, and notes before making the final report commit.
- Re-parse all result JSON files after the Phase 4 run. The burn-down phase owns:
  - validator infrastructure problems
  - package override install problems
  - proof generation problems
  - residual mixed failures that involve more than one prior category
  - suspected validator-bug rows reassigned from Phases 2-4
  - validator bug documentation
- A validator-bug finding must include:
  - exact testcase ID
  - validator commit
  - log path
  - why Ubuntu original-package behavior or validator script logic is inconsistent with the stated test
  - why modifying libzstd-safe would be incorrect
  - exact generated skip mechanics, if used
- For each suspected validator-bug row handed off from an earlier phase, either prove it as a validator bug and apply the generated skip path below, or reclassify it as a true libzstd-safe residual failure, add a checked-in regression, fix it in safe code or package metadata, and mark it `fixed` with the Phase 5 fix commit.
- When generating a skip, remove any stale `safe/out/validator/tests-filtered/`, then create this generated dual layout:
  - copy `validator/tests/libzstd/` to `safe/out/validator/tests-filtered/libzstd/`
  - create `safe/out/validator/tests-filtered/tests/`
  - copy `validator/tests/libzstd/` to `safe/out/validator/tests-filtered/tests/libzstd/`
  - remove only the invalid testcase entries from both generated `testcases.yml` files
  - preserve the copied Dockerfile, entrypoint, fixtures, and case scripts in both generated copies
  - leave `validator/tests/_shared/` consumed from the unmodified validator checkout; do not copy or edit `_shared`
- Resolve `PYTHON` exactly as in `safe/scripts/run-validator-libzstd.sh`, preferring `safe/out/validator/venv/bin/python` when it exists and otherwise requiring `python3 -c 'import yaml'` to pass. Validate the generated skip root before the filtered run with `"$PYTHON" validator/tools/testcases.py --config validator/repositories.yml --tests-root "$PWD/safe/out/validator/tests-filtered" --library libzstd --check --min-source-cases "$VALIDATOR_MIN_SOURCE_CASES" --min-usage-cases "$VALIDATOR_MIN_USAGE_CASES" --min-cases "$VALIDATOR_MIN_CASES"`. Run the filtered matrix with the unmodified `--config "$PWD/validator/repositories.yml"`, `--tests-root "$PWD/safe/out/validator/tests-filtered"`, `--mode port-04-test`, `--override-deb-root "$PWD/safe/out/validator/override-debs"`, `--port-deb-lock "$PWD/safe/out/validator/artifacts/proof/port-04-test-debs-lock.json"`, and `--library libzstd`.
- `safe/out/validator/skip.env` must define:
  - `VALIDATOR_TESTS_ROOT` as the absolute path to `safe/out/validator/tests-filtered`
  - `VALIDATOR_SKIPPED_CASES` as a space-separated list of skipped testcase IDs
  - `VALIDATOR_CANONICAL_SOURCE_CASES=5`, `VALIDATOR_CANONICAL_USAGE_CASES=80`, and `VALIDATOR_CANONICAL_CASES=85`
  - `VALIDATOR_SKIPPED_SOURCE_CASES`, `VALIDATOR_SKIPPED_USAGE_CASES`, and `VALIDATOR_SKIPPED_TOTAL_CASES` as the documented skipped counts
  - `VALIDATOR_MIN_SOURCE_CASES`, `VALIDATOR_MIN_USAGE_CASES`, and `VALIDATOR_MIN_CASES` as 5, 80, and 85 minus the documented skipped source, usage, and total counts
- A true libzstd-safe residual failure must still get a checked-in regression before the fix.

## Verification Phases

- `check_validator_remaining_software_tester` - type: `check`; fixed `bounce_target: impl_validator_remaining_burn_down`; purpose: verify no unclassified failures remain and all fixed failures have regressions. Commands to run:
- `bash safe/scripts/run-validator-regressions.sh`
  - `bash -lc 'set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; set -e; VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --completed-phase impl_validator_remaining_burn_down; test "$status" -eq 0'`
  - `python3 -m json.tool safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json >/dev/null`
  - `rg -n 'Unclassified|Remaining|Validator Bug|Skip|No remaining failures assigned to impl_validator_remaining_burn_down' validator-report.md`
  - `cargo test --manifest-path safe/Cargo.toml --release --all-targets`
  - `bash safe/scripts/verify-export-parity.sh`
- `check_validator_remaining_senior_tester` - type: `check`; fixed `bounce_target: impl_validator_remaining_burn_down`; purpose: review any validator-bug claims and ensure skips are minimal and generated-only. Commands to run:
  - `bash -lc 'base=$(awk "/Phase 5 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'`
  - `test -z "$(git -C validator status --porcelain --untracked-files=no)"`
  - `bash -lc 'if [ -f safe/out/validator/skip.env ]; then py=python3; if [ -x safe/out/validator/venv/bin/python ]; then py="$PWD/safe/out/validator/venv/bin/python"; else python3 -c "import yaml"; fi; set -a; . safe/out/validator/skip.env; set +a; test -n "${VALIDATOR_TESTS_ROOT:-}"; test -d "$VALIDATOR_TESTS_ROOT/libzstd"; test -d "$VALIDATOR_TESTS_ROOT/tests/libzstd"; "$py" validator/tools/testcases.py --config validator/repositories.yml --tests-root "$VALIDATOR_TESTS_ROOT" --library libzstd --check --min-source-cases "$VALIDATOR_MIN_SOURCE_CASES" --min-usage-cases "$VALIDATOR_MIN_USAGE_CASES" --min-cases "$VALIDATOR_MIN_CASES"; fi'`
  - `find safe/out/validator -maxdepth 3 -type d | sort | sed -n '1,120p'`
  - `rg -n 'skip|skipped|validator bug|justification|No remaining failures assigned to impl_validator_remaining_burn_down' validator-report.md`

## Verification

- Run the two check phases above.
- Confirm `validator-report.md` has no unclassified failures.
- Confirm any skip is generated under `safe/out/validator/` and justified in the report.

## Success Criteria

- No unclassified or open failures remain after Phase 5 all-completed checking.
- Every `fixed` row has a non-empty `regression_test` and `fix_commit`.
- Every `skipped_validator_bug` row has non-empty notes naming generated skip artifacts and a clear justification.
- Any skip root and threshold reduction are generated only under `safe/out/validator/`, minimal, validated, and recorded with `VALIDATOR_SKIPPED_SOURCE_CASES`, `VALIDATOR_SKIPPED_USAGE_CASES`, and `VALIDATOR_SKIPPED_TOTAL_CASES`.
- `validator/` remains unmodified except normal clone/fetch/switch/fast-forward operations.

## Git Commit Requirement

The implementer must commit all Phase 5 work to git before yielding to the verifier phases.
