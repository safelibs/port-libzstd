# libzstd Rust Port

This document describes the Rust port in `safe/` as of the commit that contains this file. It is grounded in the checked-in Rust, C shim, ABI baselines, scripts, tests, Debian packaging, and current build artifacts in this repository. The upstream source snapshot under `original/libzstd-1.5.5+dfsg2/` is used only as reference material.

## High-level architecture

The port is a single Cargo workspace rooted at `safe/Cargo.toml`. `cargo metadata --manifest-path safe/Cargo.toml --format-version 1 --no-deps` reports one workspace member, package `libzstd-safe`, with library target `zstd` and crate types `cdylib`, `staticlib`, and `rlib` (`safe/Cargo.toml:11-13`). The checked-in ABI baseline has 185 public exports (`safe/src/ffi/symbols.rs:1`, `safe/abi/original.exports.txt`), and the release shared object built in `safe/target/release/libzstd.so` exports the same 185 symbols when checked with `nm -D --defined-only` and `safe/scripts/verify-export-parity.sh`.

Directory map:

```text
safe/
  Cargo.toml, build.rs        Cargo package metadata, features, crate types, and legacy C build glue.
  src/common/                 Shared frame, skippable-frame, version, allocation, and error helpers.
  src/compress/               Compression contexts, streaming, parameters, sequences, match finders, and strategies.
  src/decompress/             Decompression contexts, dictionaries, streaming, frame decode, FSE/HUF, and legacy bridge.
  src/dict_builder/           ZDICT training, cover, and fastcover-compatible entry points.
  src/ffi/                    C ABI adapters, opaque handle management, C-compatible structs, and export metadata.
  src/threading/              Thread-pool ABI surface enabled by the libzstd threading cfg.
  include/                    Checked-in upstream-compatible public headers: zstd.h, zdict.h, zstd_errors.h.
  abi/                        Checked-in ABI baseline and export ownership metadata.
  debian/                     Debian source package metadata, install manifests, rules, and autopkgtests.
  scripts/                    Build, ABI, upstream-suite, dependent, and packaging verification harnesses.
  tests/                      Rust integration tests, C API smoke tests, upstream-suite matrix, dependent matrix.
  third_party/structured-zstd Local Rust dependency used for structured zstd encode/decode logic.
```

`safe/src/lib.rs:5-11` exposes the top-level Rust modules: `common`, `compress`, `decompress`, `dict_builder`, `ffi`, and conditionally `threading`. `safe/src/lib.rs:13-45` also exposes compile-time build metadata such as the ABI version, whether threading support was enabled, and the default artifact mode.

The public boundary is the libzstd C ABI/API. Public C symbols are implemented as `#[no_mangle] pub extern "C" fn ZSTD_*` and `ZDICT_*` functions across `safe/src/common/`, `safe/src/compress/`, `safe/src/decompress/`, `safe/src/dict_builder/`, and `safe/src/threading/`. These functions translate raw C pointers, C structs from `safe/src/ffi/types.rs`, and opaque handles into Rust contexts. Internal Rust implementation code then operates on `Vec`, slices, typed context structs, and Rust error values before translating results back to libzstd-style `size_t` values and output buffers. `safe/include/zstd.h`, `safe/include/zdict.h`, and `safe/include/zstd_errors.h` are checked in and installed by the packaging/build scripts; no `cbindgen` or `bindgen` invocation is present in the current build.

Opaque C handles are represented by Rust-owned state. Compression APIs in `safe/src/ffi/compress.rs` use `EncoderContext`, `EncoderDictionary`, parameter helpers, and streaming buffers before calling into `safe/src/compress/` and the Rust dependencies. Decompression APIs in `safe/src/ffi/decompress.rs` use `DecoderContext` and `DecoderDictionary`, then call `safe/src/decompress/dctx.rs`, `safe/src/decompress/ddict.rs`, `safe/src/decompress/dstream.rs`, and `safe/src/decompress/frame.rs`. Dictionary-backed and frame-backed decode flows go through Rust dictionary parsing and frame decoding, with legacy frame support isolated behind `safe/src/decompress/legacy.rs`.

Data flow is intentionally narrow at the ABI edge:

- One-shot decompression validates the C source/destination buffers in `safe/src/ffi/decompress.rs`, resolves optional dictionary state through `DecoderContext` or `DecoderDictionary`, decodes frames through `safe/src/decompress/frame.rs`, and copies the decoded bytes back to the caller's destination pointer.
- Streaming decompression stores pending input/output state in `DecoderContext` and implements `ZSTD_decompressStream`, bufferless replay, and related query APIs through `safe/src/decompress/dstream.rs` and the FFI adapter.
- One-shot and streaming compression use `EncoderContext` in `safe/src/ffi/compress.rs`, parameters in `safe/src/compress/cctx_params.rs`, sequence/match-state modules under `safe/src/compress/`, and `structured-zstd`/`oxiarc-zstd` for Rust compression and dictionary support.
- Dictionary training and finalization enter through `safe/src/dict_builder/zdict.rs`, `safe/src/dict_builder/cover.rs`, and `safe/src/dict_builder/fastcover.rs`.
- Legacy v0.5-v0.7 frame decode is not reimplemented in safe Rust; it is routed through the internal shim in `safe/src/decompress/legacy.rs` and `safe/src/ffi/legacy_shim.c`.

`safe/build.rs` wires Cargo features into cfgs and build metadata. It enables `libzstd_threading` by default unless the static/no-MT variant features request otherwise (`safe/build.rs:36-65`), sets `LIBZSTD_THREADING`, `LIBZSTD_VARIANT_SUFFIX`, and `LIBZSTD_DEFAULT_ARTIFACT` environment values (`safe/build.rs:83-88`), and gives the `cdylib` an ELF SONAME of `libzstd.so.1` (`safe/build.rs:89`). Its only C compilation job is the legacy decode bridge: it resolves upstream legacy sources from an optional in-tree source override or from `../original/libzstd-1.5.5+dfsg2/lib` (`safe/build.rs:18-24`, `safe/build.rs:91-101`) and compiles `xxhash.c`, `zstd_v05.c`, `zstd_v06.c`, `zstd_v07.c`, and `safe/src/ffi/legacy_shim.c` with `ZSTD_LEGACY_SUPPORT=5` (`safe/build.rs:95-123`).

