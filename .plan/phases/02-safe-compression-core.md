# Phase Name

Safe Compression Core Independence

# Implement Phase ID

`impl_safe_compression_core`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- All outputs from `impl_safe_decompression_independence`, including `.plan/phases/`, `.plan/workflow-structure.yaml`, `workflow.yaml`, `safe/abi/export_map.toml`, `safe/tests/upstream_test_matrix.toml`, `safe/src/decompress/`, `safe/src/ffi/decompress.rs`, `safe/tests/rust/decompress.rs`, `safe/tests/capi/`, `safe/scripts/run-capi-decompression.sh`, `safe/scripts/capture-upstream-abi.sh`, `safe/scripts/verify-baseline-contract.sh`, and `safe/docs/unsafe-audit.md`
- `safe/Cargo.toml`
- `safe/build.rs`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `safe/abi/original.exports.txt`
- `safe/abi/original.soname.txt`
- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
- `safe/src/common/error.rs`
- `safe/src/common/frame.rs`
- `safe/src/compress/block.rs`
- `safe/src/compress/cctx.rs`
- `safe/src/compress/cstream.rs`
- `safe/src/compress/frame.rs`
- `safe/src/compress/literals.rs`
- `safe/src/compress/ldm.rs`
- `safe/src/compress/match_state.rs`
- `safe/src/compress/params.rs`
- `safe/src/compress/sequences.rs`
- `safe/src/compress/strategies/double_fast.rs`
- `safe/src/compress/strategies/fast.rs`
- `safe/src/compress/strategies/lazy.rs`
- `safe/src/compress/strategies/opt.rs`
- `safe/src/decompress/dctx.rs`
- `safe/src/decompress/ddict.rs`
- `safe/src/decompress/frame.rs`
- `safe/src/ffi/compress.rs`
- `safe/src/ffi/decompress.rs`
- `safe/tests/rust/compress.rs`
- `safe/tests/rust/decompress.rs`
- `safe/tests/capi/roundtrip_smoke.c`
- `safe/tests/capi/bigdict_driver.c`
- `safe/tests/capi/invalid_dictionaries_driver.c`
- `safe/tests/capi/zstream_driver.c`
- `safe/tests/capi/paramgrill_driver.c`
- `safe/tests/capi/external_matchfinder_driver.c`
- `safe/scripts/run-capi-roundtrip.sh`
- `safe/scripts/run-original-examples.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `original/libzstd-1.5.5+dfsg2/lib/compress/`
- `original/libzstd-1.5.5+dfsg2/tests/bigdict.c`
- `original/libzstd-1.5.5+dfsg2/tests/invalidDictionaries.c`
- `original/libzstd-1.5.5+dfsg2/tests/roundTripCrash.c`
- `original/libzstd-1.5.5+dfsg2/tests/paramgrill.c`
- `original/libzstd-1.5.5+dfsg2/tests/external_matchfinder.c`
- `original/libzstd-1.5.5+dfsg2/examples/`

The decompression implementation, rebased ownership metadata, regenerated `workflow.yaml`, and upstream compression sources are consumed in place. Do not patch the upstream source snapshot unless no safe-side change can preserve a public interface.

# New Outputs

- rewritten `safe/abi/export_map.toml`
- rewritten `safe/tests/upstream_test_matrix.toml`
- rewritten `safe/src/compress/block.rs`
- rewritten `safe/src/compress/cctx.rs`
- rewritten `safe/src/compress/cstream.rs`
- implemented `safe/src/compress/frame.rs`
- implemented `safe/src/compress/literals.rs`
- implemented `safe/src/compress/ldm.rs`
- implemented `safe/src/compress/match_state.rs`
- rewritten `safe/src/compress/params.rs`
- implemented `safe/src/compress/sequences.rs`
- implemented `safe/src/compress/strategies/double_fast.rs`
- implemented `safe/src/compress/strategies/fast.rs`
- implemented `safe/src/compress/strategies/lazy.rs`
- implemented `safe/src/compress/strategies/opt.rs`
- rewritten `safe/src/ffi/compress.rs`
- rewritten `safe/tests/rust/compress.rs`
- rewritten `safe/tests/capi/roundtrip_smoke.c`
- rewritten `safe/tests/capi/bigdict_driver.c`
- rewritten `safe/tests/capi/invalid_dictionaries_driver.c`
- rewritten `safe/tests/capi/zstream_driver.c`
- rewritten `safe/tests/capi/paramgrill_driver.c`
- rewritten `safe/tests/capi/external_matchfinder_driver.c`
- rewritten `safe/scripts/run-capi-roundtrip.sh`

# File Changes

- Remove `load_upstream!` use from core compression entry points.
- Replace placeholder `compress/frame.rs`, `compress/literals.rs`, `compress/ldm.rs`, `compress/match_state.rs`, `compress/sequences.rs`, and strategy modules with Rust implementations or Rust-facing orchestration around already translated primitives.
- Add checked-in round-trip regressions for one-shot, block, streaming, and dictionary-using compression.
- Preserve Phase 1 ownership rebasing; this phase may update status, helper-path, prerequisite, or release-gate metadata, but it must not re-shift existing `owning_phase` values.

# Implementation Details

- Port `ZSTD_compress`, `ZSTD_compressCCtx`, `ZSTD_compress2`, `ZSTD_compressBegin`, `ZSTD_compressContinue`, `ZSTD_compressEnd`, and `ZSTD_copyCCtx` to native Rust state management in `safe/src/compress/cctx.rs`.
- Port `ZSTD_getBlockSize`, `ZSTD_compressBlock`, and required block-insertion behavior in `safe/src/compress/block.rs` so block-level callers no longer hit upstream symbols.
- Port parameter and bounds-query logic in `safe/src/compress/params.rs` instead of resolving it from upstream.
- Port streaming compression in `safe/src/compress/cstream.rs` so `ZSTD_initCStream*`, `ZSTD_compressStream*`, `ZSTD_flushStream`, and `ZSTD_endStream` are native.
- Keep behavior source-compatible with upstream around pledged source size, checksum flags, dictionary use, parameter errors, and block-size limits.
- Continue consuming the checked-in ABI baseline and upstream test matrix from the Phase 1 state.

# Verification Phases

- Phase ID: `script_safe_compression_core`
  - Type: `check`
  - `bounce_target`: `impl_safe_compression_core`
  - Purpose: run one-shot, block, streaming, and dictionary-using compression gates against the Rust implementation.
  - Commands:
    - `cargo test --manifest-path safe/Cargo.toml --release --test compress`
    - `bash safe/scripts/run-capi-roundtrip.sh`
    - `bash safe/scripts/run-original-examples.sh`
    - `rg -n 'load_upstream!' safe/src/compress/block.rs safe/src/compress/cctx.rs safe/src/compress/cstream.rs safe/src/compress/params.rs`
- Phase ID: `check_safe_compression_core_software_tester`
  - Type: `check`
  - `bounce_target`: `impl_safe_compression_core`
  - Purpose: review round-trip correctness, error handling, and test coverage for the core compression APIs.
  - Commands: none; perform source, test, and evidence review.
- Phase ID: `check_safe_compression_core_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_safe_compression_core`
  - Purpose: review API completeness, ownership metadata discipline, and removal of shared-object fallback from the compression core.
  - Commands: none; perform senior implementation and ABI review.

# Success Criteria

- Core compression no longer forwards through shared-object fallback.
- Placeholder compression modules are replaced by native Rust behavior for the covered surfaces.
- Round-trip, dictionary, block, streaming, and upstream example coverage passes.
- No Phase 1 rebased ownership values are shifted again.
- All listed verifier commands pass or any failure is fixed before yielding.

# Git Commit Requirement

The implementer must commit the Phase 2 work to git before yielding. That commit must exist before any verifier phase for `impl_safe_compression_core` runs.
