# Unsafe Audit

Last reviewed: 2026-03-28

Remaining `unsafe` is intentionally limited to three categories:

1. FFI boundaries where the published `libzstd` ABI still exposes raw pointers,
   opaque contexts, and foreign callbacks.
2. Allocator and ownership interop where Rust-owned `DecoderContext` or
   `DecoderDictionary` values are passed through opaque C handles and later
   recovered.
3. Equivalent low-level glue such as out-parameter writes, raw buffer copies,
   and dynamic symbol binding that cannot be expressed in safe Rust without
   changing the upstream ABI contract.

No `unsafe` remains in the core frame parser, block decoder, FSE/HUF decoding
logic, or the high-level Rust compression and decompression algorithms
themselves.

## Unsafe Block Inventory

### `safe/src/common/frame.rs`

- `90`: writes the parsed `ZSTD_frameHeader` into the caller-provided
  `ZSTD_frameHeader*` out-parameter.

### `safe/src/common/skippable.rs`

- `46`: writes the optional `magicVariant` out-parameter supplied by C callers.
- `50`: copies the skippable payload into the caller's destination buffer with
  `copy_nonoverlapping`.
- `70`: forwards `ZSTD_writeSkippableFrame()` to the linked C helper with the
  original raw pointer arguments unchanged.

### `safe/src/compress/block.rs`

- `11`, `26`, `39`: call dynamically loaded upstream block helpers
  (`ZSTD_getBlockSize`, `ZSTD_compressBlock`, `ZSTD_insertBlock`) with opaque
  C contexts and raw buffer pointers.

### `safe/src/compress/cctx.rs`

- `63`, `76`, `82`, `147`, `156`, `174`, `314`, `320`, `328`, `343`, `362`,
  `368`: call linked helper symbols for `CCtx` allocation, destruction,
  parameter mutation, advanced compression, and size estimation; the helper ABI
  is pointer-based and opaque.
- `95`, `112`, `127`, `140`, `167`, `199`, `223`, `232`, `246`, `262`, `277`,
  `292`, `301`: call optional compatibility symbols resolved from upstream with
  `dlsym`; the symbol type must exactly match the requested `unsafe extern "C"`
  signature.

### `safe/src/compress/cctx_params.rs`

- `64`, `73`, `82`, `88`, `103`, `112`, `122`, `131`, `137`, `146`, `155`:
  forward `CCtxParams` lifecycle and mutation APIs to linked helpers that still
  operate on raw C pointers and upstream-layout structs.

### `safe/src/compress/cdict.rs`

- `78`, `87`, `97`, `110`, `119`, `136`, `148`, `160`, `175`, `190`, `199`:
  call optional upstream `CDict` compatibility symbols through dynamically
  loaded function pointers.
- `215`, `232`, `251`, `261`, `271`, `282`, `294`, `311`: call linked helper
  symbols for advanced `CDict` creation, loading, and size estimation with raw
  dictionary buffers and opaque dictionary handles.

### `safe/src/compress/cstream.rs`

- `91`, `104`, `110`, `119`, `129`, `140`, `145`, `154`, `164`, `169`, `174`,
  `179`, `184`, `189`, `197`, `205`, `225`, `235`, `243`: call linked helper
  symbols for `CStream` creation, destruction, initialization, streaming steps,
  and size estimation; all of these APIs preserve the upstream raw-pointer ABI.

### `safe/src/compress/params.rs`

- `43`, `52`: call dynamically loaded bounds-query helpers with upstream enum
  values.
- `64`, `79`, `90`, `100`, `106`, `112`, `123`: call linked helper symbols for
  parameter derivation, validation, and compression-level queries.

### `safe/src/compress/sequence_api.rs`

- `41`, `55`, `67`, `77`, `86`: forward sequence-API entry points to linked
  helpers because the public ABI still accepts raw sequence arrays, buffers,
  and opaque `CCtx` state.

### `safe/src/compress/static_ctx.rs`

- `63`, `77`, `86`, `104`, `132`, `156`, `170`: initialize static contexts,
  dictionaries, and streams against caller-supplied workspaces via linked
  helper symbols; raw workspace pointers are part of the ABI contract.

### `safe/src/decompress/dctx.rs`

- `54`, `59`, `70`: create, use, and free a temporary upstream `DCtx` when the
  dict-backed compatibility path must match upstream decode semantics exactly.
- `337`: writes the current `ZSTD_dParameter` value into the caller-provided
  `int*` out-parameter for `ZSTD_DCtx_getParameter()`.

### `safe/src/decompress/dstream.rs`

- `72`: reborrows caller-provided `ZSTD_outBuffer*` and `ZSTD_inBuffer*` as
  mutable Rust references for streaming decode.
- `118`, `124`: call linked helper symbols for stream-size estimators.
- `145`, `151`: read the caller-owned `dstPos` and `srcPos` values for the
  simple-args streaming wrapper.
- `155`: writes updated `dstPos` and `srcPos` values back to the caller.

### `safe/src/decompress/frame.rs`

- `410`, `415`, `426`: create, use, and free a temporary upstream `DCtx` when
  dictionary-backed decode must match upstream exactly.
- `795`: copies the decoded frame payload into the caller's destination buffer.

### `safe/src/decompress/legacy.rs`

- `28`, `32`, `41`, `46`, `55`, `67`: call the legacy-support C shim for
  version probing, sizing, and decompression of legacy frames.

### `safe/src/dict_builder/cover.rs`

- `35`, `57`: forward COVER training and optimization entry points to linked
  helper symbols that operate on raw sample buffers and caller-owned output
  storage.

### `safe/src/dict_builder/fastcover.rs`

