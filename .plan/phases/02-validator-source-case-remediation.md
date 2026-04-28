# Phase 2: Source CLI, Dictionary, Multiframe, and Corruption Failures

## Phase Name

Validator Source Case Remediation

## Implement Phase ID

`impl_validator_source_cli_regressions`

## Preexisting Inputs

- Outputs from `impl_validator_setup_initial_run`
- `safe/tests/rust/compress.rs`
- `safe/tests/rust/decompress.rs`
- `safe/tests/capi/roundtrip_smoke.c`
- `safe/tests/capi/decompress_smoke.c`
- `safe/tests/capi/frame_probe.c`
- `safe/tests/capi/invalid_dictionaries_driver.c`
- `safe/tests/capi/bigdict_driver.c`
- `safe/scripts/run-capi-roundtrip.sh`
- `safe/scripts/run-capi-decompression.sh`
- `safe/src/compress/`
- `safe/src/decompress/`
- `safe/src/dict_builder/`
- `safe/src/common/error.rs`
- `safe/src/common/frame.rs`
- `safe/src/ffi/compress.rs`
- `safe/src/ffi/decompress.rs`
- `validator-report.md` failure table from Phase 1

## New Outputs

- Minimal regression tests for each failing source case in existing Rust/C test files or in `safe/tests/validator/`
- Optional executable `safe/tests/validator/source_cases.sh` that mirrors the failing validator source command with local paths and is run by `safe/scripts/run-validator-regressions.sh`
- Updated `safe/scripts/run-validator-regressions.sh` only if the Phase 1 helper needs a narrow extension to execute the new validator-local scripts
- Fixed safe implementation files
- Refreshed `validator-report.md` Phase 2 section
- A pre-validator code/test commit when safe code, packaging, or regression tests changed, plus a final Phase 2 report commit

## File Changes

- Add tests before fixing each issue:
- `zstd-compress-decompress`: add a roundtrip/listing regression covering `zstd -q -c`, `zstd -q -dc`, and `zstd -lv` behavior against safe-built packages or local artifacts.
  - `dictionary-train-use`: add a dictionary training and dictionary compression/decompression regression covering `ZDICT_trainFromBuffer`, `ZSTD_compress_usingDict`, and `ZSTD_decompress_usingDict` or a CLI-level mirror if the failure is CLI-only.
  - `multi-frame-behavior`: add a regression for concatenated independent frames and continuous decode through one-shot and/or streaming decode.
  - `corrupted-frame-rejection`: add a malformed-frame regression that asserts failure status and stable `ZSTD_error_prefix_unknown` or the upstream-compatible error class.
- Fix narrow implementation files based on failure root cause:
  - compression frame output: `safe/src/compress/cctx.rs`, `safe/src/compress/cstream.rs`, `safe/src/compress/frame.rs`
  - dictionary training/use: `safe/src/dict_builder/zdict.rs`, `safe/src/dict_builder/cover.rs`, `safe/src/dict_builder/fastcover.rs`, `safe/src/compress/cdict.rs`, `safe/src/decompress/ddict.rs`
  - multiframe decode and frame metadata: `safe/src/decompress/frame.rs`, `safe/src/common/frame.rs`, `safe/src/decompress/dstream.rs`
  - error code/name parity: `safe/src/common/error.rs`, `safe/src/ffi/types.rs`
- Update `validator-report.md` with each failing source case, reproducer path, fixed files, and post-fix command results.

## Implementation Details

