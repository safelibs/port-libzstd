# Phase 6: Final Clean Validator Run and Release Gate

## Phase Name

Final Validator Evidence And Review

## Implement Phase ID

`impl_validator_final_clean_run`

## Preexisting Inputs

- Outputs from Phases 1-5
- All checked-in regressions and fixes
- `safe/scripts/run-validator-libzstd.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/scripts/build-dependent-image.sh`
- `validator-report.md`

## New Outputs

- Final clean validator artifacts under `safe/out/validator/artifacts/`
- Final proof JSON under `safe/out/validator/artifacts/proof/port-04-test-validation-proof.json`
- Final `validator-report.md` with complete run summary
- A final git commit for Phase 6

## File Changes

- Update `validator-report.md` final sections:
- validator commit
  - safe commits included
  - commands executed
  - package filenames and hashes
  - canonical 85-case summary, plus adjusted executed-case summary if a generated validator-bug skip exists
  - all failures found across the workflow
  - fixes applied
  - regressions added
  - skips, if any, with justification
  - final validator proof path
  - final `safe/scripts/run-full-suite.sh` result
- Do not make new code changes in this phase unless a final verifier finds a blocking issue. Any final verifier bounce must go only to this phase's fixed `bounce_target`, `impl_validator_final_clean_run`; the final implementor must document the failure, make the smallest necessary correction or report-only update in this phase, commit it, and rerun the final evidence commands.

## Implementation Details

- Capture `Phase 6 Base Commit: $(git rev-parse HEAD)` before the final evidence run and record that value in the final report section.
- Rerun the full validator matrix from a clean artifact root to avoid stale pass/fail data:

```bash
rm -rf safe/out/validator/artifacts
set +e
bash safe/scripts/run-validator-libzstd.sh
validator_status=$?
set -e
VALIDATOR_RUNNER_STATUS=$validator_status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --completed-phase impl_validator_remaining_burn_down
test "$validator_status" -eq 0
```

If `safe/out/validator/skip.env` exists, `safe/scripts/run-validator-libzstd.sh` must print the skipped testcase IDs, use the filtered tests root recorded in that file, and use the adjusted proof thresholds recorded there. Without `skip.env`, it must use the unmodified `validator/tests` root and the canonical 5/80/85 thresholds.

- Then run local full release verification:

```bash
bash safe/scripts/run-validator-regressions.sh
bash safe/scripts/build-dependent-image.sh
bash safe/scripts/run-full-suite.sh
```

- Without a validator-bug skip, the final validator summary must show 85 executed cases, 5 source cases, 80 usage cases, and 0 failures. With a documented validator-bug skip, the final summary must show the adjusted executed counts, 0 failures, and `validator-report.md` must state the canonical 85-case inventory and the omitted testcase IDs.
- The final report must be enough for a checker to reproduce the exact run from clone/pull through proof generation.

## Verification Phases

- `check_validator_final_software_tester` - type: `check`; fixed `bounce_target: impl_validator_final_clean_run`; purpose: independently rerun the complete validator command and local release gates. Commands to run:
- `bash -lc 'set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; set -e; VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --completed-phase impl_validator_remaining_burn_down; test "$status" -eq 0'`
  - `bash safe/scripts/run-validator-regressions.sh`
  - `bash safe/scripts/build-dependent-image.sh`
  - `bash safe/scripts/run-full-suite.sh`
  - `python3 -m json.tool safe/out/validator/artifacts/proof/port-04-test-debs-lock.json >/dev/null`
  - `python3 -m json.tool safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json >/dev/null`
  - `python3 -m json.tool safe/out/validator/artifacts/proof/port-04-test-validation-proof.json >/dev/null`
- `check_validator_final_senior_tester` - type: `check`; fixed `bounce_target: impl_validator_final_clean_run`; purpose: final architectural review of evidence, commits, validator isolation, and report completeness. Commands to run:
  - `bash -lc 'base=$(awk "/Phase 6 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'`
  - `git status --short`
  - `git log --oneline --decorate -8`
  - `git -C validator rev-parse HEAD`
  - `git -C validator status --short --branch`
  - `VALIDATOR_RUNNER_STATUS=0 python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --completed-phase impl_validator_remaining_burn_down`
  - `bash -lc 'if [ -f safe/out/validator/skip.env ]; then py=python3; if [ -x safe/out/validator/venv/bin/python ]; then py="$PWD/safe/out/validator/venv/bin/python"; else python3 -c "import yaml"; fi; set -a; . safe/out/validator/skip.env; set +a; test -d "$VALIDATOR_TESTS_ROOT/libzstd"; test -d "$VALIDATOR_TESTS_ROOT/tests/libzstd"; "$py" validator/tools/testcases.py --config validator/repositories.yml --tests-root "$VALIDATOR_TESTS_ROOT" --library libzstd --check --min-source-cases "$VALIDATOR_MIN_SOURCE_CASES" --min-usage-cases "$VALIDATOR_MIN_USAGE_CASES" --min-cases "$VALIDATOR_MIN_CASES"; fi'`
  - `rg -n 'Final Run|Validator Commit|Checks Executed|Failures Found|Fixes Applied|Skips|Proof' validator-report.md`
  - `test ! -e .gitmodules || ! rg -n 'validator' .gitmodules`

## Verification

- Run the two check phases above.
- Confirm all work is committed and the only untracked files are generated/ignored artifacts or unrelated preexisting bytecode.

## Success Criteria

- The final validator run exits zero and produces valid port lock, summary, and proof JSON artifacts.
- Without skips, the final summary records 85 total cases, 5 source cases, 80 usage cases, and 0 failures; with documented skips, adjusted counts and skipped IDs are recorded while the canonical 85-case inventory remains in the report.
- `validator-report.md` summarizes validator commit, safe commits, commands, package hashes, failures, fixes, regressions, skips, proof path, and release-gate result.
- `safe/scripts/run-validator-regressions.sh`, `safe/scripts/build-dependent-image.sh`, and `safe/scripts/run-full-suite.sh` pass after the final validator run.
- All work is committed; any remaining untracked files are generated/ignored artifacts or unrelated preexisting bytecode.

## Git Commit Requirement

The implementer must commit all Phase 6 work to git before yielding to the verifier phases.
