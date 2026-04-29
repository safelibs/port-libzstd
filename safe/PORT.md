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

`safe/build.rs` wires Cargo features into cfgs and build metadata. It emits `libzstd_threading` when threading is selected (`safe/build.rs:41-65`), sets `LIBZSTD_THREADING`, `LIBZSTD_VARIANT_SUFFIX`, and `LIBZSTD_DEFAULT_ARTIFACT` environment values (`safe/build.rs:83-89`), and gives the `cdylib` an ELF SONAME of `libzstd.so.1` (`safe/build.rs:87-89`). Its only C compilation job is the legacy decode bridge: it resolves upstream legacy sources from an optional in-tree source override or from `../original/libzstd-1.5.5+dfsg2/lib` (`safe/build.rs:18-24`, `safe/build.rs:91-101`) and compiles `xxhash.c`, `zstd_v05.c`, `zstd_v06.c`, `zstd_v07.c`, and `safe/src/ffi/legacy_shim.c` with `ZSTD_LEGACY_SUPPORT=5` (`safe/build.rs:111-123`).

Cargo features are defined in `safe/Cargo.toml:23-30`: the default feature set is empty, `legacy` is a named but empty feature, and `threading`, `build-shared-default`, `build-static-default`, `variant-mt`, and `variant-nomt` drive build metadata and cfg selection. `safe/scripts/build-artifacts.sh` builds release shared and static artifacts with Cargo, installs `libzstd.so.1.5.5`, `libzstd.so.1`, `libzstd.so`, `libzstd.a`, and the checked-in headers, and generates pkg-config and CMake install files from `safe/pkgconfig/` and `safe/cmake/`.

Debian packaging lives under `safe/debian/`. `safe/debian/control` declares source package `libzstd`, binary packages `libzstd-dev`, `libzstd1`, `zstd`, and `libzstd1-udeb`, and build dependencies including `cargo`, `rustc`, `cmake`, `debhelper (>> 13.3.2~)`, `dh-package-notes`, `help2man`, `liblz4-dev`, `liblzma-dev`, and `zlib1g-dev`; the debhelper compatibility level is recorded separately as `14` in `safe/debian/compat`. `safe/debian/rules` drives `safe/scripts/build-artifacts.sh` and `safe/scripts/build-original-cli-against-safe.sh`, then installs the Rust-built library together with the upstream CLI built against that library. `safe/debian/tests/control` registers autopkgtest coverage for the zstd self-test and pkg-config/CMake consumer builds.

## Where the unsafe Rust lives

The current inventory was produced with `rg -n '\bunsafe\b' safe --glob '*.rs' --glob '!target/**' --glob '!out/**'`, then reviewed modulo comments and strings. It includes first-party library code, Rust integration tests, and the local path dependency `safe/third_party/structured-zstd/`. Generated build artifacts under `safe/target/` and verification outputs under `safe/out/` are intentionally excluded.

