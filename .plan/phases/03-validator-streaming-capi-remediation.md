# Phase 3: Streaming C API Validator Failures

## Phase Name

Validator Streaming C API Remediation

## Implement Phase ID

`impl_validator_streaming_capi_regressions`

## Preexisting Inputs

- Outputs from Phases 1 and 2
- `safe/src/compress/cstream.rs`
- `safe/src/compress/cctx.rs`
- `safe/src/compress/cctx_params.rs`
- `safe/src/compress/params.rs`
- `safe/src/ffi/compress.rs`
- `safe/src/decompress/dstream.rs`
- `safe/src/decompress/dctx.rs`
- `safe/src/ffi/decompress.rs`
- `safe/src/ffi/types.rs`
- `safe/tests/capi/zstream_driver.c`
- `safe/tests/capi/roundtrip_smoke.c`
- `safe/tests/capi/paramgrill_driver.c`
- `safe/scripts/run-capi-roundtrip.sh`
- Validator failing result/log for `streaming-c-api-smoke`, if any

## New Outputs

- Minimal C regression reproducer for the validator smoke program in `safe/tests/capi/zstream_driver.c` or a new `safe/tests/capi/validator_streaming_smoke.c`
- Updated C API runner script if a new C file is added
- Fixed C streaming implementation
- Refreshed `validator-report.md` Phase 3 section
- A pre-validator code/test commit when safe code, packaging, or regression tests changed, plus a final Phase 3 report commit

## File Changes

- Add a C regression that matches the validator smoke shape:
- create `ZSTD_CCtx`
  - initialize `ZSTD_inBuffer` and `ZSTD_outBuffer`
  - call `ZSTD_compressStream2(..., ZSTD_e_end)`
  - free the compression context
  - create `ZSTD_DCtx`
  - call `ZSTD_decompressStream`
  - assert decoded bytes and buffer positions
- Fix `safe/src/ffi/compress.rs` and `safe/src/compress/cstream.rs` for:
  - final-frame production on `ZSTD_e_end`
  - correct `input.pos` and `output.pos` updates
  - return value semantics for complete vs pending output
  - destination-too-small progress handling
- Fix `safe/src/ffi/decompress.rs` and `safe/src/decompress/dstream.rs` for:
  - one-call full-frame decode
  - empty-output and partial-output behavior
  - `ZSTD_inBuffer` / `ZSTD_outBuffer` pointer validation
  - no-forward-progress error semantics

## Implementation Details

- Capture `Phase 3 Base Commit: $(git rev-parse HEAD)` before making tracked changes and record that value in the Phase 3 report section.
- If no Phase 1 failure row is assigned to `impl_validator_streaming_capi_regressions`, update `validator-report.md` with the exact note `No streaming C API failures assigned to impl_validator_streaming_capi_regressions`, record the phase base commit and commands inspected, commit that report-only update, and yield without changing safe code.
- For true libzstd-safe streaming C API rows assigned to this phase, add regressions and fixes first, commit the code/test/package changes before running `safe/scripts/run-validator-libzstd.sh`, then update the failure table rows from `open` to `fixed` with `regression_test`, `fix_commit`, and notes before making the final report commit.
- If an assigned streaming C API row appears to be a validator bug rather than a libzstd-safe bug, do not create `skip.env` in this phase and do not mark the row `skipped_validator_bug`. Reassign that row to `impl_validator_remaining_burn_down`, keep it `open`, add `suspected_validator_bug_deferred_to_phase5:impl_validator_streaming_capi_regressions: <short reason>` to `notes`, include the supporting result/log paths in the Phase 3 report section, and make the final report commit so Phase 5 owns the proof or reclassification.
- Maintain exact C layout for `ZSTD_inBuffer`, `ZSTD_outBuffer`, and `ZSTD_EndDirective` in `safe/src/ffi/types.rs`.
- Preserve allocation/free behavior for null contexts and null buffers.
- For partial buffers, loop only while forward progress is possible; return the expected remaining hint rather than spinning.
- If the validator failure comes from compile/link behavior rather than runtime behavior, inspect installed headers and pkg-config metadata:
  - `safe/include/zstd.h`
  - `safe/pkgconfig/libzstd.pc.in`
  - `safe/cmake/`
  - `safe/scripts/build-artifacts.sh`

## Verification Phases

- `check_validator_capi_software_tester` - type: `check`; fixed `bounce_target: impl_validator_streaming_capi_regressions`; purpose: verify the exact streaming C API smoke behavior and local C ABI regressions. Commands to run:
- `cargo test --manifest-path safe/Cargo.toml --release --all-targets`
  - `bash safe/scripts/run-capi-roundtrip.sh`
  - `bash safe/scripts/run-capi-decompression.sh`
  - `cargo test --manifest-path safe/Cargo.toml --release --test compress`
  - `cargo test --manifest-path safe/Cargo.toml --release --test decompress`
  - `bash -lc 'if [ -d safe/tests/validator ]; then test -x safe/scripts/run-validator-regressions.sh; bash safe/scripts/run-validator-regressions.sh; fi'`
  - `test ! -f safe/out/validator/skip.env`
  - `bash -lc 'set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; set -e; VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --allow-remaining-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down'`
  - `rg -n 'streaming-c-api-smoke|No streaming C API failures assigned to impl_validator_streaming_capi_regressions' validator-report.md`
- `check_validator_capi_senior_tester` - type: `check`; fixed `bounce_target: impl_validator_streaming_capi_regressions`; purpose: review buffer position semantics, ownership/lifetime handling, and C ABI compatibility. Commands to run:
  - `bash -lc 'base=$(awk "/Phase 3 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'`
  - `bash safe/scripts/verify-export-parity.sh`
  - `bash safe/scripts/verify-link-compat.sh`
  - `rg -n 'streaming-c-api-smoke|No streaming C API failures assigned|ZSTD_compressStream2|ZSTD_decompressStream' validator-report.md safe/tests safe/src`

## Verification

- Run the two check phases above.
- Rebuild `.deb` packages and rerun the validator matrix.
- Confirm `streaming-c-api-smoke` passes or the report states there was no Phase 3 failure.

## Success Criteria

- Every Phase 1 row assigned to `impl_validator_streaming_capi_regressions` is fixed with a checked-in C/Rust regression and `fix_commit`, or explicitly deferred to Phase 5 as a suspected validator bug.
- If no streaming C API rows were assigned, `validator-report.md` contains `No streaming C API failures assigned to impl_validator_streaming_capi_regressions` and a report-only commit.
- `streaming-c-api-smoke` passes when this phase is marked complete, while only later-phase rows may remain open.
- C buffer position, return-hint, pointer validation, and ABI layout behavior remain compatible with upstream and are covered by the listed commands.

## Git Commit Requirement

The implementer must commit all Phase 3 work to git before yielding to the verifier phases.
