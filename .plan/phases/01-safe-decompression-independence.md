# Phase Name

Safe Decompression Independence and Metadata Rebase

# Implement Phase ID

`impl_safe_decompression_independence`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/plan.md`
- `.plan/phases/01-safe-decompression-independence.md`
- `.plan/phases/02-safe-compression-core.md`
- `.plan/phases/03-safe-advanced-abi-completion.md`
- `.plan/phases/04-safe-packaging-install.md`
- `.plan/phases/05-upstream-release-gates.md`
- `.plan/phases/06-dependent-image-matrix.md`
- `.plan/phases/07-compat-regressions-and-fixes.md`
- `.plan/phases/08-final-release-burn-down.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- `safe/Cargo.toml`
- `safe/build.rs`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `safe/abi/original.exports.txt`
- `safe/abi/original.soname.txt`
- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
- `safe/src/decompress/dctx.rs`
- `safe/src/decompress/ddict.rs`
- `safe/src/decompress/dstream.rs`
- `safe/src/decompress/frame.rs`
- `safe/src/decompress/huf.rs`
- `safe/src/decompress/fse.rs`
- `safe/src/decompress/legacy.rs`
- `safe/src/ffi/compress.rs`
- `safe/src/ffi/decompress.rs`
- `safe/src/ffi/legacy_shim.c`
- `safe/tests/rust/decompress.rs`
- `safe/tests/capi/decompress_smoke.c`
- `safe/tests/capi/frame_probe.c`
- `safe/tests/capi/legacy_decode.c`
- `safe/scripts/run-capi-decompression.sh`
- `safe/scripts/capture-upstream-abi.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `safe/scripts/verify-export-parity.sh`
- `safe/docs/unsafe-audit.md`
- `original/libzstd-1.5.5+dfsg2/lib/common/*`
- `original/libzstd-1.5.5+dfsg2/lib/decompress/*`
- `original/libzstd-1.5.5+dfsg2/lib/legacy/*`
- `original/libzstd-1.5.5+dfsg2/tests/golden-decompression/*`
- `original/libzstd-1.5.5+dfsg2/tests/golden-dictionaries/*`

# New Outputs

- rewritten `.plan/phases/01-safe-decompression-independence.md`
- rewritten `.plan/phases/02-safe-compression-core.md`
- rewritten `.plan/phases/03-safe-advanced-abi-completion.md`
- rewritten `.plan/phases/04-safe-packaging-install.md`
- rewritten `.plan/phases/05-upstream-release-gates.md`
- rewritten `.plan/phases/06-dependent-image-matrix.md`
- rewritten `.plan/phases/07-compat-regressions-and-fixes.md`
- rewritten `.plan/phases/08-final-release-burn-down.md`
- rewritten `.plan/workflow-structure.yaml`
- regenerated `workflow.yaml`
- rewritten `safe/abi/export_map.toml`
- rewritten `safe/tests/upstream_test_matrix.toml`
- rewritten `safe/src/decompress/dctx.rs`
- rewritten `safe/src/decompress/ddict.rs`
- rewritten `safe/src/decompress/dstream.rs`
- rewritten `safe/src/decompress/frame.rs`
- rewritten `safe/src/decompress/huf.rs`
- rewritten `safe/src/decompress/fse.rs`
- rewritten `safe/src/ffi/decompress.rs`
- rewritten `safe/tests/rust/decompress.rs`
- rewritten `safe/tests/capi/decompress_smoke.c`
- rewritten `safe/tests/capi/frame_probe.c`
- rewritten `safe/tests/capi/legacy_decode.c`
- rewritten `safe/scripts/run-capi-decompression.sh`
- rewritten `safe/scripts/capture-upstream-abi.sh`
- rewritten `safe/scripts/verify-baseline-contract.sh`
- rewritten `safe/docs/unsafe-audit.md`

# File Changes

- Remove all decompression-side `load_upstream!` calls.
- Remove all decompression-side `SAFE_UPSTREAM_LIB` use from scripts.
- Rewrite `.plan/workflow-structure.yaml` and the existing numbered `.plan/phases/*.md` files in place to the new 8-phase order, then regenerate and commit `workflow.yaml` from those rewritten sources.
- Rebase `safe/abi/export_map.toml` exactly from old ownership `2 -> 1`, `3 -> 2`, and `4 -> 3`; no export entry may keep `owning_phase` greater than 3 after this edit.
- Verify representative export results: `ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_DCtx_reset`, and `ZSTD_getDictID_fromFrame` at Phase 1; `ZSTD_compressBound`, `ZSTD_copyCCtx`, `ZSTD_flushStream`, and `ZSTD_endStream` at Phase 2; `ZSTD_createThreadPool`, `ZSTD_freeThreadPool`, `ZSTD_CCtx_refThreadPool`, `ZSTD_estimateCStreamSize_usingCParams`, and `ZDICT_addEntropyTablesFromBuffer` at Phase 3.
- Rebase `safe/tests/upstream_test_matrix.toml` exactly from old ownership `2 -> 1`, `3 -> 2`, `4 -> 3`, `5 -> 4`, and `6 -> 5`; no upstream-suite entry may keep `owning_phase` greater than 5 after this edit.
- Verify representative upstream-suite results: `tests:decodecorpus` and `tests:legacy` at Phase 1; `tests:paramgrill`, `tests:external_matchfinder`, `tests:bigdict`, `tests:invalidDictionaries`, `tests:roundTripCrash`, `tests:fullbench`, `tests:datagen`, and `tests:longmatch` at Phase 2; `tests:fuzzer`, `tests:zstreamtest`, and `tests:poolTests` at Phase 3; `debian:zstd-selftest`, `debian:build-pkg-config`, and `debian:build-cmake` at Phase 4; and every preserved upstream black-box wrapper currently tagged Phase 6 at Phase 5.
- Decouple the baseline ABI checker from absent upstream shared objects so `verify-baseline-contract.sh` and `capture-upstream-abi.sh --check` validate the checked-in ABI baseline files directly.
- Rebase the phase-ownership checks in `safe/scripts/verify-baseline-contract.sh` to the same fixed mapping while keeping the pre-Phase-6 10-dependent contract unchanged.
- Add checked-in decompression regressions for dictionary-backed decode, bufferless decode replay, and malformed-input behavior.

# Implementation Details

- Preserve `safe/Cargo.toml` crate outputs and features: `cdylib`, `staticlib`, `rlib`, `build-shared-default`, `build-static-default`, `variant-mt`, and `variant-nomt`.
- Consume the existing upstream source snapshot and checked-in ABI baselines in place. Do not require or regenerate absent upstream artifacts such as `original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5`, `original/libzstd-1.5.5+dfsg2/lib/libzstd.a`, or `original/libzstd-1.5.5+dfsg2/lib/libzstd.pc`.
- Replace the temporary upstream-`DCtx` compatibility path in `safe/src/decompress/dctx.rs` and `safe/src/decompress/frame.rs` with native Rust dictionary-backed decompression that preserves frame format, dict-ID, and error-code behavior.
- Replace the upstream-formatted-dictionary validation path in `safe/src/ffi/decompress.rs` with native validation derived from the existing FSE/HUF dictionary parsing logic.
- Replace `UpstreamBufferlessSession` in `safe/src/ffi/decompress.rs` with a native bufferless/session state machine that drives `ZSTD_decompressBegin`, `ZSTD_decompressContinue`, `ZSTD_nextSrcSizeToDecompress`, `ZSTD_decompressBlock`, and `ZSTD_decompressStream` without loading upstream symbols.
- Keep `safe/src/ffi/legacy_shim.c` as the only decompression-side C bridge if legacy v0.5-v0.7 decode truly still requires it; document the exact boundary in `safe/docs/unsafe-audit.md`.
- Extend decompression tests to cover `ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_decompress_usingDict`, `ZSTD_decompress_usingDDict`, bufferless APIs, legacy decode, size-query behavior, corrupt dictionaries, corrupt frames, and malformed-input error parity.
- Treat `original/libzstd-1.5.5+dfsg2/` as a consumed baseline, not a normal patch target. Default to zero edits under `original/`.

# Verification Phases

- Phase ID: `script_safe_decompression_independence`
- Type: `check`
- `bounce_target`: `impl_safe_decompression_independence`
- Purpose: build the library, run decompression-side Rust/C ABI gates, and prove that decompression no longer requires an upstream shared object.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - `cargo test --manifest-path safe/Cargo.toml --release --test decompress`
  - `bash safe/scripts/run-capi-decompression.sh`
  - `bash safe/scripts/verify-export-parity.sh`
  - `bash safe/scripts/verify-baseline-contract.sh`
  - `rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym' safe/src/decompress safe/src/ffi/decompress.rs safe/scripts/run-capi-decompression.sh`

- Phase ID: `check_safe_decompression_software_tester`
- Type: `check`
- `bounce_target`: `impl_safe_decompression_independence`
- Purpose: review decompression semantics, new regression coverage, and negative-path handling.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

- Phase ID: `check_safe_decompression_senior_tester`
- Type: `check`
- `bounce_target`: `impl_safe_decompression_independence`
- Purpose: review ABI preservation, unsafe reduction, and elimination of decompression-side dynamic loading.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

# Success Criteria

- Decompression APIs run from native safe-side code without `SAFE_UPSTREAM_LIB`, `load_upstream!`, `dlopen()`, or `dlsym()`.
- Baseline ABI verification passes using only `safe/abi/original.exports.txt`, `safe/abi/original.soname.txt`, and `safe/abi/export_map.toml`.
- Export and upstream-suite ownership metadata matches the fixed post-scaffold rebase and is enforced by `safe/scripts/verify-baseline-contract.sh`.
- The decompression regressions are checked in and the script verifier commands pass.

# Git Commit Requirement

The implementer must commit all Phase 1 work to git before yielding. That commit must exist before `script_safe_decompression_independence`, `check_safe_decompression_software_tester`, and `check_safe_decompression_senior_tester` run.
