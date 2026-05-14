# Phase Name

Advanced ABI, Dict-Builder, Threading, and Build Purge

# Implement Phase ID

`impl_safe_advanced_abi_completion`

# Preexisting Inputs

- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
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
- `safe/build.rs`
- `safe/src/ffi/advanced.rs`
- `safe/src/compress/cctx_params.rs`
- `safe/src/compress/cdict.rs`
- `safe/src/compress/sequence_api.rs`
- `safe/src/compress/static_ctx.rs`
- `safe/src/threading/job_queue.rs`
- `safe/src/threading/pool.rs`
- `safe/src/threading/zstdmt.rs`
- `safe/src/dict_builder/cover.rs`
- `safe/src/dict_builder/divsufsort.rs`
- `safe/src/dict_builder/fastcover.rs`
- `safe/src/dict_builder/zdict.rs`
- `safe/tests/capi/dict_builder_driver.c`
- `safe/tests/capi/sequence_api_driver.c`
- `safe/tests/capi/thread_pool_driver.c`
- `safe/tests/link-compat/Makefile`
- `safe/tests/link-compat/run_pooltests.c`
- `safe/tests/link-compat/run_zstreamtest.c`
- `safe/scripts/run-advanced-mt-tests.sh`
- `safe/scripts/verify-link-compat.sh`
- `safe/scripts/verify-export-parity.sh`
- `safe/docs/unsafe-audit.md`
- `original/libzstd-1.5.5+dfsg2/lib/dictBuilder/*`
- `original/libzstd-1.5.5+dfsg2/lib/common/pool.*`
- `original/libzstd-1.5.5+dfsg2/lib/common/threading.*`
- `original/libzstd-1.5.5+dfsg2/lib/compress/zstdmt_compress.*`
- `original/libzstd-1.5.5+dfsg2/tests/poolTests.c`
- `original/libzstd-1.5.5+dfsg2/tests/zstreamtest.c`
- `original/libzstd-1.5.5+dfsg2/tests/fuzz/sequence_compression_api.c`
- `original/libzstd-1.5.5+dfsg2/examples/streaming_compression_thread_pool.c`
- `original/libzstd-1.5.5+dfsg2/examples/streaming_memory_usage.c`

# New Outputs

- rewritten `safe/build.rs`
- rewritten `safe/abi/export_map.toml`
- rewritten `safe/tests/upstream_test_matrix.toml`
- rewritten `safe/src/ffi/compress.rs`
- rewritten `safe/src/ffi/advanced.rs`
- rewritten `safe/src/compress/cctx_params.rs`
- rewritten `safe/src/compress/cdict.rs`
- rewritten `safe/src/compress/sequence_api.rs`
- rewritten `safe/src/compress/static_ctx.rs`
- rewritten `safe/src/threading/job_queue.rs`
- rewritten `safe/src/threading/pool.rs`
- rewritten `safe/src/threading/zstdmt.rs`
- rewritten `safe/src/dict_builder/cover.rs`
- rewritten `safe/src/dict_builder/divsufsort.rs`
- rewritten `safe/src/dict_builder/fastcover.rs`
- rewritten `safe/src/dict_builder/zdict.rs`
- rewritten `safe/tests/capi/dict_builder_driver.c`
- rewritten `safe/tests/capi/sequence_api_driver.c`
- rewritten `safe/tests/capi/thread_pool_driver.c`
- rewritten `safe/tests/link-compat/Makefile`
- rewritten `safe/scripts/run-advanced-mt-tests.sh`
- rewritten `safe/scripts/verify-link-compat.sh`
- rewritten `safe/docs/unsafe-audit.md`

# File Changes

- Remove `compile_upstream_phase4_helpers()` and the generated hidden helper archive from `safe/build.rs`.
- Remove `dlopen()` / `dlsym()` and the `load_upstream!` macro from `safe/src/ffi/compress.rs`.
- Port advanced parameter, dictionary, static-context, sequence API, threading, and dictionary-builder entry points to Rust.
- Preserve only the truly required legacy shim C boundary.
- Keep link compatibility for objects compiled against upstream headers.

# Implementation Details

- `safe/build.rs` must still emit cfgs, SONAME, and the legacy shim build if needed, but it must stop compiling and linking `common/*.c`, `compress/*.c`, `decompress/*.c`, and `dictBuilder/*.c` as hidden helpers.
- `safe/src/compress/cctx_params.rs`, `safe/src/compress/cdict.rs`, `safe/src/compress/sequence_api.rs`, and `safe/src/compress/static_ctx.rs` must stop forwarding to `libzstd_safe_internal_*` helper symbols and become Rust-owned implementations that preserve the public ABI.
- `safe/src/threading/*.rs` must preserve the shared-library multithread default and `-mt` / `-nomt` behavior already modeled by `safe/build.rs` and the upstream lib Makefile contract.
- `safe/src/dict_builder/*.rs` must port COVER/FastCover/divsufsort/zdict behavior into Rust or Rust-side glue that keeps remaining unsafe limited to ABI buffer handling.
- Link compatibility must still hold for objects compiled against upstream headers, so the exported symbol list, SONAME, and parameter ABI must stay exact.
- Preserve the Phase 1 ownership rebase; this phase may update statuses and metadata in place but must not re-shift preexisting `owning_phase` values.

# Verification Phases

- Phase ID: `script_safe_advanced_abi_completion`
- Type: `check`
- `bounce_target`: `impl_safe_advanced_abi_completion`
- Purpose: verify advanced APIs, dictionary builders, multithreaded entry points, export parity, and link compatibility after helper-archive removal.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - `bash safe/scripts/run-advanced-mt-tests.sh`
  - `bash safe/scripts/verify-link-compat.sh`
  - `bash safe/scripts/verify-export-parity.sh`
  - `cargo test --manifest-path safe/Cargo.toml --release --all-targets`
  - `rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe`

- Phase ID: `check_safe_advanced_abi_software_tester`
- Type: `check`
- `bounce_target`: `impl_safe_advanced_abi_completion`
- Purpose: review advanced API semantics, new regressions, and multithreaded coverage.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

- Phase ID: `check_safe_advanced_abi_senior_tester`
- Type: `check`
- `bounce_target`: `impl_safe_advanced_abi_completion`
- Purpose: review that the shipping library no longer depends on `dlopen()` or the `upstream-phase4` helper archive.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

# Success Criteria

- No shipping path depends on `SAFE_UPSTREAM_LIB`, `load_upstream!`, `dlopen()`, `dlsym()`, or `upstream-phase4`.
- Advanced ABI, dictionary-builder, static-context, sequence API, and multithreaded APIs are implemented in safe-side Rust or justified ABI glue.
- Export parity and link compatibility still pass.
- The crate feature matrix and Phase 1 metadata rebase remain stable.

# Git Commit Requirement

The implementer must commit all Phase 3 work to git before yielding. That commit must exist before `script_safe_advanced_abi_completion`, `check_safe_advanced_abi_software_tester`, and `check_safe_advanced_abi_senior_tester` run.
