# Phase Name

Safe Decompression Independence and Metadata Rebase

# Implement Phase ID

`impl_safe_decompression_independence`

# Preexisting Inputs

- `.plan/goal.md`
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
- `original/libzstd-1.5.5+dfsg2/lib/zstd.h`
- `original/libzstd-1.5.5+dfsg2/lib/zdict.h`
- `original/libzstd-1.5.5+dfsg2/lib/zstd_errors.h`
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
- `original/libzstd-1.5.5+dfsg2/`

The existing plan artifacts, upstream source snapshot, ABI baselines, headers, test matrices, and harnesses are consumed in place. Do not regenerate ABI baselines from absent artifacts such as `original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5`, `original/libzstd-1.5.5+dfsg2/lib/libzstd.a`, or `original/libzstd-1.5.5+dfsg2/lib/libzstd.pc`.

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
- regenerated and committed `workflow.yaml`
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

- Rewrite the existing numbered files under `.plan/phases/` and `.plan/workflow-structure.yaml` in place to the new 8-phase order, then regenerate `workflow.yaml` from those sources.
- Commit the regenerated `workflow.yaml` together with the Phase 1 implementation work before yielding.
- Remove all decompression-side `load_upstream!` calls.
- Remove all decompression-side `SAFE_UPSTREAM_LIB` use from scripts.
- Rebase `safe/abi/export_map.toml` exactly from old ownership `2 -> 1`, `3 -> 2`, and `4 -> 3`; after this edit no export entry may still carry an `owning_phase` above 3.
- Rebase `safe/tests/upstream_test_matrix.toml` exactly from old ownership `2 -> 1`, `3 -> 2`, `4 -> 3`, `5 -> 4`, and `6 -> 5`; after this edit no upstream-suite entry may still carry an `owning_phase` above 5.
- Preserve representative required export ownerships: `ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_DCtx_reset`, and `ZSTD_getDictID_fromFrame` at Phase 1; `ZSTD_compressBound`, `ZSTD_copyCCtx`, `ZSTD_flushStream`, and `ZSTD_endStream` at Phase 2; `ZSTD_createThreadPool`, `ZSTD_freeThreadPool`, `ZSTD_CCtx_refThreadPool`, `ZSTD_estimateCStreamSize_usingCParams`, and `ZDICT_addEntropyTablesFromBuffer` at Phase 3.
- Preserve representative required upstream-suite ownerships: `tests:decodecorpus` and `tests:legacy` at Phase 1; `tests:paramgrill`, `tests:external_matchfinder`, `tests:bigdict`, `tests:invalidDictionaries`, `tests:roundTripCrash`, `tests:fullbench`, `tests:datagen`, and `tests:longmatch` at Phase 2; `tests:fuzzer`, `tests:zstreamtest`, and `tests:poolTests` at Phase 3; `debian:zstd-selftest`, `debian:build-pkg-config`, and `debian:build-cmake` at Phase 4; every preserved upstream black-box wrapper currently tagged Phase 6 at Phase 5.
- Decouple `safe/scripts/verify-baseline-contract.sh` and `safe/scripts/capture-upstream-abi.sh --check` from the missing prebuilt upstream shared object.
- Rebase the phase-ownership checks in `safe/scripts/verify-baseline-contract.sh` to the same fixed mapping while keeping the pre-Phase-6 10-dependent contract unchanged.
- Add checked-in decompression regressions for dict-backed decode, bufferless decode replay, and malformed-input behavior.

# Implementation Details

- Replace the temporary upstream-`DCtx` compatibility path in `safe/src/decompress/dctx.rs` and `safe/src/decompress/frame.rs` with native Rust dictionary-backed decompression that preserves frame format, dict-ID, and error-code behavior.
- Replace the upstream-formatted-dictionary validation path in `safe/src/ffi/decompress.rs` with native validation derived from the existing FSE/HUF dictionary parsing logic.
- Replace `UpstreamBufferlessSession` in `safe/src/ffi/decompress.rs` with a native bufferless/session state machine that drives `ZSTD_decompressBegin`, `ZSTD_decompressContinue`, `ZSTD_nextSrcSizeToDecompress`, `ZSTD_decompressBlock`, and `ZSTD_decompressStream` without loading upstream symbols.
- `safe/scripts/capture-upstream-abi.sh --check` and `safe/scripts/verify-baseline-contract.sh` must validate `safe/abi/original.exports.txt`, `safe/abi/original.soname.txt`, and `safe/abi/export_map.toml` directly and treat those files as authoritative checked-in baseline inputs.
- Keep `safe/src/ffi/legacy_shim.c` as the only decompression-side C bridge if legacy v0.5-v0.7 decode truly still requires it; document the exact boundary in `safe/docs/unsafe-audit.md`.
- Preserve upstream header identity for `safe/include/zstd.h`, `safe/include/zdict.h`, and `safe/include/zstd_errors.h`; the headers are existing inputs, not regenerated artifacts.
- Extend the decompression test surface to cover `ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_decompress_usingDict`, `ZSTD_decompress_usingDDict`, `ZSTD_decompressBegin`, `ZSTD_decompressContinue`, `ZSTD_nextSrcSizeToDecompress`, `ZSTD_decompressBlock`, legacy decode, size queries, corrupt dictionaries, and corrupt frames.

# Verification Phases

- Phase ID: `script_safe_decompression_independence`
  - Type: `check`
  - `bounce_target`: `impl_safe_decompression_independence`
  - Purpose: build the library, run decompression-side Rust/C ABI gates, and prove that decompression no longer requires an upstream shared object.
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
  - Commands: none; perform source, test, and evidence review.
- Phase ID: `check_safe_decompression_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_safe_decompression_independence`
  - Purpose: review ABI preservation, unsafe reduction, elimination of decompression-side dynamic loading, and the regenerated workflow artifact commit.
  - Commands: none; perform senior implementation, ABI, and workflow review.

# Success Criteria

- The numbered `.plan/phases/` files, `.plan/workflow-structure.yaml`, and `workflow.yaml` all describe the same linear 8-phase workflow and `workflow.yaml` has been regenerated and committed.
- Decompression APIs no longer require `SAFE_UPSTREAM_LIB`, `dlopen()`, `dlsym()`, or `load_upstream!`.
- ABI and upstream-suite ownership metadata are rebased exactly once to the new post-scaffold numbering.
- Baseline verification consumes checked-in ABI files and does not require absent upstream shared/static/pkg-config artifacts.
- Decompression regressions cover native dictionary decode, bufferless replay, legacy decode, and malformed input.
- All listed verifier commands pass or any failure is fixed before yielding.

# Git Commit Requirement

The implementer must commit the Phase 1 work, including the regenerated `workflow.yaml`, to git before yielding. That commit must exist before any verifier phase for `impl_safe_decompression_independence` runs.
