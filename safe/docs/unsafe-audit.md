# Unsafe Audit

Last reviewed: 2026-03-28

This crate still contains `unsafe`, but the remaining surface is intentionally
constrained to three cases:

1. FFI boundaries where the public `libzstd` ABI is defined in terms of raw
   pointers, opaque C contexts, or `unsafe extern "C"` callbacks.
2. Allocator and ownership interop where opaque `ZSTD_*Ctx` / `ZSTD_*Dict`
   handles are created in Rust and later recovered from raw pointers.
3. Equivalent low-level glue that cannot be expressed in safe Rust while
   preserving the upstream ABI contract, such as out-parameter writes,
   `copy_nonoverlapping`, or direct calls into linked helper symbols.

No `unsafe` remains for core compression or decompression logic itself. The
algorithmic work is in safe Rust or in explicitly linked helper entry points.

## Inventory

- `safe/src/ffi/types.rs`: declares allocator callbacks and exported function
  pointer types as `unsafe extern "C"` so the Rust types exactly match the C
  ABI that downstream callers expect.
- `safe/src/ffi/compress.rs`: uses `dlopen`, `dlsym`, and
  `core::mem::transmute_copy` to lazily bind selected compatibility symbols
  from the upstream `libzstd.so.1`. This is unavoidable dynamic-linker glue.
- `safe/src/ffi/decompress.rs`: validates formatted dictionaries through the
  upstream ABI, converts incoming raw pointers to slices and typed contexts,
  recovers `Box` ownership in `free_dctx` / `free_ddict`, and copies staged
  decode output into caller-owned buffers. These are the main allocator and
  pointer-interoperability sites.
- `safe/src/common/frame.rs`: writes a parsed `ZSTD_frameHeader` into the
  caller-provided `ZSTD_frameHeader*` output pointer.
- `safe/src/common/skippable.rs`: writes the optional `magicVariant`
  out-parameter, copies skippable-frame payload bytes into the caller buffer,
  and forwards `ZSTD_writeSkippableFrame` to the linked helper symbol.
- `safe/src/compress/block.rs`: dispatches block-oriented compatibility entry
  points (`ZSTD_getBlockSize`, `ZSTD_compressBlock`, `ZSTD_insertBlock`) through
  dynamically loaded upstream symbols because the ABI is pointer-based and the
  implementations stay delegated.
- `safe/src/compress/cctx.rs`: declares and calls linked helper symbols for
  `CCtx` allocation, parameter setting, size estimation, and advanced
  compression entry points. The unsafe here is limited to raw C context calls.
- `safe/src/compress/cctx_params.rs`: forwards `CCtxParams` creation, mutation,
  and query operations to linked helper symbols while preserving upstream
  parameter-struct layout and pointer semantics.
- `safe/src/compress/cdict.rs`: bridges `CDict` creation, destruction, loading,
  and `CDict`-based compression calls through helper symbols and upstream
  compatibility hooks. The unsafe is raw-pointer ABI glue around opaque dict
  objects.
- `safe/src/compress/cstream.rs`: forwards streaming-compression constructors,
  reset operations, and stream-step functions to linked helper symbols. This is
  still opaque C-stream interop, not algorithmic unsafe.
- `safe/src/compress/params.rs`: exposes parameter-bound queries by calling
  linked helper symbols with upstream enum and struct layouts.
- `safe/src/compress/sequence_api.rs`: forwards sequence-API entry points to
  linked helpers because the exported ABI still accepts raw buffers and opaque
  `CCtx` state.
- `safe/src/compress/static_ctx.rs`: initializes static `CCtx`, `DCtx`,
  `CDict`, `DDict`, `CStream`, and `DStream` objects against caller-supplied
  workspaces. This is unavoidable raw-workspace glue.
- `safe/src/decompress/dctx.rs`: creates a temporary upstream `DCtx` for the
  dict-based compatibility path and writes integer out-parameters for
  `ZSTD_DCtx_getParameter`.
- `safe/src/decompress/dstream.rs`: reborrows caller-supplied
  `ZSTD_inBuffer*` / `ZSTD_outBuffer*`, reads and writes `dstPos` / `srcPos`
  pointers, and forwards size-estimator helper calls.
- `safe/src/decompress/frame.rs`: creates a temporary upstream `DCtx` when
  dictionary-backed decode behavior must match upstream exactly and copies the
  final decoded payload into the caller buffer.
- `safe/src/decompress/legacy.rs`: calls into the C legacy-shim helpers for
  legacy frame detection, sizing, and decode because that compatibility layer
  is still maintained as linked C glue.
- `safe/src/dict_builder/cover.rs`: forwards COVER dictionary-training entry
  points to linked helper symbols with caller-owned sample buffers and
  parameter structs.
- `safe/src/dict_builder/fastcover.rs`: same rationale as `cover.rs`, but for
  FastCover training and optimization entry points.
- `safe/src/dict_builder/zdict.rs`: forwards legacy and modern `ZDICT_*`
  training, finalization, and error-reporting entry points to linked helpers.
- `safe/src/threading/pool.rs`: exposes thread-pool allocation, destruction,
  and `CCtx` association through helper symbols. This is ABI-level threading
  glue around opaque C types.
- `safe/src/threading/zstdmt.rs`: forwards the multithread flush/progression
  helpers to linked symbols to preserve the upstream ABI.
- `safe/tests/rust/compress.rs`: uses `CStr::from_ptr` only to turn upstream
  error-name pointers into Rust strings for assertion messages.
- `safe/tests/rust/decompress.rs`: uses `CStr::from_ptr` for the same
  assertion-message path plus the exported `ZSTD_versionString()` test.

## What Is Intentionally Not Here

- There is no `unsafe` in the safe Rust frame parser, block decoder, FSE/HUF
  decoding logic, or the high-level structured decompression flow.
- There is no `unsafe` in the shell tooling under `safe/scripts/`.
- There is no remaining `unsafe` that exists only for convenience. Every site
  is there to preserve the published C ABI or to bridge ownership across it.
