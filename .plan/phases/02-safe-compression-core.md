# Phase Name

Safe Compression Core Independence

# Implement Phase ID

`impl_safe_compression_core`

# Preexisting Inputs

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
- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
- `safe/src/decompress/dctx.rs`
- `safe/src/decompress/ddict.rs`
- `safe/src/decompress/dstream.rs`
- `safe/src/decompress/frame.rs`
- `safe/src/decompress/huf.rs`
- `safe/src/decompress/fse.rs`
- `safe/src/ffi/decompress.rs`
- `safe/tests/rust/decompress.rs`
- `safe/tests/capi/decompress_smoke.c`
- `safe/tests/capi/frame_probe.c`
- `safe/tests/capi/legacy_decode.c`
- `safe/scripts/run-capi-decompression.sh`
- `safe/scripts/capture-upstream-abi.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `safe/docs/unsafe-audit.md`
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
- `safe/src/ffi/compress.rs`
- `safe/tests/rust/compress.rs`
- `safe/tests/capi/roundtrip_smoke.c`
- `safe/tests/capi/bigdict_driver.c`
- `safe/tests/capi/invalid_dictionaries_driver.c`
- `safe/tests/capi/zstream_driver.c`
- `safe/tests/capi/paramgrill_driver.c`
- `safe/tests/capi/external_matchfinder_driver.c`
- `safe/scripts/run-capi-roundtrip.sh`
- `safe/scripts/run-original-examples.sh`
- `original/libzstd-1.5.5+dfsg2/lib/compress/*`
- `original/libzstd-1.5.5+dfsg2/tests/bigdict.c`
- `original/libzstd-1.5.5+dfsg2/tests/invalidDictionaries.c`
- `original/libzstd-1.5.5+dfsg2/tests/roundTripCrash.c`
- `original/libzstd-1.5.5+dfsg2/tests/paramgrill.c`
- `original/libzstd-1.5.5+dfsg2/tests/external_matchfinder.c`
- `original/libzstd-1.5.5+dfsg2/examples/*compression*.c`

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

- Remove `load_upstream!` use from the core compression entry points.
- Replace placeholder `compress/frame.rs`, `compress/literals.rs`, `compress/ldm.rs`, `compress/match_state.rs`, `compress/sequences.rs`, and strategy modules with real Rust implementations or Rust-facing orchestration around already translated primitives.
- Add checked-in round-trip regressions for one-shot, block, streaming, and dictionary-using compression.
- Preserve the Phase 1 rebased ownership metadata; later phases may update status, owner-module, prerequisite, helper-path, and `release_gate` metadata but must not re-shift preexisting entries.

# Implementation Details

- Port the `ZSTD_compress`, `ZSTD_compressCCtx`, `ZSTD_compress2`, `ZSTD_compressBegin`, `ZSTD_compressContinue`, `ZSTD_compressEnd`, and `ZSTD_copyCCtx` surfaces to native Rust state management in `safe/src/compress/cctx.rs`.
- Port `ZSTD_getBlockSize`, `ZSTD_compressBlock`, and any required block-insertion behavior in `safe/src/compress/block.rs` so block-level callers no longer hit upstream symbols.
- Port parameter and bounds-query logic in `safe/src/compress/params.rs` instead of resolving it from upstream.
- Port streaming compression in `safe/src/compress/cstream.rs` so `ZSTD_initCStream*`, `ZSTD_compressStream*`, `ZSTD_flushStream`, and `ZSTD_endStream` are native.
- Keep behavior source-compatible with upstream, especially around pledged source size, checksum flags, dictionary use, and block-size limits.
- Use upstream compression sources under `original/libzstd-1.5.5+dfsg2/` as reference material while preserving the consume-existing-artifacts contract.

# Verification Phases

- Phase ID: `script_safe_compression_core`
- Type: `check`
- `bounce_target`: `impl_safe_compression_core`
- Purpose: run one-shot, block, streaming, and dictionary-using compression gates against the Rust implementation.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - `cargo test --manifest-path safe/Cargo.toml --release --test compress`
  - `bash safe/scripts/run-capi-roundtrip.sh`
  - `bash safe/scripts/run-original-examples.sh`
  - `rg -n 'load_upstream!' safe/src/compress/block.rs safe/src/compress/cctx.rs safe/src/compress/cstream.rs safe/src/compress/params.rs`

- Phase ID: `check_safe_compression_core_software_tester`
- Type: `check`
- `bounce_target`: `impl_safe_compression_core`
- Purpose: review round-trip correctness, error handling, and test coverage for the core compression APIs.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

- Phase ID: `check_safe_compression_core_senior_tester`
- Type: `check`
- `bounce_target`: `impl_safe_compression_core`
- Purpose: review API completeness and the removal of shared-object fallback from the compression core.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

# Success Criteria

- Core compression APIs no longer forward through the upstream dynamic-loader path.
- One-shot, block, streaming, and dictionary compression have checked-in regressions.
- The Phase 1 ownership rebase remains intact and no preexisting entry is re-shifted.
- The listed Rust, C API, examples, and source-search verifier commands pass.

# Git Commit Requirement

The implementer must commit all Phase 2 work to git before yielding. That commit must exist before `script_safe_compression_core`, `check_safe_compression_core_software_tester`, and `check_safe_compression_core_senior_tester` run.