- Capture `Phase 2 Base Commit: $(git rev-parse HEAD)` before making tracked changes and record that value in the Phase 2 report section.
- If no Phase 1 failure row is assigned to `impl_validator_source_cli_regressions`, update `validator-report.md` with the exact note `No source-case failures assigned to impl_validator_source_cli_regressions`, record the phase base commit and commands inspected, commit that report-only update, and yield without changing safe code.
- For true libzstd-safe source-case rows assigned to this phase, add regressions and fixes first, commit the code/test/package changes before running `safe/scripts/run-validator-libzstd.sh`, then update the failure table rows from `open` to `fixed` with `regression_test`, `fix_commit`, and notes before making the final report commit.
- If an assigned source-case row appears to be a validator bug rather than a libzstd-safe bug, do not create `skip.env` in this phase and do not mark the row `skipped_validator_bug`. Reassign that row to `impl_validator_remaining_burn_down`, keep it `open`, add `suspected_validator_bug_deferred_to_phase5:impl_validator_source_cli_regressions: <short reason>` to `notes`, include the supporting result/log paths in the Phase 2 report section, and make the final report commit so Phase 5 owns the proof or reclassification.
- Preserve validator commands exactly; do not weaken scripts or edit validator.
- Do not add broad rewrites. Scope each fix to the function that mismatches upstream behavior.
- For dictionary failures, preserve raw dictionary and formatted dictionary behavior. Validate edge cases:
  - empty dictionary pointer with zero size
  - non-null pointer with zero size
  - malformed formatted dictionary header
  - trained dictionary ID lookup
  - dictionary-compressed payload decode with matching and missing dictionaries
- For multiframe failures, preserve concatenated frame semantics in both content-size discovery and actual decode. Avoid treating successful first-frame decode as archive completion when more valid frame bytes remain.
- For corruption failures, make the C API return an error code that `ZSTD_isError` recognizes and make the CLI return nonzero through the upstream CLI linked against safe `libzstd`.

## Verification Phases

- `check_validator_source_software_tester` - type: `check`; fixed `bounce_target: impl_validator_source_cli_regressions`; purpose: run local regressions and validator source cases after fixes. Commands to run:
- `cargo test --manifest-path safe/Cargo.toml --release --all-targets`
  - `bash safe/scripts/verify-export-parity.sh`
  - `cargo test --manifest-path safe/Cargo.toml --release --test compress`
  - `cargo test --manifest-path safe/Cargo.toml --release --test decompress`
  - `bash safe/scripts/run-capi-roundtrip.sh`
  - `bash safe/scripts/run-capi-decompression.sh`
  - `bash -lc 'if [ -d safe/tests/validator ]; then test -x safe/scripts/run-validator-regressions.sh; bash safe/scripts/run-validator-regressions.sh; fi'`
  - `test ! -f safe/out/validator/skip.env`
  - `bash -lc 'set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; set -e; VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --allow-remaining-phase impl_validator_streaming_capi_regressions --allow-remaining-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down'`
  - `rg -n 'zstd-compress-decompress|dictionary-train-use|multi-frame-behavior|corrupted-frame-rejection|No source-case failures assigned to impl_validator_source_cli_regressions' validator-report.md`
- `check_validator_source_senior_tester` - type: `check`; fixed `bounce_target: impl_validator_source_cli_regressions`; purpose: review each source-case failure-to-regression mapping and ensure no validator files were patched. Commands to run:
  - `bash -lc 'base=$(awk "/Phase 2 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'`
  - `git -C validator status --short --branch`
  - `rg -n 'Phase 2|Regression|Fix|No source-case failures' validator-report.md`
  - `rg -n 'SAFE_UPSTREAM_LIB|dlopen|dlsym|load_upstream' safe/src safe/scripts || true`

## Verification

- Run the two check phases above.
- Rebuild validator override packages before rerunning validator.
- Confirm all Phase 1 source-case failures now pass, or that no source-case failures existed and the report says so.

## Success Criteria

- Every Phase 1 row assigned to `impl_validator_source_cli_regressions` is fixed with a checked-in regression and `fix_commit`, or explicitly deferred to Phase 5 as a suspected validator bug.
- If no source-case rows were assigned, `validator-report.md` contains `No source-case failures assigned to impl_validator_source_cli_regressions` and a report-only commit.
- Source CLI, dictionary, multiframe, and corruption validator cases pass when this phase is marked complete, while only later-phase rows may remain open.
- Required Rust, C API, validator-regression, package rebuild, and phase-result checker commands have been run and recorded.

## Git Commit Requirement

The implementer must commit all Phase 2 work to git before yielding to the verifier phases.
