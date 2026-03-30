# Safe Decompression Independence and Metadata Rebase

## Phase Name
Safe Decompression Independence and Metadata Rebase

## Implement Phase ID
`impl_safe_decompression_independence`

## Preexisting Inputs
- `.plan/goal.md`
- `workflow.yaml`
- `.plan/workflow-structure.yaml`
- `.plan/phases/01-safe-decompression-independence.md`
- `.plan/phases/02-safe-compression-core.md`
- `.plan/phases/03-safe-advanced-abi-completion.md`
- `.plan/phases/04-safe-packaging-install.md`
- `.plan/phases/05-upstream-release-gates.md`
- `.plan/phases/06-dependent-image-matrix.md`
- `.plan/phases/07-compat-regressions-and-fixes.md`
- `.plan/phases/08-final-release-burn-down.md`
- `safe/Cargo.toml`
- `safe/build.rs`
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
- `safe/tests/rust/decompress.rs`
- `safe/tests/capi/decompress_smoke.c`
- `safe/tests/capi/frame_probe.c`
- `safe/tests/capi/legacy_decode.c`
- `safe/scripts/run-capi-decompression.sh`
- `safe/scripts/verify-export-parity.sh`
- `safe/scripts/capture-upstream-abi.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `safe/docs/unsafe-audit.md`
- `original/libzstd-1.5.5+dfsg2/lib/common/`
- `original/libzstd-1.5.5+dfsg2/lib/decompress/`
- `original/libzstd-1.5.5+dfsg2/lib/legacy/`
- `original/libzstd-1.5.5+dfsg2/tests/golden-decompression/`
- `original/libzstd-1.5.5+dfsg2/tests/golden-dictionaries/`

## New Outputs
- rewritten `workflow.yaml`
- rewritten `.plan/workflow-structure.yaml`
- rewritten `.plan/phases/01-safe-decompression-independence.md`
- rewritten `.plan/phases/02-safe-compression-core.md`
- rewritten `.plan/phases/03-safe-advanced-abi-completion.md`
- rewritten `.plan/phases/04-safe-packaging-install.md`
- rewritten `.plan/phases/05-upstream-release-gates.md`
- rewritten `.plan/phases/06-dependent-image-matrix.md`
- rewritten `.plan/phases/07-compat-regressions-and-fixes.md`
- rewritten `.plan/phases/08-final-release-burn-down.md`
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

## File Changes
- Remove all decompression-side `load_upstream!` calls.
- Remove all decompression-side `SAFE_UPSTREAM_LIB` use from scripts.
- Keep `safe/Cargo.toml` crate types and the existing feature matrix intact; add or adjust Rust dependencies only if needed to finish the safe-only implementation.
- Rebase `safe/abi/export_map.toml` exactly from old ownership `2 -> 1`, `3 -> 2`, `4 -> 3`; after this edit no export entry may still carry an `owning_phase` above 3.
- Rebase `safe/tests/upstream_test_matrix.toml` exactly from old ownership `2 -> 1`, `3 -> 2`, `4 -> 3`, `5 -> 4`, `6 -> 5`; after this edit no upstream-suite entry may still carry an `owning_phase` above 5.
- Make Phase 1 the only global `owning_phase` renumbering step; later phases may extend status, prerequisites, helper paths, and release-gate metadata in place, but they must not re-shift the rebased preexisting ownership table.
- Decouple the baseline ABI checker from the missing upstream shared object so `verify-baseline-contract.sh` and `capture-upstream-abi.sh --check` validate the checked-in ABI baseline files directly instead of trying to inspect `original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5`.
- Rebase the phase-ownership checks in `safe/scripts/verify-baseline-contract.sh` to that same fixed mapping while keeping the pre-Phase-6 10-dependent contract unchanged.
- Preserve the representative fixed rebase results from the workflow contract in both metadata files and the checker: `ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_DCtx_reset`, and `ZSTD_getDictID_fromFrame` at Phase 1; `ZSTD_compressBound`, `ZSTD_copyCCtx`, `ZSTD_flushStream`, and `ZSTD_endStream` at Phase 2; `ZSTD_createThreadPool`, `ZSTD_freeThreadPool`, `ZSTD_CCtx_refThreadPool`, `ZSTD_estimateCStreamSize_usingCParams`, and `ZDICT_addEntropyTablesFromBuffer` at Phase 3; `tests:decodecorpus` and `tests:legacy` at Phase 1; `tests:paramgrill`, `tests:external_matchfinder`, `tests:bigdict`, `tests:invalidDictionaries`, `tests:roundTripCrash`, `tests:fullbench`, `tests:datagen`, and `tests:longmatch` at Phase 2; `tests:fuzzer`, `tests:zstreamtest`, and `tests:poolTests` at Phase 3; `debian:zstd-selftest`, `debian:build-pkg-config`, and `debian:build-cmake` at Phase 4; and the preserved upstream black-box wrappers, examples, and seekable suites at Phase 5.
- Add checked-in decompression regressions for dict-backed decode, bufferless decode replay, and malformed-input behavior.
- Preserve the consume-existing-artifacts contract for `workflow.yaml`, `.plan/workflow-structure.yaml`, and the numbered `.plan/phases/*.md` files by rewriting those planning artifacts in place instead of rediscovering or regenerating them as a parallel workflow description.
- Rewrite `workflow.yaml`, `.plan/workflow-structure.yaml`, and `.plan/phases/*.md` in place to the new 8-phase order instead of leaving a scaffold-era parallel workflow description.
- Keep the generated workflow strictly linear, inline-only, and explicit-phase-only: no `parallel_groups`, no top-level `include`, and no phase-level `prompt_file`, `workflow_file`, `workflow_dir`, `checks`, or bounce-target lists.

## Implementation Details
- Replace the temporary upstream-`DCtx` compatibility path in `safe/src/decompress/dctx.rs` and `safe/src/decompress/frame.rs` with native Rust dictionary-backed decompression that preserves frame format, dict-ID, and error-code behavior.
- Replace the upstream-formatted-dictionary validation path in `safe/src/ffi/decompress.rs` with native validation derived from the existing FSE/HUF dictionary parsing logic.
- Replace `UpstreamBufferlessSession` in `safe/src/ffi/decompress.rs` with a native bufferless/session state machine that drives `ZSTD_decompressBegin`, `ZSTD_decompressContinue`, `ZSTD_nextSrcSizeToDecompress`, `ZSTD_decompressBlock`, and `ZSTD_decompressStream` without loading upstream symbols.
- `safe/scripts/capture-upstream-abi.sh --check` and `safe/scripts/verify-baseline-contract.sh` must stop assuming any prebuilt upstream `libzstd.so` lives under `original/lib/`; they must validate the checked-in exports, SONAME, and ownership metadata directly and treat `safe/abi/original.*` as the authoritative baseline inputs.
- `safe/scripts/capture-upstream-abi.sh` must remain the explicit baseline recapture path for `safe/abi/original.exports.txt` and `safe/abi/original.soname.txt`; only its verification path changes in this phase.
- When rewriting `workflow.yaml`, `.plan/workflow-structure.yaml`, and the numbered phase markdown files, keep every verifier as an explicit top-level `script` or `check` phase with exactly one fixed `bounce_target` equal to the implement phase it verifies.
- `original/libzstd-1.5.5+dfsg2/` remains a consumed baseline, not a normal patch target; default to zero edits under `original/` and keep fixes on the safe side unless no safe-side change can preserve the required public interface.
- Keep `safe/src/ffi/legacy_shim.c` as the only decompression-side C bridge if legacy v0.5-v0.7 decode truly still requires it; document the exact boundary in `safe/docs/unsafe-audit.md`.
- Extend the decompression test surface to cover:
  - `ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_decompress_usingDict`, and `ZSTD_decompress_usingDDict`
  - `ZSTD_decompressBegin`, `ZSTD_decompressContinue`, `ZSTD_nextSrcSizeToDecompress`, and `ZSTD_decompressBlock`
  - legacy decode and size-query behavior
  - corrupt-dictionary and corrupt-frame error parity

## Verification Phases
- `script_safe_decompression_independence` | type: `script` | `bounce_target: impl_safe_decompression_independence` | purpose: build the library, run decompression-side Rust/C ABI gates, and prove that decompression no longer requires an upstream shared object.
- `check_safe_decompression_software_tester` | type: `check` | `bounce_target: impl_safe_decompression_independence` | purpose: review decompression semantics, new regression coverage, and negative-path handling.
- `check_safe_decompression_senior_tester` | type: `check` | `bounce_target: impl_safe_decompression_independence` | purpose: review ABI preservation, unsafe reduction, and elimination of decompression-side dynamic loading.

## Verification Commands
- `cargo test --manifest-path safe/Cargo.toml --release --test decompress`
- `bash safe/scripts/run-capi-decompression.sh`
- `bash safe/scripts/verify-export-parity.sh`
- `bash safe/scripts/verify-baseline-contract.sh`
- `rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym' safe/src/decompress safe/src/ffi/decompress.rs safe/scripts/run-capi-decompression.sh`

## Success Criteria
- Decompression-side library code and decompression harness scripts no longer depend on `SAFE_UPSTREAM_LIB`, `load_upstream!`, `dlopen()`, or `dlsym()`, except for any explicitly documented legacy shim boundary.
- `safe/abi/export_map.toml`, `safe/tests/upstream_test_matrix.toml`, and `safe/scripts/verify-baseline-contract.sh` reflect the fixed ownership rebase and validate the checked-in ABI baseline files directly.
- No export entry remains assigned above Phase 3 and no upstream-suite entry remains assigned above Phase 5 after the Phase 1 rebase.
- `workflow.yaml`, `.plan/workflow-structure.yaml`, and the numbered `.plan/phases/*.md` files are rewritten in place, remain aligned with `.plan/plan.md`, and preserve a strictly linear, inline-only, explicit-phase workflow with one fixed `bounce_target` per verifier.
- The listed verification commands pass and the new decompression regressions are checked in next to the touched Rust and C API surfaces.

## Git Commit Requirement
The implementer must commit all work for this phase to git before yielding to the verifier phases.