Cargo features are defined in `safe/Cargo.toml:23-30`: the default feature set is empty, `legacy` is a named but empty feature, and `threading`, `build-shared-default`, `build-static-default`, `variant-mt`, and `variant-nomt` drive build metadata and cfg selection. `safe/scripts/build-artifacts.sh` builds release shared and static artifacts with Cargo, installs `libzstd.so.1.5.5`, `libzstd.so.1`, `libzstd.so`, `libzstd.a`, and the checked-in headers, and generates pkg-config and CMake install files from `safe/pkgconfig/` and `safe/cmake/`.

Debian packaging lives under `safe/debian/`. `safe/debian/control:1-68` declares source package `libzstd`, binary packages `libzstd-dev`, `libzstd1`, `zstd`, and `libzstd1-udeb`, and build dependencies including `cargo`, `rustc`, `cmake`, `debhelper (>> 13.3.2~)`, `dh-package-notes`, `dpkg-build-api (= 1)`, `help2man`, `liblz4-dev`, `liblzma-dev`, `zlib1g-dev`, `less <!nocheck>`, and `python3 <!nocheck>`; the debhelper compatibility level is `14` in `safe/debian/compat`. `safe/debian/rules:46-66` drives `safe/scripts/build-artifacts.sh` and `safe/scripts/build-original-cli-against-safe.sh`, then installs the Rust-built library together with the upstream CLI built against that library. `safe/debian/tests/control:1-24` registers autopkgtest coverage for the zstd self-test and pkg-config/CMake consumer builds.

## Where the unsafe Rust lives

The maintained-source inventory was produced with `rg -n '\bunsafe\b' safe --glob '*.rs' --glob '!target/**' --glob '!out/**'`, plus `rg -n 'unsafe fn|unsafe impl|unsafe extern' safe --glob '*.rs' --glob '!target/**' --glob '!out/**'`. It includes first-party library code, Rust integration tests, and the local path dependency `safe/third_party/structured-zstd/`. Generated artifacts under `safe/target/` and `safe/out/` are excluded from the table because `safe/out/debian-src/` contains staged copies of the same Rust sources and upstream C test/source comments rather than independently maintained Rust code. A plain `grep -RIn '\bunsafe\b' safe` was still run as a cross-check; its additional matches were generated `safe/out/` copies, `safe/docs/unsafe-audit.md`, this document, upstream C comments, or the lint attribute `#![deny(unsafe_op_in_unsafe_fn)]` at `safe/src/lib.rs:1`, not separate unsafe blocks, functions, impls, or extern declarations.

### Public ABI adapters and raw C buffers

| Sites | Justification |
| --- | --- |
| `safe/src/common/frame.rs:90` | Writes a parsed `ZSTD_frameHeader` into the caller-provided `zfhPtr` out-parameter after a null check. |
| `safe/src/common/skippable.rs:36,42,72,77` | Writes the optional skippable-frame magic variant, copies payload bytes, and converts validated C source/destination buffers to Rust slices for the skippable-frame API. |
| `safe/src/compress/cctx.rs:135` | Stores a queried compression parameter through the caller's optional `int*` output pointer. |
| `safe/src/compress/cctx_params.rs:262` | Stores a queried `ZSTD_CCtx_params` value through the caller's optional `int*` output pointer. |
| `safe/src/compress/cstream.rs:253,258,261,264` | Reads and writes `srcPos`/`dstPos` pointers for `ZSTD_compressStream2_simpleArgs`. |
| `safe/src/decompress/dctx.rs:304` | Stores a queried decompression parameter through the caller's validated `int*` output pointer. |
| `safe/src/decompress/dstream.rs:75,169,175,179` | Reborrows caller-provided `ZSTD_inBuffer`/`ZSTD_outBuffer` pointers and reads/writes simple-args position pointers. |
| `safe/src/dict_builder/cover.rs:108`; `safe/src/dict_builder/fastcover.rs:138` | Mutably borrows caller-provided dictionary-training parameter structs so optimization results can be returned through the C API. |
| `safe/src/dict_builder/zdict.rs:106,117,232` | Converts checked sample buffers and sample-size arrays into Rust slices for `ZDICT_*` training/finalization APIs. |
| `safe/src/ffi/compress.rs:70,958,965,3012,3024,3058,3069,3076,3515,3566` | Converts by-reference dictionaries, input/output buffers, external-sequence arrays, and streaming buffers between C pointers and Rust slices or element writes. |
| `safe/src/ffi/decompress.rs:46,265,650,1010,1127` | Converts by-reference dictionaries and input buffers to slices, copies staged decoded output to caller memory, and computes an in-bounds streaming output pointer. |

### Opaque handles, static workspaces, and ownership casts

| Sites | Justification |
| --- | --- |
| `safe/src/compress/cctx_params.rs:153,160,195` | Casts opaque `ZSTD_CCtx_params` pointers back to `CCtxParamsState` and drops boxed state allocated by the Rust constructor. |
| `safe/src/ffi/compress.rs:972,979,986,1025,1039,1163,1164,1203,1216` | Casts `ZSTD_CCtx`/`ZSTD_CDict` handles to Rust state, initializes static workspaces in place, copies static dictionary bytes into caller workspace, and drops heap-owned handles. |
| `safe/src/ffi/decompress.rs:658,666,674,705,723,761,762,774,788` | Casts `ZSTD_DCtx`/`ZSTD_DDict` handles to Rust state, initializes static workspaces in place, copies static dictionary bytes into caller workspace, and drops heap-owned handles. |
| `safe/src/threading/pool.rs:31,58` | Casts the opaque thread-pool handle back to `ThreadPoolState` and drops the boxed pool allocated by `ZSTD_createThreadPool`. |

### Callback and external function ABI

| Sites | Justification |
| --- | --- |
| `safe/src/ffi/types.rs:42,44` | Defines upstream-compatible custom allocator callback types as `unsafe extern "C" fn` because Rust cannot verify arbitrary C callback contracts. |
| `safe/src/ffi/types.rs:439` | Defines the upstream sequence-producer callback type as `unsafe extern "C" fn` to match the published advanced compression ABI. |
| `safe/src/ffi/compress.rs:3435` | Calls the registered sequence-producer callback after building the C argument list from the current compression context. |
| `safe/src/decompress/legacy.rs:12,48,52,61,67,76,88,113,131,154` | Declares and calls the internal C legacy-decode shim for v0.5-v0.7 frame compatibility, passing only slice-backed pointers or shim-owned stream contexts. |

