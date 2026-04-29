# Phase Name

Advanced ABI, Dict-Builder, Threading, and Build Purge

# Implement Phase ID

`impl_safe_advanced_abi_completion`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- All outputs from `impl_safe_compression_core`, including `safe/abi/export_map.toml`, `safe/tests/upstream_test_matrix.toml`, `safe/src/compress/`, `safe/src/ffi/compress.rs`, `safe/tests/rust/compress.rs`, `safe/tests/capi/`, and `safe/scripts/run-capi-roundtrip.sh`
- `safe/Cargo.toml`
- `safe/build.rs`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `safe/abi/original.exports.txt`
- `safe/abi/original.soname.txt`
- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
- `safe/src/ffi/compress.rs`
- `safe/src/ffi/advanced.rs`
- `safe/src/ffi/legacy_shim.c`
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
- `original/libzstd-1.5.5+dfsg2/lib/dictBuilder/`
- `original/libzstd-1.5.5+dfsg2/lib/common/pool.c`
- `original/libzstd-1.5.5+dfsg2/lib/common/pool.h`
- `original/libzstd-1.5.5+dfsg2/lib/common/threading.c`
- `original/libzstd-1.5.5+dfsg2/lib/common/threading.h`
- `original/libzstd-1.5.5+dfsg2/lib/compress/zstdmt_compress.c`
- `original/libzstd-1.5.5+dfsg2/lib/compress/zstdmt_compress.h`
- `original/libzstd-1.5.5+dfsg2/tests/poolTests.c`
- `original/libzstd-1.5.5+dfsg2/tests/zstreamtest.c`
- `original/libzstd-1.5.5+dfsg2/tests/fuzz/sequence_compression_api.c`
- `original/libzstd-1.5.5+dfsg2/examples/streaming_compression_thread_pool.c`
- `original/libzstd-1.5.5+dfsg2/examples/streaming_memory_usage.c`

The upstream helper sources are reference inputs only. The final safe library must not keep `upstream-phase4` as a shipping helper archive.

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
- Remove `dlopen()`, `dlsym()`, and the `load_upstream!` macro from `safe/src/ffi/compress.rs`.
- Port advanced parameter, dictionary, static-context, sequence API, threading, and dictionary-builder entry points to Rust.
- Preserve only the truly required legacy shim C boundary, with `safe/src/ffi/legacy_shim.c` as the ceiling rather than a baseline for more C.

# Implementation Details

- `safe/build.rs` must still emit cfgs, SONAME, variant selection, and the legacy shim build if needed, but it must stop compiling and linking upstream `common/*.c`, `compress/*.c`, `decompress/*.c`, and `dictBuilder/*.c` as hidden helpers.
- `safe/src/compress/cctx_params.rs`, `safe/src/compress/cdict.rs`, `safe/src/compress/sequence_api.rs`, and `safe/src/compress/static_ctx.rs` must stop forwarding to `libzstd_safe_internal_*` helper symbols and become Rust-owned implementations that preserve the public ABI.
- `safe/src/threading/*.rs` must preserve shared-library multithread defaults and `-mt` / `-nomt` behavior already modeled by `safe/build.rs`.
- `safe/src/dict_builder/*.rs` must port COVER, FastCover, divsufsort, and zdict behavior into Rust or Rust-side glue with remaining unsafe limited to ABI buffer handling.
- Link compatibility must hold for objects compiled against upstream headers; exported symbols, SONAME, and parameter ABI stay exact.
- Preserve the fixed Phase 1 ownership rebase; later metadata edits must not re-shift preexisting `owning_phase` values.

# Verification Phases

- Phase ID: `script_safe_advanced_abi_completion`
  - Type: `check`
  - `bounce_target`: `impl_safe_advanced_abi_completion`
  - Purpose: verify advanced APIs, dictionary builders, multithreaded entry points, export parity, and link compatibility after helper-archive removal.
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
  - Commands: none; perform source, test, and evidence review.
- Phase ID: `check_safe_advanced_abi_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_safe_advanced_abi_completion`
  - Purpose: review that the shipping library no longer depends on `dlopen()` or the `upstream-phase4` helper archive.
  - Commands: none; perform senior implementation, ABI, and build-boundary review.

# Success Criteria

- `safe/build.rs` no longer produces or links the hidden upstream helper archive.
- Advanced, dictionary-builder, sequence, static-context, and threading surfaces are native to the safe crate.
- Export parity and link compatibility pass.
- The only remaining C is justified ABI-boundary or legacy-format glue and is documented.
- All listed verifier commands pass or any failure is fixed before yielding.

# Git Commit Requirement

The implementer must commit the Phase 3 work to git before yielding. That commit must exist before any verifier phase for `impl_safe_advanced_abi_completion` runs.