- `35`, `57`: forward FastCover training and optimization entry points to
  linked helper symbols with the same raw-buffer ABI constraints as upstream.

### `safe/src/dict_builder/zdict.rs`

- `65`, `89`, `113`, `131`, `140`, `152`, `166`, `172`: call linked helper
  symbols for dictionary training, dictionary finalization, header/id queries,
  and error-name lookup; these helpers still speak the upstream C ABI.

### `safe/src/ffi/compress.rs`

- `39`, `47`: call `dlopen()` on either the configured upstream library path or
  the fallback SONAME to resolve compatibility-only symbols at runtime.
- `60`: calls `dlsym()` to look up a compatibility symbol from the chosen
  upstream shared object.
- `65`: transmutes the resolved symbol pointer into the requested function
  pointer type after the caller has selected the exact ABI signature.
- `101`: the `load_upstream!` macro invokes `load_symbol()` in an unsafe block
  because the symbol name and requested function pointer type must agree.

### `safe/src/ffi/decompress.rs`

- `31`: calls upstream `ZSTD_createDDict()` to validate that a formatted
  dictionary is acceptable to the reference implementation.
- `35`: calls upstream `ZSTD_freeDDict()` to release that temporary validation
  handle.
- `250`, `257`, `261`, `267`, `268`, `271`, `272`: create and initialize a
  temporary upstream `DCtx` for bufferless decode replay, including format,
  window-size, and dict/no-dict start paths that must exactly match upstream.
- `307`: calls upstream `ZSTD_decompressContinue()` on the temporary session.
- `333`: calls upstream `ZSTD_nextSrcSizeToDecompress()` on the temporary
  session.
- `351`: calls upstream `ZSTD_decompressBlock()` on the temporary session.
- `381`: calls upstream `ZSTD_decompressStream()` on the temporary session.
- `403`: frees the temporary upstream `DCtx` in `Drop`.
- `431`: copies staged decoded output into the caller's destination buffer.
- `788`: converts an incoming `src` pointer plus size into a borrowed byte
  slice for the C ABI entry points.
- `796`: reinterprets an opaque `ZSTD_DCtx*` as the owned internal
  `DecoderContext` mutably.
- `804`: reinterprets an opaque `ZSTD_DCtx*` as the owned internal
  `DecoderContext` immutably.
- `812`: reinterprets an opaque dictionary pointer as `DecoderDictionary`.
- `840`: recovers `Box<DecoderContext>` ownership in `free_dctx()`.
- `862`: recovers `Box<DecoderDictionary>` ownership in `free_ddict()`.
- `1064`: copies the immediately produced prefix of a staged decode block into
  the caller's destination buffer before retaining any remainder.

### `safe/src/threading/pool.rs`

- `18`, `24`, `33`: forward thread-pool allocation, free, and `CCtx`
  association to linked helper symbols; the public ABI exposes opaque thread
  pool handles.

### `safe/src/threading/zstdmt.rs`

- `13`, `19`: call linked multithread helper symbols that expose opaque `CCtx`
  state through the upstream ABI.

### `safe/tests/rust/compress.rs`

- `88`: converts the C error-name pointer returned by `ZSTD_getErrorName()` to
  a Rust string for assertion messages.

### `safe/tests/rust/decompress.rs`

- `35`: converts the C error-name pointer returned by `ZSTD_getErrorName()` to
  a Rust string for assertion messages.
- `67`: converts the exported `ZSTD_versionString()` pointer to a Rust string.

## Declaration-Only Unsafe

These items are still part of the audited surface even though they are not
`unsafe { ... }` expression blocks:

- `safe/src/common/skippable.rs:8`
- `safe/src/compress/cctx.rs:11`
- `safe/src/compress/cctx_params.rs:10`
- `safe/src/compress/cdict.rs:11`
- `safe/src/compress/cstream.rs:11`
- `safe/src/compress/params.rs:10`
- `safe/src/compress/sequence_api.rs:4`
- `safe/src/compress/static_ctx.rs:10`
- `safe/src/decompress/dstream.rs:17`
- `safe/src/decompress/legacy.rs:6`
- `safe/src/dict_builder/cover.rs:4`
- `safe/src/dict_builder/fastcover.rs:4`
- `safe/src/dict_builder/zdict.rs:7`
- `safe/src/ffi/compress.rs:25`
- `safe/src/threading/pool.rs:3`
- `safe/src/threading/zstdmt.rs:3`

Each of the lines above is an `unsafe extern "C"` declaration block for linked
helper symbols or system dynamic-loader calls. Rust cannot verify the validity
of foreign symbol declarations, so these remain unsafe by definition.

- `safe/src/ffi/compress.rs:57`: `load_symbol()` is itself an `unsafe fn`
  because the caller must request the exact symbol type that matches the foreign
  definition.
- `safe/src/ffi/types.rs:42`, `safe/src/ffi/types.rs:44`,
  `safe/src/ffi/types.rs:439`: exported callback typedefs remain
  `unsafe extern "C" fn` so allocator hooks and sequence producers match the C
  ABI exactly.
- Local `type Fn = unsafe extern "C" fn(...)` aliases adjacent to the callsites
  above exist only to encode the exact foreign signature being invoked; the
  corresponding call blocks are already enumerated in the main inventory.

## Target State

The remaining unsafe surface is now restricted to:

- foreign-function declarations and calls that preserve the upstream `libzstd`
  ABI,
- allocator and opaque-handle ownership transfers across that ABI, and
- unavoidable raw buffer/out-parameter glue needed to interoperate with C
  callers.

There is no remaining `unsafe` used only for convenience.