### First-party compression/decompression internals

These sites are not just nullable C parameter conversion. They maintain zstd algorithm state, emit compatibility structures, dispatch through unsafe function pointers, or perform raw buffer work where surrounding code enforces bounds and lifetime invariants.

| Sites | Justification |
| --- | --- |
| `safe/src/compress/frame.rs:15` | Converts a validated destination pointer to a slice before writing the final empty block marker. |
| `safe/src/compress/literals.rs:28,29,56,57,82` | Converts literal-block input/output pointers to slices after capacity/null validation in the literal-copy/RLE helpers. |
| `safe/src/compress/ldm.rs:32,46,56,59,60,100,108,113,137,143,153,159,181,184` | Reborrows long-distance-match state, sequence stores, parameter structs, source bytes, and repcode arrays from upstream-compatible raw structures. |
| `safe/src/compress/match_state.rs:77,92,98,106,116,232,246,258,306,311,319,326,342` | Stores unsafe block-compressor function pointers and analyzes raw block input/repcodes through pointer-derived slices while updating match and sequence state. |
| `safe/src/compress/sequence_api.rs:73` | Converts an external sequence array to a mutable slice for in-place delimiter merging after null/length checks. |
| `safe/src/compress/sequences.rs:59,70,113,114,131,136` | Reborrows repeat-mode, frequency, destination, sequence, and sequence-store pointers for upstream-compatible sequence encoding helpers. |
| `safe/src/compress/compat.rs:73,77,102,119,122,142` | Converts FSE/HUF compatibility buffers and count tables to slices before filling normalized counters and placeholder tables. |
| `safe/src/compress/strategies/fast.rs:14,28,35,38,45,48,55`; `safe/src/compress/strategies/double_fast.rs:14,27,34,37,44,47,54` | Reborrows match state for hash-table updates and exposes unsafe strategy entry points that delegate to the shared block analyzer. |
| `safe/src/compress/strategies/lazy.rs:8,15,32,39,56,63,80,87,104,111,128,135,152,159,176,183,200,207,224,231,248,255,272,279` | Exposes lazy/greedy strategy entry points whose callers must provide valid match-state, sequence-store, repcode, and source pointers. |
| `safe/src/compress/strategies/opt.rs:9,17,24,41,48,65,72,89,96,113,120,137,144,161,168` | Reborrows binary-tree strategy state and exposes optimal-strategy entry points with the same raw-pointer contract as the other block compressors. |
| `safe/src/decompress/frame.rs:982` | Copies decoded frame bytes into the caller's destination pointer after size and null checks. |

### Local path dependency unsafe

`structured-zstd` is vendored under `safe/third_party/structured-zstd/`, so its unsafe code is part of this source tree even though it is not the public libzstd C ABI adapter.

| Sites | Justification |
| --- | --- |
| `safe/third_party/structured-zstd/src/decoding/decode_buffer.rs:90,122` | Calls the ring buffer's unchecked repeat-copy primitive after validating match ranges and reserving capacity. |
| `safe/third_party/structured-zstd/src/decoding/ringbuffer.rs:23,26` | Marks `RingBuffer` as `Send` and `Sync` based on its ownership model and absence of unsynchronized interior mutability. |
| `safe/third_party/structured-zstd/src/decoding/ringbuffer.rs:74,94,104,129,142,169,217,225,259,281,292,313,320,327,345,357,379,386,393,412,419,426,450,457,472,475,538,540,560,579,613,623,639,649,673,683,778,985,1020,1035,1047` | Implements manual allocation, deallocation, pointer arithmetic, wraparound copying, unchecked internal repeat-copy functions, and raw slice views for the decoder ring buffer. |

### Rust integration tests

| Sites | Justification |
| --- | --- |
| `safe/tests/rust/compress.rs:277,296` | Converts C error-name pointers returned by exported ABI functions into `CStr` for assertion messages. |
| `safe/tests/rust/decompress.rs:39,57,119` | Converts C error-name and version-string pointers returned by exported ABI functions into `CStr` for assertions. |

Unsafe code that is not required merely by the original public C ABI/API boundary is concentrated in `safe/src/compress/frame.rs`, `safe/src/compress/literals.rs`, `safe/src/compress/ldm.rs`, `safe/src/compress/match_state.rs`, `safe/src/compress/sequence_api.rs`, `safe/src/compress/sequences.rs`, `safe/src/compress/compat.rs`, the files under `safe/src/compress/strategies/`, `safe/src/decompress/frame.rs:982`, and the vendored `structured-zstd` decoder buffer/ring buffer. The legacy bridge in `safe/src/decompress/legacy.rs` is required for compatibility with legacy frame formats, but it is also the only remaining first-party foreign-function bridge beyond a pure Rust implementation.

## Remaining unsafe FFI beyond the original ABI/API boundary

