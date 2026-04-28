# Phase 4: Libarchive Usage Validator Failures

## Phase Name

Validator Libarchive Usage Remediation

## Implement Phase ID

`impl_validator_libarchive_usage_regressions`

## Preexisting Inputs

- Outputs from Phases 1-3
- Validator libarchive usage failure logs under `safe/out/validator/artifacts/port-04-test/logs/libzstd/`
- `safe/docker/dependents/entrypoint.sh`
- `safe/tests/dependents/dependent_matrix.toml`
- `safe/tests/dependents/src/libarchive_probe.c`
- `safe/scripts/run-dependent-matrix.sh`
- `safe/scripts/build-dependent-image.sh`
- `safe/src/compress/`
- `safe/src/decompress/`
- `safe/src/common/frame.rs`
- `safe/src/ffi/compress.rs`
- `safe/src/ffi/decompress.rs`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`

## New Outputs

- Minimal libarchive usage reproducer for each unique usage failure class
- Optional executable `safe/tests/validator/libarchive_usage_cases.sh` with failing validator case mirrors, run by `safe/scripts/run-validator-regressions.sh`
- Updated dependent runtime `test_libarchive` if the validator uncovers a durable downstream scenario not already covered locally
- Fixed safe implementation or package metadata
- Refreshed `validator-report.md` Phase 4 section
- A pre-validator code/test/package commit when safe code, packaging, or regression tests changed, plus a final Phase 4 report commit

## File Changes

- Add a reproducer before each fix. Prefer an existing dependent runtime test when the issue is truly downstream behavior:
- `safe/docker/dependents/entrypoint.sh` `test_libarchive`
  - `safe/tests/dependents/src/libarchive_probe.c`
  - fixtures under `safe/tests/dependents/fixtures/` if needed
- If the failure is specific to validator shell behavior, add a narrower script under `safe/tests/validator/` rather than expanding the dependent matrix unnecessarily.
- Fix likely implementation areas:
  - zstd frame generation and final block handling in `safe/src/compress/frame.rs`, `safe/src/compress/cstream.rs`, and `safe/src/ffi/compress.rs`
  - continuous stream decode and concatenated frame handling in `safe/src/decompress/frame.rs`, `safe/src/decompress/dstream.rs`, and `safe/src/ffi/decompress.rs`
  - frame size and content size probes in `safe/src/common/frame.rs`
  - package library/header metadata in `safe/pkgconfig/libzstd.pc.in`, `safe/cmake/`, `safe/debian/`, or `safe/scripts/build-artifacts.sh`

## Implementation Details

- Capture `Phase 4 Base Commit: $(git rev-parse HEAD)` before making tracked changes and record that value in the Phase 4 report section.
- If no Phase 1 failure row is assigned to `impl_validator_libarchive_usage_regressions`, update `validator-report.md` with the exact note `No libarchive usage failures assigned to impl_validator_libarchive_usage_regressions`, record the phase base commit and commands inspected, commit that report-only update, and yield without changing safe code.
- For true libzstd-safe libarchive usage rows assigned to this phase, add regressions and fixes first, commit the code/test/package changes before running `safe/scripts/run-validator-libzstd.sh`, then update the failure table rows from `open` to `fixed` with `regression_test`, `fix_commit`, and notes before making the final report commit.
- If an assigned libarchive usage row appears to be a validator bug rather than a libzstd-safe bug, do not create `skip.env` in this phase and do not mark the row `skipped_validator_bug`. Reassign that row to `impl_validator_remaining_burn_down`, keep it `open`, add `suspected_validator_bug_deferred_to_phase5:impl_validator_libarchive_usage_regressions: <short reason>` to `notes`, include the supporting result/log paths in the Phase 4 report section, and make the final report commit so Phase 5 owns the proof or reclassification.
- Group the 80 usage cases by failing symptom and fix one root cause per change set:
  - archive creation failures
  - archive listing failures
  - extraction data mismatch
  - stdin/stdout streaming failures
  - metadata or mode preservation failures
  - large-file or multi-member failures
  - dynamic linker resolving the wrong `libzstd`
- Preserve validator's use of installed Debian packages. Do not run libarchive usage tests by pointing `LD_LIBRARY_PATH` at build-tree artifacts unless the local reproducer is explicitly marked as pre-package triage.
- When a failure is due to `bsdtar --zstd` producing archives that safe cannot read, fix compression output. When a failure is due to safe `zstd` output that `bsdtar` cannot read, fix frame format compatibility. When a failure occurs only after package install, fix packaging or install layout.

## Verification Phases

- `check_validator_usage_software_tester` - type: `check`; fixed `bounce_target: impl_validator_libarchive_usage_regressions`; purpose: run dependent/libarchive-focused regressions and confirm usage-case deltas. Commands to run:
- `cargo test --manifest-path safe/Cargo.toml --release --all-targets`
  - `bash safe/scripts/verify-export-parity.sh`
  - `bash -lc 'if [ -d safe/tests/validator ]; then test -x safe/scripts/run-validator-regressions.sh; bash safe/scripts/run-validator-regressions.sh; fi'`
  - `test ! -f safe/out/validator/skip.env`
  - `bash -lc 'set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; set -e; VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down'`
  - `bash safe/scripts/build-dependent-image.sh`
  - `bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps libarchive`
  - `bash test-original.sh --runtime-only --apps libarchive`
  - `rg -n 'usage-libarchive-tools|No libarchive usage failures assigned to impl_validator_libarchive_usage_regressions' validator-report.md`
- `check_validator_usage_senior_tester` - type: `check`; fixed `bounce_target: impl_validator_libarchive_usage_regressions`; purpose: review package install behavior, dynamic linkage, streaming semantics, and metadata handling. Commands to run:
  - `bash -lc 'base=$(awk "/Phase 4 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'`
  - `bash safe/scripts/verify-install-layout.sh`
  - `bash safe/scripts/verify-install-layout.sh --debian`
  - `bash safe/scripts/verify-link-compat.sh`
  - `git -C validator status --short --branch`

## Verification

- Run the two check phases above.
- Rebuild packages and dependent image after every safe or packaging fix.
- Rerun the validator matrix and confirm the affected usage cases pass.

## Success Criteria

- Every Phase 1 row assigned to `impl_validator_libarchive_usage_regressions` is fixed with a checked-in validator/dependent regression and `fix_commit`, or explicitly deferred to Phase 5 as a suspected validator bug.
- If no libarchive usage rows were assigned, `validator-report.md` contains `No libarchive usage failures assigned to impl_validator_libarchive_usage_regressions` and a report-only commit.
- Affected `libarchive-tools` and `bsdtar --zstd` usage cases pass when this phase is marked complete, while only Phase 5 rows may remain open.
- Package install layout, dynamic linkage, dependent-image libarchive runtime coverage, and safe frame compatibility are verified with the listed commands.

## Git Commit Requirement

The implementer must commit all Phase 4 work to git before yielding to the verifier phases.