| Purpose | Sites | Justification |
| --- | --- | --- |
| C ABI buffer conversion, by-reference dictionary buffers, output copies, optional pointer handling, and C struct out-parameters | `safe/src/common/frame.rs:90`; `safe/src/common/skippable.rs:36,42,72,77`; `safe/src/ffi/decompress.rs:46,265,650,1010,1127`; `safe/src/ffi/compress.rs:70,958,965,972,979,986,3012,3024,3058,3069,3076,3515,3566`; `safe/src/decompress/dctx.rs:304`; `safe/src/decompress/dstream.rs:75,169,175,179`; `safe/src/compress/cctx.rs:135`; `safe/src/compress/cstream.rs:253,258,261,264`; `safe/src/dict_builder/zdict.rs:106,117,232`; `safe/src/dict_builder/cover.rs:108`; `safe/src/dict_builder/fastcover.rs:138` | These are the expected raw-pointer operations at the libzstd ABI boundary: converting nullable C buffers to Rust slices, keeping by-reference dictionary buffers as slices under the caller's lifetime contract, copying encoded/decoded bytes into caller-owned buffers, and filling caller-provided structs. |
| Opaque handle ownership, static workspace APIs, and pointer casts for contexts/dictionaries | `safe/src/ffi/decompress.rs:658,666,674,705,723,761,762,774,788`; `safe/src/ffi/compress.rs:1025,1039,1163,1164,1203,1216`; `safe/src/compress/cctx_params.rs:153,160,195,262`; `safe/src/threading/pool.rs:31,58` | The C ABI represents contexts, dictionaries, and thread pools as opaque pointers or caller-provided workspaces; these sites cast back to Rust state, initialize in-place storage, or drop boxed state. |
| C callback ABI types and callback invocation | `safe/src/ffi/types.rs:42,44,439`; `safe/src/ffi/compress.rs:3435` | The original libzstd API exposes allocator callbacks and the sequence-producer callback as `unsafe extern "C" fn` types. The callback call is necessarily unsafe because Rust cannot validate the caller's function pointer contract. |
| Legacy decompression C bridge | `safe/src/decompress/legacy.rs:12,48,52,61,67,76,88,113,131,154` | These are declarations and calls into the internal C shim for zstd legacy frame versions. This is FFI beyond pure Rust, but it is isolated to legacy decode compatibility. |
| Internal compression match-state, match-finder, sequence, literal, compatibility, and strategy code | `safe/src/compress/frame.rs:15`; `safe/src/compress/ldm.rs:32,46,56,59,60,100,108,113,137,143,153,159,181,184`; `safe/src/compress/literals.rs:28,29,56,57,82`; `safe/src/compress/match_state.rs:77,92,98,106,116,232,246,258,306,311,319,326,342`; `safe/src/compress/sequence_api.rs:73`; `safe/src/compress/sequences.rs:59,70,113,114,131,136`; `safe/src/compress/compat.rs:73,77,102,119,122,142`; `safe/src/compress/strategies/double_fast.rs:14,27,34,37,44,47,54`; `safe/src/compress/strategies/fast.rs:14,28,35,38,45,48,55`; `safe/src/compress/strategies/lazy.rs:8,15,32,39,56,63,80,87,104,111,128,135,152,159,176,183,200,207,224,231,248,255,272,279`; `safe/src/compress/strategies/opt.rs:9,17,24,41,48,65,72,89,96,113,120,137,144,161,168`; `safe/src/decompress/frame.rs:982` | These sites are not merely thin C ABI shims. They implement performance-oriented compression/decompression internals, upstream-compatible raw structures, function-pointer strategy dispatch, unchecked slice indexing, or pointer arithmetic where the surrounding code maintains zstd invariants. They are the main remaining first-party unsafe code not required solely by raw C API parameter conversion. |
| Local Rust dependency internals | `safe/third_party/structured-zstd/src/decoding/decode_buffer.rs:90,122`; `safe/third_party/structured-zstd/src/decoding/ringbuffer.rs:23,26,74,94,104,129,142,169,217,225,259,281,292,313,320,327,345,357,379,386,393,412,419,426,450,457,472,475,538,540,560,579,613,623,639,649,673,683,778,985,1020,1035,1047` | `structured-zstd` is a local path dependency, not the public C ABI boundary. Its ring-buffer and decode-buffer internals use unsafe ownership and pointer operations for decoder buffering. |
| Rust integration tests | `safe/tests/rust/compress.rs:277,296`; `safe/tests/rust/decompress.rs:39,57,119` | Test code uses unsafe to call exported C ABI functions and to read or write through C-style buffers. |

Unsafe code that is not required by the public C ABI/API boundary is concentrated in the compression strategy/match-state modules and the local `structured-zstd` dependency. Those areas are internal algorithmic implementations and should remain the primary targets for future unsafe reduction after ABI shims and legacy compatibility are accounted for.

## Remaining unsafe FFI beyond the original ABI/API boundary

The port's intended public FFI boundary is the original libzstd C ABI/API exposed through the 185 `ZSTD_*` and `ZDICT_*` exports and the checked-in headers under `safe/include/`. Beyond that boundary, the current Rust code has one explicit foreign-function surface: the internal legacy decompression bridge.

| Surface | Symbols | Provider | Why it remains | Plausible safe-Rust replacement |
| --- | --- | --- | --- | --- |
| Legacy frame decode shim | `libzstd_safe_legacy_support`, `libzstd_safe_is_legacy`, `libzstd_safe_get_decompressed_size_legacy`, `libzstd_safe_decompress_legacy`, `libzstd_safe_find_frame_compressed_size_legacy`, `libzstd_safe_find_decompressed_bound_legacy`, `libzstd_safe_free_legacy_stream`, `libzstd_safe_init_legacy_stream`, `libzstd_safe_decompress_legacy_stream` | `safe/src/ffi/legacy_shim.c`, compiled with upstream legacy sources selected in `safe/build.rs:95-123` | libzstd promises decode compatibility for older frame formats. The Rust port delegates v0.5-v0.7 details to the upstream legacy decoder through `safe/src/decompress/legacy.rs:12`. | A native Rust implementation of the v0.5-v0.7 legacy frame decoders would remove this C bridge. |
| System runtime imports in the built shared object | glibc and runtime symbols visible in `nm -D --undefined-only safe/target/release/libzstd.so`, including allocation, memory, file-descriptor, thread-local, timing, and unwind support; `objdump -p` shows `NEEDED` entries for `libgcc_s.so.1`, `libm.so.6`, and `libc.so.6`. | Rust standard library/runtime, glibc, libm, libgcc unwinder, and the linked legacy C bridge object | The current crate uses `std`, builds an ELF shared object, and links the legacy decode bridge built from upstream legacy C decoder sources plus the Rust-facing shim. | A `no_std` design plus a native Rust legacy decoder and different panic/unwind strategy could reduce these imports, but that is outside the current drop-in Debian package goal. |

