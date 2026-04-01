# Unsafe Audit

Last reviewed: 2026-03-31

The shipping `libzstd` no longer relies on dynamic loader symbol resolution,
an environment-selected upstream helper library, or a hidden helper-object
archive for advanced compression APIs. `safe/build.rs` only compiles the
bounded legacy decode shim, while the advanced parameter, dictionary,
static-context, sequence, threading, and dictionary-builder entry points are
owned by Rust code in the shared library itself.

The final release gate now consumes the refreshed Phase 4 install and Debian
outputs together with the Phase 6 dependent image artifacts directly. It does
not rebuild those roots implicitly, and it does not reintroduce any runtime
dependency on upstream C beyond the approved legacy decode shim.

## Remaining Unsafe Categories

1. C ABI boundaries that still accept raw pointers, opaque handles, callback
   function pointers, or caller-owned out-parameters.
2. Ownership transfers for boxed Rust state that backs opaque `ZSTD_*`
   handles, including compression contexts, decompression contexts,
   dictionaries, thread pools, and parameter blocks.
3. Raw buffer reborrows needed to convert caller-provided memory into slices or
   to copy produced data back into caller-owned output storage.
4. The legacy v0.5-v0.7 decode shim, which remains the only linked C bridge in
   the shipping library.

No `unsafe` remains in order to support deleted helper archives, dynamic symbol
resolution, or transitional upstream replay paths.

## Module Inventory

- `safe/src/common/frame.rs` and `safe/src/common/skippable.rs` write parsed
  results into caller-owned out-parameters and copy payload bytes across the C
  ABI boundary.
- `safe/src/ffi/compress.rs` and `safe/src/ffi/decompress.rs` validate raw
  input/output pointers, reinterpret opaque handles as Rust-owned state, and
  recover `Box` ownership in the corresponding free functions.
- `safe/src/compress/cctx.rs`, `safe/src/compress/cctx_params.rs`,
  `safe/src/compress/cdict.rs`, `safe/src/compress/cstream.rs`,
  `safe/src/compress/sequence_api.rs`, and `safe/src/compress/static_ctx.rs`
  preserve the upstream ABI while converting caller buffers and opaque handles
  into Rust-owned encoder state.
- `safe/src/threading/pool.rs` and `safe/src/threading/zstdmt.rs` cast opaque
  thread-pool handles, report frame progression, and expose the public
  multithread ABI without leaving Rust-owned state.
- `safe/src/dict_builder/cover.rs`, `safe/src/dict_builder/fastcover.rs`, and
  `safe/src/dict_builder/zdict.rs` reinterpret caller sample buffers, write
  trained dictionary bytes in place, and report dictionary metadata through the
  public C entry points.
- `safe/src/decompress/ddict.rs`, `safe/src/decompress/dctx.rs`,
  `safe/src/decompress/dstream.rs`, and `safe/src/decompress/frame.rs`
  reinterpret caller buffers and context handles while the actual modern decode
  pipeline remains Rust-owned.
- `safe/src/decompress/legacy.rs` is the only remaining foreign bridge; it
  forwards legacy frame support to the dedicated C shim built from upstream
  legacy sources.
- `safe/src/compress/compat.rs`, `safe/src/compress/match_state.rs`,
  `safe/src/compress/ldm.rs`, `safe/src/compress/literals.rs`,
  `safe/src/compress/sequences.rs`, and the files under
  `safe/src/compress/strategies/` use `unsafe` only to adapt upstream-compatible
  raw structs and buffers to Rust implementations of the compression pipeline.
- `safe/tests/rust/compress.rs` and `safe/tests/rust/decompress.rs` use
  `unsafe` only for `CStr::from_ptr()` conversions when asserting C ABI
  results.

## Declaration-Only Unsafe

- `safe/src/decompress/legacy.rs` declares the linked legacy shim functions as
  `unsafe extern "C"`.
- `safe/src/ffi/types.rs` keeps allocator hooks and sequence producer callbacks
  as `unsafe extern "C" fn` typedefs so the published ABI matches upstream
  headers exactly.

## Final State

The remaining `unsafe` surface is now restricted to preserving the upstream C
ABI, handling opaque-handle ownership, copying across raw buffer boundaries,
and calling the bounded legacy decode shim. There is no remaining `unsafe`
used only for convenience.