The port's intended public FFI boundary is the original libzstd C ABI/API exposed through the 185 `ZSTD_*` and `ZDICT_*` exports and the checked-in headers under `safe/include/`. Source-level evidence from `rg -n 'extern "C"' safe/src safe/include safe/tests safe/build.rs --glob '!target/**' --glob '!out/**'` shows that the first-party Rust sources contain public libzstd exports, the original ABI callback typedefs in `safe/src/ffi/types.rs:42,44,439`, and one non-public `unsafe extern "C"` block in `safe/src/decompress/legacy.rs:12`. `rg -n 'libc::|dlopen|dlsym|syscall|pthread_|mmap|malloc\(' safe/src safe/build.rs safe/Cargo.toml --glob '!target/**' --glob '!out/**'` found no direct Rust source calls to libc, OS syscalls, pthreads, mmap, malloc, or dynamic loading APIs.

| Surface | Symbols | Provider | Why it remains | Plausible safe-Rust replacement |
| --- | --- | --- | --- | --- |
| Legacy frame decode shim | Rust declares `libzstd_safe_legacy_support`, `libzstd_safe_is_legacy`, `libzstd_safe_get_decompressed_size_legacy`, `libzstd_safe_decompress_legacy`, `libzstd_safe_find_frame_compressed_size_legacy`, `libzstd_safe_find_decompressed_bound_legacy`, `libzstd_safe_free_legacy_stream`, `libzstd_safe_init_legacy_stream`, and `libzstd_safe_decompress_legacy_stream` in `safe/src/decompress/legacy.rs:12-40`. The C shim forwards to upstream legacy symbols such as `ZSTD_isLegacy`, `ZSTD_decompressLegacy`, and `ZSTD_decompressLegacyStream` in `safe/src/ffi/legacy_shim.c:3-55`. | `safe/src/ffi/legacy_shim.c`, plus upstream `xxhash.c`, `zstd_v05.c`, `zstd_v06.c`, and `zstd_v07.c` selected by `safe/build.rs:95-123` and compiled into the library through the `cc` crate. | libzstd preserves decode compatibility for v0.5-v0.7 legacy frames. The Rust port detects and delegates those frames through `safe/src/decompress/legacy.rs`, while modern frame decode stays in Rust. | A native Rust implementation of the v0.5-v0.7 legacy frame decoders and xxhash compatibility code would remove this C bridge. |
| ELF/POSIX runtime imports in the built shared object | `nm -D --undefined-only safe/target/release/libzstd.so` shows weak ELF hooks `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__cxa_thread_atexit_impl`, and `__gmon_start__`; libc/glibc imports `__errno_location`, `__stack_chk_fail`, `__tls_get_addr`, `__xpg_strerror_r`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, weak `getrandom`, weak `gettid`, `lseek64`, `malloc`, `memcmp`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `nanosleep`, `open64`, `poll`, `posix_memalign`, `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, weak `statx`, `strlen`, `syscall`, `write`, and `writev`; libm import `log2`; libgcc unwinder imports `_Unwind_Backtrace`, `_Unwind_DeleteException`, `_Unwind_GetDataRelBase`, `_Unwind_GetIP`, `_Unwind_GetIPInfo`, `_Unwind_GetLanguageSpecificData`, `_Unwind_GetRegionStart`, `_Unwind_GetTextRelBase`, `_Unwind_RaiseException`, `_Unwind_Resume`, `_Unwind_SetGR`, and `_Unwind_SetIP`. `objdump -p safe/target/release/libzstd.so` records `NEEDED` entries for `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and `ld-linux-x86-64.so.2`. | Rust `std` and allocator/runtime support, glibc/POSIX, libm, libgcc unwinding support, the dynamic loader, compiler stack-protector/runtime hooks, and the statically linked legacy C bridge. | The crate builds a normal Linux ELF `cdylib`/`staticlib`, uses `std`, and links a small C legacy decoder island. These are runtime/library imports, not hand-written Rust calls to extra C libraries. | A `no_std` or `panic = "abort"` design, a different allocator/runtime strategy, and a native Rust legacy decoder could reduce these imports, but that would be a separate portability/package-design effort. |

No upstream dynamic fallback or plugin-loading surface remains. `rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe --glob '!target/**' --glob '!out/**' --glob '!PORT.md'` produced no matches, and `cargo tree --manifest-path safe/Cargo.toml` shows no direct dependency that links a third-party C/C++ compression library. The callback types in `safe/src/ffi/types.rs:42,44,439` are part of the original ABI surface, not extra FFI. Custom allocation callbacks are currently accepted only in the upstream-compatible type definitions; the implementation supports the default/null allocator path. The sequence-producer callback can be invoked through the original advanced compression API at `safe/src/ffi/compress.rs:3435`.

## Remaining issues

- Custom memory allocators are not fully implemented. The ABI types for `ZSTD_allocFunction`, `ZSTD_freeFunction`, and `ZSTD_customMem` exist in `safe/src/ffi/types.rs:42-75`, but the implementation accepts only the default/null allocator configuration in `safe/src/decompress/dctx.rs:14-15`, `safe/src/decompress/ddict.rs:14-15`, `safe/src/decompress/dstream.rs:27-28`, and `safe/src/ffi/compress.rs:3772-3773`; non-null allocator callbacks make the advanced create/load functions return null or an error path, for example `safe/src/decompress/dctx.rs:355-359`, `safe/src/decompress/ddict.rs:143-152`, and `safe/src/compress/cctx.rs:354-357`.
- Legacy v0.5-v0.7 decode still relies on C sources through `safe/src/ffi/legacy_shim.c` and `safe/src/decompress/legacy.rs`. This preserves compatibility, but it leaves a small non-Rust decoder island compiled by `safe/build.rs:96-117`.
- The root crate advertises `rust-version = "1.82"` in `safe/Cargo.toml:5`, but the resolved dependency graph requires a newer toolchain: `cargo metadata` reports `oxiarc-core` and `oxiarc-zstd` with `rust_version = "1.85"` and the local `structured-zstd` manifest records `rust-version = "1.92"` in `safe/third_party/structured-zstd/Cargo.toml:14`. The checked-in `safe/rust-toolchain.toml` selects `stable`, and this refresh used `rustc 1.94.0`; `safe/debian/control:7-18` does not pin a minimum Rust compiler version beyond `rustc`.
- The local `structured-zstd` dependency has unfinished or invariant-heavy internals. The current marker scan found executable `unimplemented!()` branches in `safe/third_party/structured-zstd/src/encoding/frame_compressor.rs:256,408` and TODO comments in `safe/third_party/structured-zstd/src/encoding/blocks/compressed.rs:538`, `safe/third_party/structured-zstd/src/encoding/frame_compressor.rs:271`, `safe/third_party/structured-zstd/src/encoding/frame_header.rs:44,74`, `safe/third_party/structured-zstd/src/dictionary/mod.rs:45`, `safe/third_party/structured-zstd/src/huff0/huff0_encoder.rs:117,201,322`, `safe/third_party/structured-zstd/src/bit_io/bit_writer.rs:384`, `safe/third_party/structured-zstd/src/tests/mod.rs:100`, `safe/third_party/structured-zstd/src/fse/fse_encoder.rs:362`, `safe/third_party/structured-zstd/src/decoding/block_decoder.rs:27`, `safe/third_party/structured-zstd/src/decoding/decode_buffer.rs:105,186`, `safe/third_party/structured-zstd/src/decoding/streaming_decoder.rs:39,117`, `safe/third_party/structured-zstd/src/decoding/ringbuffer.rs:93,200,240`, and `safe/third_party/structured-zstd/src/decoding/sequence_section_decoder.rs:467`. The public compression adapter currently maps requested levels through `safe/src/ffi/compress.rs:1724-1728`, avoiding the unimplemented compression-level branches in normal ABI use. No first-party `TODO`, `FIXME`, `todo!`, `unimplemented!`, or `panic!("TODO")` markers were found under `safe/src`, `safe/tests`, `safe/scripts`, or `safe/debian`; the actionable markers are in the vendored `safe/third_party/structured-zstd/` source.
- This documentation refresh did not rerun the whole `safe/scripts/run-full-suite.sh`. It reran `cargo build --manifest-path safe/Cargo.toml --release`, `cargo test --manifest-path safe/Cargo.toml --release --all-targets`, `safe/scripts/verify-baseline-contract.sh`, `safe/scripts/verify-export-parity.sh`, and both dependent matrix entry points. `bash safe/scripts/run-dependent-matrix.sh --compile-only` compiled all 12 probes, and `bash safe/scripts/run-dependent-matrix.sh --runtime-only` reported all 12 dependent runtime tests passing in `safe/out/dependents/logs/runtime.log`. The top-level gate still includes broader upstream, packaging, CLI-permission, performance, and downstream coverage through `safe/scripts/run-full-suite.sh:48-76`.
- Some release-gate coverage remains intentionally host-dependent. `safe/scripts/run-upstream-tests.sh:173-248` skips 32-bit, sanitizer, or valgrind subcases when the local host lacks support, and it has a known valgrind fuzzer-smoke skip for upstream fuzzer worker-parameter behavior. `safe/scripts/run-pzstd-tests.sh:190-197` has sanitizer-runtime skip handling, `safe/scripts/run-zlibwrapper-tests.sh:80-91` allows documented zlib 1.3 expectation mismatches, and `safe/scripts/run-upstream-fuzz-tests.sh:132-163` permits bounded fuzz-driver timeouts/failures as long as at least one driver passes. The stale log `safe/out/phase6/run-full-suite-final.log` still stops in the old upstream valgrind fuzzer-smoke area; run `bash safe/scripts/run-full-suite.sh` for a fresh full-release signal instead of treating that file as the latest success artifact.
- Performance coverage is a smoke threshold, not a full benchmark against upstream. `safe/scripts/run-performance-smoke.sh:49-53` builds a 16 MiB corpus from checked-in fixtures, and `safe/scripts/run-performance-smoke.sh:70-73` enforces default minimum throughput thresholds of 1.0 MiB/s compression and 2.0 MiB/s decompression with 15 second maximums. No checked-in report records bit-for-bit speed parity or compression-ratio parity across the full upstream benchmark matrix.
- Dependent coverage is representative, not exhaustive. `dependents.json:1-260` lists a curated Ubuntu 24.04 set of 12 packages; `safe/tests/dependents/dependent_matrix.toml:19-113` maps each source package to a C compile probe and one runtime test. `safe/docker/dependents/entrypoint.sh:48-113` checks inventory consistency, `safe/docker/dependents/entrypoint.sh:116-147` verifies that the safe `libzstd1`, `libzstd-dev`, and `zstd` packages are installed, and `safe/docker/dependents/entrypoint.sh:149-164` checks that each exercised runtime binary resolves `libzstd.so.1` to the installed safe library. The host-side runner requires the checked image metadata (`safe/scripts/run-dependent-matrix.sh:66-83`), uses only log and compile-output bind mounts (`safe/scripts/run-dependent-matrix.sh:122-130`), and adds `--privileged --tmpfs /run --tmpfs /run/lock` for runtime tests (`safe/scripts/run-dependent-matrix.sh:133-135`). The image is built from `ubuntu:24.04` (`safe/docker/dependents/Dockerfile:1-2`) and installs the downstream tool set in `safe/docker/dependents/Dockerfile:9-41`; `safe/scripts/build-dependent-image.sh:38-52` stages `dependents.json`, the matrix fixtures, helper scripts, safe Debian packages, and `safe/out/dependents/image-context/metadata.env`.

Downstream coverage and caveats:

| Source package | Binary/runtime surface | Compile probe | Runtime coverage | Caveat |
| --- | --- | --- | --- | --- |
| `apt` | `libapt-pkg6.0t64` via `apt` | `safe/tests/dependents/src/apt_probe.c` | `test_apt` in `safe/docker/dependents/entrypoint.sh:176-249` builds a zstd-compressed `.deb`, serves `Packages.zst`, runs `apt-get update`, and checks that APT fetched the `.zst` metadata. | Exercises a local unsigned test repository, not mirror authentication, proxying, or all apt compression settings. |
| `dpkg` | `dpkg`/`dpkg-deb` | `safe/tests/dependents/src/dpkg_probe.c` | `test_dpkg` in `safe/docker/dependents/entrypoint.sh:251-274` builds, inspects, extracts, and compares a `.deb` with zstd members. | Covers `dpkg-deb` package-member handling, not a full package install transaction. |
| `rsync` | `rsync` | `safe/tests/dependents/src/rsync_probe.c` | `test_rsync` in `safe/docker/dependents/entrypoint.sh:276-304` starts a local daemon and transfers a file with `--compress-choice=zstd`. | Covers a loopback daemon/client transfer, not WAN behavior or protocol-version negotiation breadth. |
| `systemd` | `libsystemd-shared` via `systemd-journald` | `safe/tests/dependents/src/systemd_probe.c` | `test_systemd` in `safe/docker/dependents/entrypoint.sh:306-363` starts journald, writes a large field through `systemd-cat`, verifies journal readback, and checks for zstd frame magic in the journal file. | Requires the privileged runtime container and writable `/run`; it does not exercise coredump compression. |
| `libarchive` | `libarchive13t64` via `bsdtar` | `safe/tests/dependents/src/libarchive_probe.c` | `test_libarchive` in `safe/docker/dependents/entrypoint.sh:365-395` creates/extracts `.tar.zst` archives and checks selective extraction. | Uses `bsdtar`; applications embedding libarchive are represented only through this CLI path. |
| `btrfs-progs` | `btrfs-progs` | `safe/tests/dependents/src/btrfs-progs_probe.c` | `test_btrfs` in `safe/docker/dependents/entrypoint.sh:397-428` creates loopback filesystems, mounts with `compress=zstd`, sends compressed extents, receives with force-decompress, and compares data. | Requires privileged container access, loop devices, and mounts; it is a smoke test rather than broad filesystem coverage. |
| `squashfs-tools` | `squashfs-tools` | `safe/tests/dependents/src/squashfs-tools_probe.c` | `test_squashfs` in `safe/docker/dependents/entrypoint.sh:430-446` creates a zstd Squashfs image and extracts it with `unsquashfs`. | Covers a tiny image only, not large dictionaries, fragments, or all Squashfs compressor options. |
| `qemu` | `qemu-utils` via `qemu-img` | `safe/tests/dependents/src/qemu_probe.c` | `test_qemu` in `safe/docker/dependents/entrypoint.sh:448-462` converts raw data to compressed qcow2 with `compression_type=zstd`, checks image metadata, and round-trips back to raw. | Exercises `qemu-img` utilities, not running virtual machines or block backends under guest I/O. |
| `curl` | `libcurl4t64` via `curl` | `safe/tests/dependents/src/curl_probe.c` | `test_curl` in `safe/docker/dependents/entrypoint.sh:464-503` serves a `Content-Encoding: zstd` response and checks `curl --compressed` decodes it. | Uses the curl CLI against a local HTTP server; embedders of libcurl are represented through the CLI path. |
| `tiff` | `libtiff6` via `tiffcp`/`tiffinfo`/`tiffcmp` | `safe/tests/dependents/src/tiff_probe.c` | `test_tiff` in `safe/docker/dependents/entrypoint.sh:505-521` writes a zstd-compressed TIFF and verifies metadata plus pixel equality. | Covers a small RGB image, not tiled/striped variants or every TIFF zstd option. |
| `rpm` | `rpm`, `rpmbuild`, and `rpm2cpio` | `safe/tests/dependents/src/rpm_probe.c` | `test_rpm` in `safe/docker/dependents/entrypoint.sh:523-565` uses checked fixtures under `safe/tests/dependents/fixtures/rpm/`, builds a `w19.zstdio` RPM, checks `%{PAYLOADCOMPRESSOR}` is `zstd`, extracts with `rpm2cpio`, and compares the payload. | Covers a tiny noarch RPM payload and the Ubuntu Noble `librpmio9t64 -> libzstd1` path, not full RPM database operations. |
| `zarchive` | `zarchive-tools` via `zarchive` | `safe/tests/dependents/src/zarchive_probe.c` | `test_zarchive` in `safe/docker/dependents/entrypoint.sh:567-580` uses checked fixtures under `safe/tests/dependents/fixtures/zarchive/`, creates a `.za` archive, extracts it, and diffs the tree. | Covers a small directory round trip through `libzarchive0.1`, not large archives or every zarchive mode. |

`safe/out/dependents/logs/compile.log` records all 12 probes compiled against `libzstd-dev 1.5.5+dfsg2-2build1.1+safelibs1`; `safe/out/dependents/logs/runtime.log` records all 12 runtime tests passing. Packages outside this curated set are not covered by checked-in dependent evidence.
- `relevant_cves.json` contains two relevant records, CVE-2021-24031 and CVE-2021-24032, both for zstd CLI output-file permissions rather than the core library. `safe/scripts/check-cli-permissions.sh:29-52` requires those two CVEs to be present, and `safe/scripts/check-cli-permissions.sh:73-132` audits atomic output-file creation with `strace`. That CLI-permission gate was not rerun for this documentation-only change; these CVEs are covered by the CLI gate, not by the Rust library implementation alone.
- No repository-level upgrade report file is present.

## Dependencies and other libraries used

Direct Cargo dependencies from `safe/Cargo.toml:41-47`:

| Dependency | Version | Kind | Purpose and safety notes |
| --- | --- | --- | --- |
| `oxiarc-core` | `=0.2.5` | normal | Provides shared OxiArc error/types used when mapping Rust zstd decode failures, including `OxiArcError` in `safe/src/decompress/frame.rs:10,367`. The external crate does not declare `#![forbid(unsafe_code)]`; `rg` found unsafe CRC/SIMD and optional mmap implementation sites in the registry source, but this port enables only its default feature set through `cargo tree -e features` and relies on the local ABI/release gates for coverage. |
| `oxiarc-zstd` | `=0.2.5` | normal | Provides Rust zstd encode/decode helpers used for fallback dictionary/frame decode (`safe/src/decompress/frame.rs:497-500,596-598`) and dictionary-builder encoding (`safe/src/dict_builder/zdict.rs:1049`). `rg -n '\bunsafe\b'` found no unsafe matches in the resolved `oxiarc-zstd-0.2.5/src` tree, but the crate does not declare `#![forbid(unsafe_code)]`. |
| `structured-zstd` | `=0.0.3`, path `third_party/structured-zstd` | normal | Local structured zstd encoder/decoder used for frame parsing, dictionary-aware decode, streaming/block compression, and frame emission (`safe/src/ffi/compress.rs:27-28`, `safe/src/ffi/decompress.rs:19`, `safe/src/decompress/frame.rs:12`). Its remaining unsafe sites are inventoried above because the source is vendored in this repository; it also enables the `hash` feature, which pulls in `twox-hash`. |
| `cc` | `1.2` in `Cargo.toml`, resolved to `1.2.58` in `safe/Cargo.lock:5-13` | build | Build-time helper used by `safe/build.rs:111-123` to compile the legacy C shim and upstream legacy decoder sources. Its own implementation contains unsafe code in the registry source, but it is a build tool and is not linked into `libzstd.so`. |

`safe/Cargo.lock` resolves the normal transitive dependencies to `thiserror 2.0.18`, `thiserror-impl 2.0.18`, `proc-macro2 1.0.106`, `quote 1.0.45`, `syn 2.0.117`, `unicode-ident 1.0.24`, and `twox-hash 2.1.2`; the `cc` build dependency pulls in `find-msvc-tools 0.1.9` and `shlex 1.3.0` (`safe/Cargo.lock:49-121`). `twox-hash` contains unsafe fixed-buffer and SIMD/hash implementation code in the registry source, but it is a Rust hashing dependency reached only through `structured-zstd`'s `hash` feature; no direct dependency links a third-party C/C++ compression library according to `cargo tree --manifest-path safe/Cargo.toml`.

Build behavior:

- `safe/Cargo.toml:11-13` builds library target `zstd` as `cdylib`, `staticlib`, and `rlib`; `safe/Cargo.toml:23-30` defines feature flags for threaded/default/variant builds.
- `safe/build.rs:41-65` maps features to `libzstd_threading` and artifact cfgs, `safe/build.rs:83-89` emits `LIBZSTD_*` metadata and `-Wl,-soname,libzstd.so.1`, and `safe/build.rs:91-123` compiles the legacy C bridge.
- `safe/scripts/build-artifacts.sh:262-281` builds shared and static artifacts with `cargo rustc`, installs `libzstd.so.1.5.5`, `libzstd.so.1`, `libzstd.so`, and `libzstd.a`, and `safe/scripts/build-artifacts.sh:283-363` installs the checked-in headers plus generated pkg-config and CMake metadata from `safe/pkgconfig/` and `safe/cmake/`.
- `safe/scripts/build-original-cli-against-safe.sh:324-356` installs safe headers/libraries into the helper tree, writes `libzstd.a` as an `INPUT ( libzstd.so )` linker indirection, builds upstream `programs/` and `contrib/pzstd/`, and verifies that `zstd` and `pzstd` resolve libzstd from the helper root.
- `safe/scripts/build-deb.sh:227-246` stages the safe Rust sources, `third_party/`, scripts, pkg-config/CMake templates, Debian metadata, `Cargo.toml`, `build.rs`, `Cargo.lock`, and `rust-toolchain.toml`; `safe/scripts/build-deb.sh:248-296` stages upstream helper source trees and docs needed by Debian metadata; `safe/scripts/build-deb.sh:298-365` runs `dpkg-buildpackage`, optionally assembles the udeb, extracts `.deb` contents into the profile-specific stage root under `safe/out/deb/`, and writes `metadata.env`.

Build-time tools and packaging dependencies:

- Debian source Build-Depends are `cargo`, `cmake (>= 3.24~)`, `debhelper (>> 13.3.2~)`, `dh-package-notes`, `dpkg-build-api (= 1)`, `help2man`, `liblz4-dev`, `liblzma-dev`, `rustc`, `zlib1g-dev`, `less <!nocheck>`, and `python3 <!nocheck>` (`safe/debian/control:7-18`); debhelper compatibility level is `14` in `safe/debian/compat`.
- `safe/debian/rules:46-66` delegates library and CLI builds to the safe scripts. `safe/debian/rules:97-137` runs the shlibs/debhelper steps and generates `zstdmt.1` and `pzstd.1` when docs are enabled.
- Autopkgtests require the built packages plus `build-essential`, `pkgconf`, `cmake`, `python3`, `python3-click`, and `python3-typedload` as declared in `safe/debian/tests/control:1-24`. The checked-in Python helper requirement files also list `click`, `typedload`, `tomli` for older Python, and `pytest` under `safe/debian/tests/requirements/`.
- Downstream dependent validation is image-based. `safe/docker/dependents/Dockerfile:1-2` pins the default base image to `ubuntu:24.04`; `safe/docker/dependents/Dockerfile:9-41` installs `apt`, `apt-utils`, `btrfs-progs`, `build-essential`, `ca-certificates`, `cmake`, `cpio`, `curl`, `debhelper`, `devscripts`, `dh-package-notes`, `dpkg-dev`, `fakeroot`, `file`, `help2man`, `jq`, `libarchive-tools`, `liblz4-dev`, `liblzma-dev`, `libtiff-tools`, `less`, `pkgconf`, `python3`, `python3-pil`, `qemu-utils`, `rpm`, `rsync`, `squashfs-tools`, `systemd`, `zarchive-tools`, `zlib1g-dev`, and `zstd`. Those packages are validation dependencies only; they are not linked into the Rust `libzstd` library.
- The shell scripts also invoke normal Debian/build tools such as `bash`, `python3`, `rsync`, `make`, `dpkg-architecture`, `dpkg-buildpackage`, `fakeroot`, `dpkg-deb`, `ldd`, `nm`, and `objdump`. Some of these are implicit in the Debian build environment rather than explicit `Build-Depends`.

The Rust library build does not use `bindgen`, `cbindgen`, `pkg-config`, or a third-party C/C++ compression library. The built shared object currently links to normal system runtime libraries (`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and the dynamic loader as shown by `ldd` and `objdump -p safe/target/release/libzstd.so`). The only C code intentionally compiled into the Rust library is the legacy decode bridge listed in `safe/build.rs:95-101`: upstream `xxhash.c`, `zstd_v05.c`, `zstd_v06.c`, `zstd_v07.c`, and `safe/src/ffi/legacy_shim.c`.

## How this document was produced

Commands run or consulted for this documentation refresh:

```sh
git status --short
test -f safe/PORT.md
grep -n '^## High-level architecture\|^## Where the unsafe Rust lives\|^## Remaining unsafe FFI beyond the original ABI/API boundary\|^## Remaining issues\|^## Dependencies and other libraries used\|^## How this document was produced' safe/PORT.md
sed -n '1,320p' safe/PORT.md
nl -ba safe/PORT.md | sed -n '45,290p'
sed -n '1,260p' dependents.json
sed -n '1,260p' safe/tests/dependents/dependent_matrix.toml
find safe/tests/dependents -maxdepth 4 -type f | sort
sed -n '1,260p' test-original.sh
sed -n '1,260p' safe/scripts/build-dependent-image.sh
sed -n '1,320p' safe/scripts/run-dependent-matrix.sh
sed -n '1,760p' safe/docker/dependents/entrypoint.sh
sed -n '1,260p' safe/docker/dependents/Dockerfile
sed -n '1,260p' safe/scripts/check-dependent-compile-compat.sh
sed -n '1,560p' safe/scripts/verify-baseline-contract.sh
sed -n '1,220p' safe/Cargo.toml
sed -n '1,180p' safe/build.rs
sed -n '1,80p' safe/src/lib.rs
sed -n '1,260p' relevant_cves.json
nl -ba safe/src/decompress/legacy.rs | sed -n '1,180p'
nl -ba safe/src/ffi/legacy_shim.c | sed -n '1,80p'
sed -n '1,280p' safe/scripts/run-upstream-tests.sh
sed -n '1,220p' safe/scripts/run-performance-smoke.sh
sed -n '1,180p' safe/scripts/check-cli-permissions.sh
find safe/debian -maxdepth 3 -type f | sort
sed -n '1,180p' safe/debian/control
find safe/tests -maxdepth 4 -type f | sort
rg -n 'TODO|FIXME|todo!|unimplemented!|panic!\("TODO' safe/src safe/tests safe/scripts safe/debian safe/third_party/structured-zstd --glob '!target/**' --glob '!out/**'
cargo metadata --manifest-path safe/Cargo.toml --format-version 1 --no-deps
cargo metadata --manifest-path safe/Cargo.toml --format-version 1
jq -r '.packages[] | select(.name=="libzstd-safe" or .name=="oxiarc-core" or .name=="oxiarc-zstd" or .name=="structured-zstd" or .name=="twox-hash" or .name=="cc") | "\(.name) \(.version) rust_version=\(.rust_version // "none") manifest=\(.manifest_path)"' < <(cargo metadata --manifest-path safe/Cargo.toml --format-version 1)
cargo tree --manifest-path safe/Cargo.toml
cargo tree --manifest-path safe/Cargo.toml -e features
if command -v cargo-geiger >/dev/null; then cargo geiger --manifest-path safe/Cargo.toml; else echo cargo-geiger unavailable; fi
rustc --version
cargo --version
sed -n '1,140p' safe/Cargo.lock
sed -n '1,120p' safe/third_party/structured-zstd/Cargo.toml
rg -n '\bunsafe\b' safe --glob '*.rs' --glob '!target/**' --glob '!out/**'
rg -n 'unsafe fn|unsafe impl|unsafe extern' safe --glob '*.rs' --glob '!target/**' --glob '!out/**'
grep -RIn '\bunsafe\b' safe
grep -RIn 'unsafe fn\|unsafe impl\|unsafe extern' safe
grep -RIn 'extern "C"' safe
find safe/out -type f -name '*.rs' -print
rg -n 'extern "C"' safe/src safe/include safe/tests safe/build.rs --glob '!target/**' --glob '!out/**'
grep -RIn 'libc::\|dlopen\|dlsym\|syscall\|pthread\|mmap\|malloc\|free' safe
rg -n 'libc::|dlopen|dlsym|syscall|pthread_|mmap|malloc\(' safe/src safe/build.rs safe/Cargo.toml --glob '!target/**' --glob '!out/**'
rg -n '\bunsafe\b|forbid\(unsafe_code\)' /home/yans/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/oxiarc-core-0.2.5/src /home/yans/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/oxiarc-zstd-0.2.5/src /home/yans/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/twox-hash-2.1.2/src /home/yans/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cc-1.2.58/src
rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe --glob '!target/**' --glob '!out/**' --glob '!PORT.md'
rg -n 'oxiarc|OxiArc|structured_zstd|structured-zstd|twox|cc::|cc::Build|Build::new' safe/src safe/build.rs safe/third_party/structured-zstd/Cargo.toml --glob '!target/**' --glob '!out/**'
rg -n 'bindgen|cbindgen|pkg-config|pkgconf' safe --glob '!target/**' --glob '!out/**' --glob '!PORT.md'
rg -n 'extern "C"|unsafe extern' safe/src/decompress/legacy.rs safe/src/ffi/legacy_shim.c safe/build.rs
nm -D --defined-only safe/target/release/libzstd.so
nm -D --undefined-only safe/target/release/libzstd.so
nm -D --defined-only safe/target/release/libzstd.so | awk '{print $3}' | sed 's/@@.*//' | sort | wc -l
diff -u <(sed '1,2d' safe/abi/original.exports.txt | awk '{print $1}' | sort) <(nm -D --defined-only safe/target/release/libzstd.so | awk '{print $3}' | sed 's/@@.*//' | sort)
objdump -p safe/target/release/libzstd.so
ldd safe/target/release/libzstd.so
cargo build --manifest-path safe/Cargo.toml --release
find safe/target safe/out -type f \( -name 'libzstd*.so*' -o -name 'libzstd*.a' \) 2>/dev/null
cargo test --manifest-path safe/Cargo.toml --release --all-targets
bash safe/scripts/verify-baseline-contract.sh
bash safe/scripts/verify-export-parity.sh
bash safe/scripts/run-dependent-matrix.sh --compile-only
bash safe/scripts/run-dependent-matrix.sh --runtime-only
jq -r '.packages | length' dependents.json
jq -r '.packages[] | [.source_package,.binary_package] | @tsv' dependents.json
jq -r '.relevant_cves[] | .cve_id' relevant_cves.json
find safe/out/dependents/stamps safe/out/dependents/logs -maxdepth 1 -type f -printf '%TY-%Tm-%Td %TH:%TM:%TS %p\n' | sort
tail -80 safe/out/dependents/logs/compile.log
tail -80 safe/out/dependents/logs/runtime.log
git status --short
```

Final sanity checks included the prompt's backtick-token scan, parsed the path/line citations in this file, compared the unsafe inventory against the maintained-source `rg` output, confirmed direct dependencies against `safe/Cargo.toml`, and grepped the cited legacy/allocator/sequence symbols. `cargo geiger` was attempted but was not installed (`cargo` reported `no such command: geiger`). `test-original.sh` was read and documented as the top-level downstream wrapper; it was not rerun for this documentation-only change because it rebuilds the dependent image before invoking the dependent matrix. `safe/scripts/run-full-suite.sh` was not rerun because no source, packaging, ABI, or test harness code changed. Files consulted in addition to the source tree include `safe/docs/unsafe-audit.md`, `safe/tests/upstream_test_matrix.toml`, `safe/tests/dependents/dependent_matrix.toml`, `dependents.json`, `relevant_cves.json`, `safe/out/dependents/image-context/metadata.env`, `safe/out/dependents/logs/compile.log`, `safe/out/dependents/logs/runtime.log`, and the dependent verification stamps under `safe/out/dependents/stamps/`.