Evidence that no upstream dynamic fallback remains in decompression: `rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym' safe/src/decompress safe/src/ffi/decompress.rs safe/scripts/run-capi-decompression.sh` produced no matches, and `safe/scripts/run-capi-decompression.sh`, `safe/scripts/verify-export-parity.sh`, and `safe/scripts/verify-baseline-contract.sh` passed in this documentation pass. The only `unsafe extern "C"` block found in the Rust sources is the legacy shim declaration at `safe/src/decompress/legacy.rs:12`; the other `extern "C"` functions are the intended public libzstd ABI exports.

The callback types in `safe/src/ffi/types.rs:42,44,439` are part of the original ABI surface, not extra FFI. Custom allocation callbacks are currently accepted only in the upstream-compatible type definitions; the implementation supports the default/null allocator path. The sequence-producer callback can be invoked through the original advanced compression API at `safe/src/ffi/compress.rs:3435`.

## Remaining issues

- Custom memory allocators are not fully implemented. The ABI types for `ZSTD_allocFunction`, `ZSTD_freeFunction`, and `ZSTD_customMem` exist in `safe/src/ffi/types.rs`, but implementation helpers such as `safe/src/decompress/dctx.rs:14-15`, `safe/src/decompress/ddict.rs:14-15`, `safe/src/decompress/dstream.rs:27-28`, and `safe/src/ffi/compress.rs:3772-3773` accept only the default/null allocator configuration.
- Legacy v0.5-v0.7 decode still relies on C sources through `safe/src/ffi/legacy_shim.c` and `safe/src/decompress/legacy.rs`. This preserves compatibility but leaves a small non-Rust decoder island.
- The local `structured-zstd` dependency has unfinished or invariant-heavy internals. Notable markers include `safe/third_party/structured-zstd/src/decoding/ringbuffer.rs:93`, `safe/third_party/structured-zstd/src/decoding/block_decoder.rs:27`, `safe/third_party/structured-zstd/src/encoding/frame_header.rs:44,74`, and `safe/third_party/structured-zstd/src/encoding/frame_compressor.rs:256,408`. The public compression adapter currently maps requested levels through `safe/src/ffi/compress.rs:1724-1728`, avoiding the unimplemented compression-level branches in normal ABI use.
- Some upstream-suite gates are intentionally host-dependent. `safe/scripts/run-upstream-tests.sh` skips 32-bit and sanitizer variants when the required toolchains are unavailable, and it has a known valgrind fuzzer-smoke skip for unsupported worker-parameter behavior. `safe/scripts/run-pzstd-tests.sh` also has sanitizer-runtime skip handling, and `safe/scripts/run-zlibwrapper-tests.sh` documents known zlib wrapper expectation mismatches.
- `safe/out/phase6/run-full-suite-final.log` is not a complete fresh success artifact for this documentation pass; it stops in the upstream valgrind fuzzer-smoke area that `safe/scripts/run-upstream-tests.sh` knows how to skip. This pass reran the decompression Rust test, C API decompression harness, export parity check, and baseline contract check rather than the full upstream matrix.
- Dependent coverage is documented for 12 packages in `dependents.json` and `safe/tests/dependents/dependent_matrix.toml`. The checked-in logs `safe/out/dependents/logs/compile.log`, `safe/out/dependents/logs/runtime.log`, and `safe/out/dependents/logs/runtime-libarchive.log` record successful compile/runtime probes, but those Docker/image-style dependent gates were not rerun during this documentation refresh.
- `relevant_cves.json` contains two relevant records, CVE-2021-24031 and CVE-2021-24032, both for zstd CLI output-file permissions rather than core library memory safety. `safe/scripts/check-cli-permissions.sh` audits the CLI behavior with `strace`; this documentation pass did not rerun that script.
- No repository-level upgrade report file is present. No first-party `TODO` or `FIXME` markers were found under `safe/src` by `rg -n 'TODO|FIXME|todo!|unimplemented!|panic!' safe --glob '!target/**' --glob '!out/**'`; remaining markers are in build error paths, scripts, tests, or `safe/third_party/structured-zstd/`.

## Dependencies and other libraries used

Direct Cargo dependencies from `safe/Cargo.toml`:

| Dependency | Version | Kind | Purpose and safety notes |
| --- | --- | --- | --- |
| `oxiarc-core` | `=0.2.5` | normal | Provides shared OxiArc error/types used when mapping Rust zstd decode failures, including in `safe/src/decompress/frame.rs`. It is an external Rust dependency; `cargo geiger` was not available in this environment, so unsafe usage inside the crate was not independently measured here. |
| `oxiarc-zstd` | `=0.2.5` | normal | Provides Rust zstd encode/decode helpers used for fallback decode behavior, dictionary handling, and dictionary-builder scoring paths. It is an external Rust dependency and is covered here by integration/ABI tests rather than by a local unsafe inventory. |
| `structured-zstd` | `=0.0.3`, path `third_party/structured-zstd` | normal | Local structured zstd encoder/decoder used for frame parsing, dictionary-aware decode, streaming/block compression, and frame emission. Its remaining unsafe sites are inventoried above because the source is vendored in this repository. |
| `cc` | `1.2` | build | Build-time helper used by `safe/build.rs` to compile the legacy C shim and upstream legacy decoder sources. |

`cargo tree --manifest-path safe/Cargo.toml --edges normal,build` also shows transitive Rust dependencies including `thiserror`, procedural macro support crates, `twox-hash`, `find-msvc-tools`, and `shlex`. They are not direct dependencies of `libzstd-safe`.

Build-time tools and packaging dependencies are declared in `safe/debian/control` and used by `safe/debian/rules`: `cargo`, `rustc`, a C compiler through the `cc` crate, `cmake`, `debhelper (>> 13.3.2~)` with compatibility level `14` in `safe/debian/compat`, `dh-package-notes`, `dpkg-build-api`, `help2man`, `less`, `python3`, `liblz4-dev`, `liblzma-dev`, and `zlib1g-dev`. The original zstd CLI and pzstd support are built from `original/libzstd-1.5.5+dfsg2/` against the safe library by `safe/scripts/build-original-cli-against-safe.sh`. The installed pkg-config and CMake metadata are generated from checked-in templates under `safe/pkgconfig/` and `safe/cmake/`.

The Rust library build does not use `bindgen`, `cbindgen`, `pkg-config`, or a third-party C/C++ compression library. The built shared object currently links to normal system runtime libraries (`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and the dynamic loader as shown by `ldd` and `objdump -p safe/target/release/libzstd.so`). The only C code intentionally compiled into the Rust library is the legacy decode bridge listed in `safe/build.rs:95-101`: upstream `xxhash.c`, `zstd_v05.c`, `zstd_v06.c`, `zstd_v07.c`, and `safe/src/ffi/legacy_shim.c`.

## How this document was produced

Commands run or consulted for this refresh:

```sh
find safe -maxdepth 3 -type f | sort
sed -n '1,240p' safe/Cargo.toml
sed -n '1,260p' safe/build.rs
sed -n '1,200p' safe/src/lib.rs
find safe/debian -maxdepth 3 -type f | sort
cargo metadata --manifest-path safe/Cargo.toml --format-version 1 --no-deps
cargo tree --manifest-path safe/Cargo.toml --edges normal,build
cargo geiger --manifest-path safe/Cargo.toml --all-targets
rg -n '\bunsafe\b' safe --glob '*.rs' --glob '!target/**' --glob '!out/**'
grep -RIn '\bunsafe\b' safe
grep -RIn 'extern "C"' safe
rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym' safe/src/decompress safe/src/ffi/decompress.rs safe/scripts/run-capi-decompression.sh
rg -n 'extern "C"|unsafe extern' safe/src/decompress/legacy.rs safe/src/ffi/legacy_shim.c safe/build.rs
nm -D --defined-only safe/target/release/libzstd.so
nm -D --undefined-only safe/target/release/libzstd.so
objdump -p safe/target/release/libzstd.so
ldd safe/target/release/libzstd.so
cargo test --manifest-path safe/Cargo.toml --release --test decompress
bash safe/scripts/run-capi-decompression.sh
bash safe/scripts/verify-export-parity.sh
bash safe/scripts/verify-baseline-contract.sh
jq -r '.packages | length' dependents.json
jq -r '.relevant_cves[] | .cve_id' relevant_cves.json
rg -n 'TODO|FIXME|todo!|unimplemented!|panic!' safe --glob '!target/**' --glob '!out/**'
git status --short
```

`cargo geiger` was attempted but was not installed (`cargo` reported `no such command: geiger`). Files consulted in addition to the source tree include `safe/docs/unsafe-audit.md`, `safe/tests/upstream_test_matrix.toml`, `safe/tests/dependents/dependent_matrix.toml`, `dependents.json`, `relevant_cves.json`, `safe/out/dependents/logs/compile.log`, `safe/out/dependents/logs/runtime.log`, `safe/out/dependents/logs/runtime-libarchive.log`, and `safe/out/phase6/run-full-suite-final.log`.
