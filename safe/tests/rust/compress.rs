use std::{
    ffi::{c_char, c_int, c_void, CStr, CString, OsString},
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use zstd::{
    compress::{block as cblock, cctx, cctx_params, cdict, cstream, params},
    decompress::{dctx, ddict, dstream},
    ffi::types::{
        ZSTD_CCtx, ZSTD_CDict, ZSTD_CStream, ZSTD_EndDirective, ZSTD_ErrorCode,
        ZSTD_ResetDirective, ZSTD_bounds, ZSTD_cParameter, ZSTD_compressionParameters,
        ZSTD_customMem, ZSTD_dParameter, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e,
        ZSTD_frameParameters, ZSTD_inBuffer, ZSTD_outBuffer, ZSTD_parameters, ZSTD_strategy,
        ZSTD_BLOCKSIZE_MAX, ZSTD_CONTENTSIZE_UNKNOWN,
    },
};

fn dict_fixture() -> Vec<u8> {
    fs::read(dict_fixture_path()).expect("read formatted dictionary fixture")
}

fn invalid_cdict_fixture() -> &'static [u8] {
    &[
        0x37, 0xa4, 0x30, 0xec, 0x2a, 0x00, 0x00, 0x00, 0x39, 0x10, 0xc0, 0xc2, 0xa6, 0x00,
        0x0c, 0x30, 0xc0, 0x00, 0x03, 0x0c, 0x30, 0x20, 0x72, 0xf8, 0xb4, 0x6d, 0x4b, 0x9f,
        0xfc, 0x97, 0x29, 0x49, 0xb2, 0xdf, 0x4b, 0x29, 0x7d, 0x4a, 0xfc, 0x83, 0x18, 0x22,
        0x75, 0x23, 0x24, 0x44, 0x4d, 0x02, 0xb7, 0x97, 0x96, 0xf6, 0xcb, 0xd1, 0xcf, 0xe8,
        0x22, 0xea, 0x27, 0x36, 0xb7, 0x2c, 0x40, 0x46, 0x01, 0x08, 0x23, 0x01, 0x00, 0x00,
        0x06, 0x1e, 0x3c, 0x83, 0x81, 0xd6, 0x18, 0xd4, 0x12, 0x3a, 0x04, 0x00, 0x80, 0x03,
        0x08, 0x0e, 0x12, 0x1c, 0x12, 0x11, 0x0d, 0x0e, 0x0a, 0x0b, 0x0a, 0x09, 0x10, 0x0c,
        0x09, 0x05, 0x04, 0x03, 0x06, 0x06, 0x06, 0x02, 0x00, 0x03, 0x00, 0x00, 0x02, 0x02,
        0x00, 0x04, 0x06, 0x03, 0x06, 0x08, 0x24, 0x6b, 0x0d, 0x01, 0x10, 0x04, 0x81, 0x07,
        0x00, 0x00, 0x04, 0xb9, 0x58, 0x18, 0x06, 0x59, 0x92, 0x43, 0xce, 0x28, 0xa5, 0x08,
        0x88, 0xc0, 0x80, 0x88, 0x8c, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
    ]
}

fn dict_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../original/libzstd-1.5.5+dfsg2/tests/golden-dictionaries/http-dict-missing-symbols")
}

fn sample_bytes(size: usize) -> Vec<u8> {
    let fragments = [
        b"{\"tenant\":\"alpha\",\"region\":\"west\",\"kind\":\"session\",\"payload\":\"".as_slice(),
        b"{\"tenant\":\"beta\",\"region\":\"east\",\"kind\":\"metric\",\"payload\":\"".as_slice(),
        b"{\"tenant\":\"gamma\",\"region\":\"north\",\"kind\":\"record\",\"payload\":\"".as_slice(),
    ];
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    let mut out = Vec::with_capacity(size);
    let mut state = 0x1234_5678u32;
    while out.len() < size {
        let fragment = fragments[(state as usize) % fragments.len()];
        for &byte in fragment {
            if out.len() == size {
                break;
            }
            out.push(byte);
        }
        for _ in 0..96 {
            if out.len() == size {
                break;
            }
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            out.push(alphabet[(state as usize) % alphabet.len()]);
        }
        if out.len() < size {
            out.push(b'"');
        }
        if out.len() < size {
            out.push(b'}');
        }
        if out.len() < size {
            out.push(b'\n');
        }
    }
    out
}

fn compressible_sample(size: usize) -> Vec<u8> {
    let pattern = b"fn compressible_payload() { return \"alpha-beta-gamma-delta\"; }\n";
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let remaining = size - out.len();
        let chunk = remaining.min(pattern.len());
        out.extend_from_slice(&pattern[..chunk]);
    }
    out
}

fn dict_biased_sample(dict: &[u8], size: usize, seed: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(size);
    let mut cursor = (seed as usize) % dict.len();
    while out.len() < size {
        let mut chunk = 64 + ((seed as usize + out.len()) % 96);
        if chunk > dict.len() {
            chunk = dict.len();
        }
        if cursor + chunk > dict.len() {
            cursor = (cursor + 131 + ((seed as usize) % 29)) % dict.len();
            if cursor + chunk > dict.len() {
                chunk = dict.len() - cursor;
            }
        }
        let remaining = size - out.len();
        chunk = chunk.min(remaining);
        out.extend_from_slice(&dict[cursor..cursor + chunk]);
        if chunk > 12 {
            let pos = out.len() - chunk;
            out[pos + 3] ^= 0x11;
            out[pos + (chunk / 2)] ^= 0x5A;
        }
        if out.len() < size {
            out.push(b'\n');
        }
        cursor = (cursor + 97 + ((seed as usize) % 23)) % dict.len();
    }
    out
}

fn check_result(code: usize, what: &str) {
    assert_eq!(
        zstd::common::error::ZSTD_isError(code),
        0,
        "{what}: {}",
        unsafe {
            CStr::from_ptr(zstd::common::error::ZSTD_getErrorName(code))
                .to_string_lossy()
                .into_owned()
        }
    );
}

fn expect_error(code: usize, what: &str) {
    assert_eq!(
        zstd::common::error::ZSTD_isError(code),
        1,
        "{what} unexpectedly succeeded"
    );
}

fn decompress_exact(compressed: &[u8], expected: &[u8]) {
    let mut decoded = vec![0u8; expected.len()];
    let decoded_size = dctx::ZSTD_decompress(
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        compressed.as_ptr().cast(),
        compressed.len(),
    );
    check_result(decoded_size, "ZSTD_decompress");
    assert_eq!(decoded_size, expected.len());
    assert_eq!(decoded, expected);
}

fn decompress_with_prefix_exact(compressed: &[u8], prefix: &[u8], expected: &[u8]) {
    let dctx_ptr = dctx::ZSTD_createDCtx();
    let mut decoded = vec![0u8; expected.len()];

    assert!(
        !dctx_ptr.is_null(),
        "failed to create dctx for prefix decode"
    );
    check_result(
        dctx::ZSTD_DCtx_refPrefix(dctx_ptr, prefix.as_ptr().cast(), prefix.len()),
        "ZSTD_DCtx_refPrefix",
    );
    let decoded_size = dctx::ZSTD_decompressDCtx(
        dctx_ptr,
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        compressed.as_ptr().cast(),
        compressed.len(),
    );
    check_result(decoded_size, "ZSTD_decompressDCtx(prefix)");
    assert_eq!(decoded_size, expected.len());
    assert_eq!(decoded, expected);
    dctx::ZSTD_freeDCtx(dctx_ptr);
}

fn noise_bytes(size: usize, seed: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(size);
    let mut state = seed | 1;
    while out.len() < size {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        out.push((state & 0xFF) as u8);
    }
    out
}

fn wrap_single_block_frame(
    block_type: usize,
    block_header_size: usize,
    decompressed_size: usize,
    block_body: &[u8],
) -> Vec<u8> {
    let mut frame = Vec::with_capacity(4 + 1 + 8 + 3 + block_body.len());
    let descriptor = (1 << 5) | (3 << 6);
    let block_header = 1usize | (block_type << 1) | (block_header_size << 3);

    frame.extend_from_slice(&0xFD2F_B528u32.to_le_bytes());
    frame.push(descriptor);
    frame.extend_from_slice(&(decompressed_size as u64).to_le_bytes());
    frame.extend_from_slice(&(block_header as u32).to_le_bytes()[..3]);
    frame.extend_from_slice(block_body);
    frame
}

fn build_raw_block_frame(chunks: &[&[u8]]) -> Vec<u8> {
    let total_size: usize = chunks.iter().map(|chunk| chunk.len()).sum();
    let mut frame = Vec::with_capacity(16 + total_size + chunks.len() * 3);
    let descriptor = (1 << 5) | (3 << 6);

    frame.extend_from_slice(&0xFD2F_B528u32.to_le_bytes());
    frame.push(descriptor);
    frame.extend_from_slice(&(total_size as u64).to_le_bytes());
    for (index, chunk) in chunks.iter().enumerate() {
        let block_header = usize::from(index + 1 == chunks.len()) | (chunk.len() << 3);
        frame.extend_from_slice(&(block_header as u32).to_le_bytes()[..3]);
        frame.extend_from_slice(chunk);
    }
    frame
}

fn frame_first_block_type(frame: &[u8]) -> u8 {
    let header_size = zstd::common::frame::ZSTD_frameHeaderSize(frame.as_ptr().cast(), frame.len());
    assert!(header_size >= 5, "invalid frame header size {header_size}");
    assert!(
        frame.len() >= header_size + 3,
        "frame too short for block header"
    );
    let header = u32::from(frame[header_size])
        | (u32::from(frame[header_size + 1]) << 8)
        | (u32::from(frame[header_size + 2]) << 16);
    ((header >> 1) & 0x3) as u8
}

fn frame_descriptor(frame: &[u8]) -> u8 {
    let header_size = zstd::common::frame::ZSTD_frameHeaderSize(frame.as_ptr().cast(), frame.len());
    assert!(header_size >= 5, "invalid frame header size {header_size}");
    frame[4]
}

fn frame_has_checksum(frame: &[u8]) -> bool {
    (frame_descriptor(frame) & 0x04) != 0
}

fn frame_content_size(frame: &[u8]) -> u64 {
    zstd::common::frame::ZSTD_getFrameContentSize(frame.as_ptr().cast(), frame.len())
}

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;

fn upstream_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct UpstreamEnvRestore(Option<OsString>);

impl Drop for UpstreamEnvRestore {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            std::env::set_var("SAFE_TEST_UPSTREAM_LIB", value);
        } else {
            std::env::remove_var("SAFE_TEST_UPSTREAM_LIB");
        }
    }
}

fn with_temp_upstream_lib_env<T>(value: &str, f: impl FnOnce() -> T) -> T {
    let _guard = upstream_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let restore = UpstreamEnvRestore(std::env::var_os("SAFE_TEST_UPSTREAM_LIB"));
    std::env::set_var("SAFE_TEST_UPSTREAM_LIB", value);
    let result = f();
    drop(restore);
    result
}

fn upstream_lib_path() -> PathBuf {
    if let Ok(path) = std::env::var("SAFE_TEST_UPSTREAM_LIB") {
        return PathBuf::from(path);
    }

    for candidate in [
        "/lib/x86_64-linux-gnu/libzstd.so.1",
        "/usr/lib/x86_64-linux-gnu/libzstd.so.1",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return path;
        }
    }

    panic!("unable to locate an upstream libzstd shared object");
}

fn upstream_decompress_using_dict_exact(compressed: &[u8], dict: &[u8], expected: &[u8]) {
    type CreateDCtxFn = unsafe extern "C" fn() -> *mut c_void;
    type FreeDCtxFn = unsafe extern "C" fn(*mut c_void) -> usize;
    type DecompressUsingDictFn = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        usize,
    ) -> usize;
    type IsErrorFn = unsafe extern "C" fn(usize) -> u32;
    type GetErrorNameFn = unsafe extern "C" fn(usize) -> *const c_char;

    let _guard = upstream_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    unsafe {
        let library = CString::new(upstream_lib_path().to_string_lossy().as_bytes()).unwrap();
        let handle = dlopen(library.as_ptr(), RTLD_NOW | RTLD_LOCAL);
        assert!(!handle.is_null(), "dlopen upstream libzstd failed");

        let create = CString::new("ZSTD_createDCtx").unwrap();
        let free = CString::new("ZSTD_freeDCtx").unwrap();
        let decode = CString::new("ZSTD_decompress_usingDict").unwrap();
        let is_error = CString::new("ZSTD_isError").unwrap();
        let get_error_name = CString::new("ZSTD_getErrorName").unwrap();

        let create_dctx_ptr = dlsym(handle, create.as_ptr());
        let free_dctx_ptr = dlsym(handle, free.as_ptr());
        let decompress_using_dict_ptr = dlsym(handle, decode.as_ptr());
        let upstream_is_error_ptr = dlsym(handle, is_error.as_ptr());
        let upstream_get_error_name_ptr = dlsym(handle, get_error_name.as_ptr());

        assert!(!create_dctx_ptr.is_null(), "missing upstream createDCtx");
        assert!(!free_dctx_ptr.is_null(), "missing upstream freeDCtx");
        assert!(
            !decompress_using_dict_ptr.is_null(),
            "missing upstream decompress_usingDict"
        );
        assert!(!upstream_is_error_ptr.is_null(), "missing upstream isError");
        assert!(
            !upstream_get_error_name_ptr.is_null(),
            "missing upstream getErrorName"
        );

        let create_dctx: CreateDCtxFn = core::mem::transmute(create_dctx_ptr);
        let free_dctx: FreeDCtxFn = core::mem::transmute(free_dctx_ptr);
        let decompress_using_dict: DecompressUsingDictFn =
            core::mem::transmute(decompress_using_dict_ptr);
        let upstream_is_error: IsErrorFn = core::mem::transmute(upstream_is_error_ptr);
        let upstream_get_error_name: GetErrorNameFn =
            core::mem::transmute(upstream_get_error_name_ptr);

        let dctx = create_dctx();
        let mut decoded = vec![0u8; expected.len()];
        let decoded_size = decompress_using_dict(
            dctx,
            decoded.as_mut_ptr().cast(),
            decoded.len(),
            compressed.as_ptr().cast(),
            compressed.len(),
            dict.as_ptr().cast(),
            dict.len(),
        );
        free_dctx(dctx);

        assert_eq!(
            upstream_is_error(decoded_size),
            0,
            "upstream ZSTD_decompress_usingDict: {}",
            CStr::from_ptr(upstream_get_error_name(decoded_size))
                .to_string_lossy()
                .into_owned()
        );
        assert_eq!(decoded_size, expected.len());
        assert_eq!(decoded, expected);
    }
}

fn emit_legacy_frame(cctx_ptr: *mut ZSTD_CCtx, segments: &[&[u8]]) -> Vec<u8> {
    let total_size: usize = segments.iter().map(|segment| segment.len()).sum();
    let mut compressed = vec![0u8; cctx::ZSTD_compressBound(total_size)];
    let mut written = 0usize;

    for segment in &segments[..segments.len().saturating_sub(1)] {
        let produced = cctx::ZSTD_compressContinue(
            cctx_ptr,
            compressed[written..].as_mut_ptr().cast(),
            compressed.len() - written,
            segment.as_ptr().cast(),
            segment.len(),
        );
        check_result(produced, "ZSTD_compressContinue");
        assert!(
            produced > 0 || segment.is_empty(),
            "ZSTD_compressContinue buffered all output until end"
        );
        written += produced;
    }

    let last = segments.last().copied().unwrap_or(&[]);
    let produced = cctx::ZSTD_compressEnd(
        cctx_ptr,
        compressed[written..].as_mut_ptr().cast(),
        compressed.len() - written,
        last.as_ptr().cast(),
        last.len(),
    );
    check_result(produced, "ZSTD_compressEnd");
    written += produced;
    compressed.truncate(written);
    compressed
}

fn compress_stream_legacy_limited(
    zcs: *mut ZSTD_CStream,
    src: &[u8],
    out_capacity: usize,
) -> (Vec<u8>, bool) {
    let mut compressed = Vec::new();
    let mut input = ZSTD_inBuffer {
        src: src.as_ptr().cast(),
        size: src.len(),
        pos: 0,
    };
    let mut produced_before_end = false;

    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream(zcs, &mut output, &mut input);
        check_result(remaining, "ZSTD_compressStream");
        produced_before_end |= output.pos > 0;
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if input.pos == input.size {
            break;
        }
    }

    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_endStream(zcs, &mut output);
        check_result(remaining, "ZSTD_endStream");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    (compressed, produced_before_end)
}

fn compress_stream2_continue_then_end(
    cctx: *mut ZSTD_CCtx,
    src: &[u8],
    out_capacity: usize,
) -> (Vec<u8>, bool) {
    let split = src.len() / 2;
    let mut compressed = Vec::new();
    let mut produced_before_end = false;
    let mut first = ZSTD_inBuffer {
        src: src[..split].as_ptr().cast(),
        size: split,
        pos: 0,
    };

    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream2(
            cctx,
            &mut output,
            &mut first,
            ZSTD_EndDirective::ZSTD_e_continue,
        );
        check_result(remaining, "ZSTD_compressStream2(continue)");
        produced_before_end |= output.pos > 0;
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if first.pos == first.size && remaining == 0 {
            break;
        }
    }

    let mut second = ZSTD_inBuffer {
        src: src[split..].as_ptr().cast(),
        size: src.len() - split,
        pos: 0,
    };
    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream2(
            cctx,
            &mut output,
            &mut second,
            ZSTD_EndDirective::ZSTD_e_end,
        );
        check_result(remaining, "ZSTD_compressStream2(end)");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    (compressed, produced_before_end)
}

fn compress_stream_legacy_flush_then_end(
    zcs: *mut ZSTD_CStream,
    src: &[u8],
    out_capacity: usize,
) -> (Vec<u8>, bool) {
    let split = src.len() / 2;
    let mut compressed = Vec::new();
    let mut first = ZSTD_inBuffer {
        src: src[..split].as_ptr().cast(),
        size: split,
        pos: 0,
    };
    let mut flushed_output = false;
    let mut out_buf = vec![0u8; out_capacity.max(1)];
    let mut output = ZSTD_outBuffer {
        dst: out_buf.as_mut_ptr().cast(),
        size: out_buf.len(),
        pos: 0,
    };
    let remaining = cstream::ZSTD_compressStream(zcs, &mut output, &mut first);
    check_result(remaining, "ZSTD_compressStream(pre-flush)");
    compressed.extend_from_slice(&out_buf[..output.pos]);

    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_flushStream(zcs, &mut output);
        check_result(remaining, "ZSTD_flushStream");
        flushed_output |= output.pos > 0;
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream(zcs, &mut output, &mut first);
        check_result(remaining, "ZSTD_compressStream(post-flush first)");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if first.pos == first.size {
            break;
        }
    }

    let mut second = ZSTD_inBuffer {
        src: src[split..].as_ptr().cast(),
        size: src.len() - split,
        pos: 0,
    };
    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream(zcs, &mut output, &mut second);
        check_result(remaining, "ZSTD_compressStream(post-flush)");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if second.pos == second.size {
            break;
        }
    }

    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_endStream(zcs, &mut output);
        check_result(remaining, "ZSTD_endStream(post-flush)");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    (compressed, flushed_output)
}

fn compress_stream2_continue_flush_then_end(
    cctx: *mut ZSTD_CCtx,
    src: &[u8],
    out_capacity: usize,
) -> (Vec<u8>, bool) {
    let split = src.len() / 2;
    let mut compressed = Vec::new();
    let mut first = ZSTD_inBuffer {
        src: src[..split].as_ptr().cast(),
        size: split,
        pos: 0,
    };
    let mut flushed_output = false;
    let mut out_buf = vec![0u8; out_capacity.max(1)];
    let mut output = ZSTD_outBuffer {
        dst: out_buf.as_mut_ptr().cast(),
        size: out_buf.len(),
        pos: 0,
    };
    let remaining = cstream::ZSTD_compressStream2(
        cctx,
        &mut output,
        &mut first,
        ZSTD_EndDirective::ZSTD_e_continue,
    );
    check_result(remaining, "ZSTD_compressStream2(pre-flush)");
    compressed.extend_from_slice(&out_buf[..output.pos]);

    let mut flush_input = ZSTD_inBuffer {
        src: core::ptr::null(),
        size: 0,
        pos: 0,
    };
    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream2(
            cctx,
            &mut output,
            &mut flush_input,
            ZSTD_EndDirective::ZSTD_e_flush,
        );
        check_result(remaining, "ZSTD_compressStream2(flush)");
        flushed_output |= output.pos > 0;
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream2(
            cctx,
            &mut output,
            &mut first,
            ZSTD_EndDirective::ZSTD_e_continue,
        );
        check_result(remaining, "ZSTD_compressStream2(post-flush first)");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if first.pos == first.size && remaining == 0 {
            break;
        }
    }

    let mut second = ZSTD_inBuffer {
        src: src[split..].as_ptr().cast(),
        size: src.len() - split,
        pos: 0,
    };
    loop {
        let mut out_buf = vec![0u8; out_capacity.max(1)];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream2(
            cctx,
            &mut output,
            &mut second,
            ZSTD_EndDirective::ZSTD_e_end,
        );
        check_result(remaining, "ZSTD_compressStream2(end after flush)");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    (compressed, flushed_output)
}

#[test]
fn compress_one_shot_context_and_block_api_roundtrip() {
    let src = sample_bytes(256 * 1024 + 19);
    let compressible = compressible_sample(128 * 1024 + 17);
    let bound = cctx::ZSTD_compressBound(src.len());
    let compressible_bound = cctx::ZSTD_compressBound(compressible.len());
    let mut compressed = vec![0u8; bound];
    let mut compressible_out = vec![0u8; compressible_bound];
    let mut second = vec![0u8; bound];
    let mut third = vec![0u8; bound];
    let cctx_ptr: *mut ZSTD_CCtx = cctx::ZSTD_createCCtx();
    let clone: *mut ZSTD_CCtx = cctx::ZSTD_createCCtx();
    let copy_src: *mut ZSTD_CCtx = cctx::ZSTD_createCCtx();
    let mut level = 0;

    assert!(bound >= src.len());

    let size = cctx::ZSTD_compress(
        compressed.as_mut_ptr().cast(),
        compressed.len(),
        src.as_ptr().cast(),
        src.len(),
        1,
    );
    check_result(size, "ZSTD_compress");
    decompress_exact(&compressed[..size], &src);

    let compressible_size = cctx::ZSTD_compress(
        compressible_out.as_mut_ptr().cast(),
        compressible_out.len(),
        compressible.as_ptr().cast(),
        compressible.len(),
        1,
    );
    check_result(compressible_size, "ZSTD_compress(compressible)");
    assert_eq!(frame_first_block_type(&compressible_out[..compressible_size]), 2);
    assert!(compressible_size < compressible.len());
    decompress_exact(&compressible_out[..compressible_size], &compressible);

    assert!(!cctx_ptr.is_null());
    assert!(!clone.is_null());
    assert!(cctx::ZSTD_sizeof_CCtx(cctx_ptr) > 0);
    check_result(
        cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_compressionLevel, 5),
        "ZSTD_CCtx_setParameter(level)",
    );
    check_result(
        cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_compressionLevel, -5),
        "ZSTD_CCtx_setParameter(negative level)",
    );
    check_result(
        cctx::ZSTD_CCtx_getParameter(
            cctx_ptr,
            ZSTD_cParameter::ZSTD_c_compressionLevel,
            &mut level,
        ),
        "ZSTD_CCtx_getParameter(negative level)",
    );
    assert_eq!(level, -5);
    check_result(
        cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_compressionLevel, 5),
        "ZSTD_CCtx_setParameter(level restore)",
    );
    check_result(
        cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_checksumFlag, 1),
        "ZSTD_CCtx_setParameter(checksum)",
    );
    check_result(
        cctx::ZSTD_CCtx_getParameter(
            cctx_ptr,
            ZSTD_cParameter::ZSTD_c_compressionLevel,
            &mut level,
        ),
        "ZSTD_CCtx_getParameter(level)",
    );
    assert_eq!(level, 5);
    check_result(
        cctx::ZSTD_CCtx_setPledgedSrcSize(cctx_ptr, src.len() as u64),
        "ZSTD_CCtx_setPledgedSrcSize",
    );
    check_result(
        cctx::ZSTD_compressBegin(copy_src, 1),
        "ZSTD_compressBegin(copy_src)",
    );
    check_result(
        cctx::ZSTD_copyCCtx(clone, copy_src, ZSTD_CONTENTSIZE_UNKNOWN),
        "ZSTD_copyCCtx",
    );

    let size = cctx::ZSTD_compress2(
        cctx_ptr,
        second.as_mut_ptr().cast(),
        second.len(),
        src.as_ptr().cast(),
        src.len(),
    );
    check_result(size, "ZSTD_compress2");
    decompress_exact(&second[..size], &src);

    let size = cctx::ZSTD_compress2(
        clone,
        third.as_mut_ptr().cast(),
        third.len(),
        src.as_ptr().cast(),
        src.len(),
    );
    check_result(size, "ZSTD_compress2(clone)");
    decompress_exact(&third[..size], &src);

    let size = cctx::ZSTD_compressCCtx(
        cctx_ptr,
        second.as_mut_ptr().cast(),
        second.len(),
        src.as_ptr().cast(),
        src.len(),
        3,
    );
    check_result(size, "ZSTD_compressCCtx");
    assert!(
        !frame_has_checksum(&second[..size]),
        "ZSTD_compressCCtx preserved sticky checksum parameters"
    );
    decompress_exact(&second[..size], &src);

    check_result(
        cctx::ZSTD_CCtx_reset(
            copy_src,
            ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
        ),
        "ZSTD_CCtx_reset(legacy)",
    );
    check_result(
        cctx::ZSTD_compressBegin(copy_src, 3),
        "ZSTD_compressBegin(legacy)",
    );
    let legacy = emit_legacy_frame(
        copy_src,
        &[
            &src[..96 * 1024],
            &src[96 * 1024..160 * 1024],
            &src[160 * 1024..],
        ],
    );
    decompress_exact(&legacy, &src);

    {
        check_result(cctx::ZSTD_compressBegin(cctx_ptr, 1), "ZSTD_compressBegin");
        let block_limit = cblock::ZSTD_getBlockSize(cctx_ptr);
        assert_eq!(block_limit, ZSTD_BLOCKSIZE_MAX);
        let block_src = vec![b'A'; block_limit];
        let mut block_compressed = vec![0u8; cctx::ZSTD_compressBound(block_limit)];
        let block_size = cblock::ZSTD_compressBlock(
            cctx_ptr,
            block_compressed.as_mut_ptr().cast(),
            block_compressed.len(),
            block_src.as_ptr().cast(),
            block_src.len(),
        );
        check_result(block_size, "ZSTD_compressBlock");
        assert!(block_size > 0);
        assert!(block_size < block_compressed.len());
        let wrapped = wrap_single_block_frame(
            1,
            block_src.len(),
            block_src.len(),
            &block_compressed[..block_size],
        );
        decompress_exact(&wrapped, &block_src);

        let too_large = vec![b'B'; block_limit + 1];
        let oversize = cblock::ZSTD_compressBlock(
            cctx_ptr,
            block_compressed.as_mut_ptr().cast(),
            block_compressed.len(),
            too_large.as_ptr().cast(),
            too_large.len(),
        );
        assert_eq!(
            zstd::common::error::ZSTD_isError(oversize),
            1,
            "ZSTD_compressBlock unexpectedly accepted oversized input",
        );
        assert_eq!(
            zstd::common::error::ZSTD_getErrorCode(oversize),
            ZSTD_ErrorCode::ZSTD_error_srcSize_wrong,
        );

        let noise = noise_bytes(block_limit, 0xA51C_9E77);
        check_result(
            cctx::ZSTD_compressBegin(cctx_ptr, 1),
            "ZSTD_compressBegin(block history)",
        );
        let first_size = cblock::ZSTD_compressBlock(
            cctx_ptr,
            block_compressed.as_mut_ptr().cast(),
            block_compressed.len(),
            noise.as_ptr().cast(),
            noise.len(),
        );
        assert_eq!(
            first_size, 0,
            "ZSTD_compressBlock unexpectedly emitted an incompressible block"
        );
        let second_size = cblock::ZSTD_compressBlock(
            cctx_ptr,
            block_compressed.as_mut_ptr().cast(),
            block_compressed.len(),
            noise.as_ptr().cast(),
            noise.len(),
        );
        check_result(second_size, "ZSTD_compressBlock(history)");
        assert!(
            second_size > 0,
            "repeated block did not use prior block history"
        );
    }

    {
        let advanced_params = params::ZSTD_getParams(3, src.len() as u64, 0);
        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(advanced legacy pledge)",
        );
        check_result(
            cctx::ZSTD_compressBegin_advanced(
                cctx_ptr,
                core::ptr::null(),
                0,
                advanced_params,
                src.len() as u64 + 1,
            ),
            "ZSTD_compressBegin_advanced",
        );
        let pledge_mismatch = cctx::ZSTD_compressEnd(
            cctx_ptr,
            third.as_mut_ptr().cast(),
            third.len(),
            src.as_ptr().cast(),
            src.len(),
        );
        assert_eq!(
            zstd::common::error::ZSTD_isError(pledge_mismatch),
            1,
            "ZSTD_compressEnd unexpectedly accepted a pledged-size mismatch",
        );
        assert_eq!(
            zstd::common::error::ZSTD_getErrorCode(pledge_mismatch),
            ZSTD_ErrorCode::ZSTD_error_srcSize_wrong,
        );
    }

    {
        let dctx_ptr = dctx::ZSTD_createDCtx();
        let first_raw = sample_bytes(8 * 1024 + 3);
        let second_raw = sample_bytes(6 * 1024 + 5);
        let frame = build_raw_block_frame(&[&first_raw, &second_raw]);
        let header_size =
            zstd::common::frame::ZSTD_frameHeaderSize(frame.as_ptr().cast(), frame.len());
        let mut offset = 0usize;

        assert!(!dctx_ptr.is_null());
        check_result(
            dctx::ZSTD_decompressBegin(dctx_ptr),
            "ZSTD_decompressBegin(insertBlock)",
        );
        check_result(
            dctx::ZSTD_decompressContinue(
                dctx_ptr,
                core::ptr::null_mut(),
                0,
                frame[offset..offset + 5].as_ptr().cast(),
                5,
            ),
            "ZSTD_decompressContinue(insertBlock prefix)",
        );
        offset += 5;
        check_result(
            dctx::ZSTD_decompressContinue(
                dctx_ptr,
                core::ptr::null_mut(),
                0,
                frame[offset..header_size].as_ptr().cast(),
                header_size - offset,
            ),
            "ZSTD_decompressContinue(insertBlock header)",
        );
        offset = header_size;
        check_result(
            dctx::ZSTD_decompressContinue(
                dctx_ptr,
                core::ptr::null_mut(),
                0,
                frame[offset..offset + 3].as_ptr().cast(),
                3,
            ),
            "ZSTD_decompressContinue(insertBlock first header)",
        );
        offset += 3;
        assert_eq!(
            dstream::ZSTD_nextSrcSizeToDecompress(dctx_ptr),
            first_raw.len()
        );
        let inserted = cblock::ZSTD_insertBlock(
            dctx_ptr,
            frame[offset..offset + first_raw.len()].as_ptr().cast(),
            first_raw.len(),
        );
        check_result(inserted, "ZSTD_insertBlock(first)");
        assert_eq!(inserted, first_raw.len());
        offset += first_raw.len();
        assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx_ptr), 3);

        check_result(
            dctx::ZSTD_decompressContinue(
                dctx_ptr,
                core::ptr::null_mut(),
                0,
                frame[offset..offset + 3].as_ptr().cast(),
                3,
            ),
            "ZSTD_decompressContinue(insertBlock second header)",
        );
        offset += 3;
        assert_eq!(
            dstream::ZSTD_nextSrcSizeToDecompress(dctx_ptr),
            second_raw.len()
        );
        let inserted = cblock::ZSTD_insertBlock(
            dctx_ptr,
            frame[offset..offset + second_raw.len()].as_ptr().cast(),
            second_raw.len(),
        );
        check_result(inserted, "ZSTD_insertBlock(second)");
        assert_eq!(inserted, second_raw.len());
        offset += second_raw.len();
        assert_eq!(offset, frame.len());
        assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx_ptr), 0);
        dctx::ZSTD_freeDCtx(dctx_ptr);
    }

    cctx::ZSTD_freeCCtx(clone);
    cctx::ZSTD_freeCCtx(copy_src);
    cctx::ZSTD_freeCCtx(cctx_ptr);
}

#[test]
fn compress_dictionary_and_prefix_helpers_roundtrip() {
    let dict = dict_fixture();
    let src = dict_biased_sample(&dict, 64 * 1024 + 131, 0x12345);
    let dict_id = ddict::ZSTD_getDictID_fromDict(dict.as_ptr().cast(), dict.len());
    let cdict_ptr: *mut ZSTD_CDict = cdict::ZSTD_createCDict(dict.as_ptr().cast(), dict.len(), 5);
    let cctx_ptr = cctx::ZSTD_createCCtx();
    let bound = cctx::ZSTD_compressBound(src.len());
    let mut compressed = vec![0u8; bound];
    let mut second = vec![0u8; bound];
    let mut decoded = vec![0u8; src.len().max(dict.len() * 4)];

    assert!(!cdict_ptr.is_null());
    assert_eq!(cdict::ZSTD_getDictID_fromCDict(cdict_ptr), dict_id);

    let size = cdict::ZSTD_compress_usingCDict(
        cctx_ptr,
        compressed.as_mut_ptr().cast(),
        compressed.len(),
        src.as_ptr().cast(),
        src.len(),
        cdict_ptr,
    );
    check_result(size, "ZSTD_compress_usingCDict");
    assert_eq!(frame_first_block_type(&compressed[..size]), 2);
    assert!(size < src.len(), "CDict compression failed to shrink the source");
    assert_eq!(
        zstd::common::frame::ZSTD_getDictID_fromFrame(compressed.as_ptr().cast(), size),
        dict_id
    );
    expect_error(
        dctx::ZSTD_decompress(
            decoded.as_mut_ptr().cast(),
            decoded.len(),
            compressed.as_ptr().cast(),
            size,
        ),
        "ZSTD_decompress(dictionary frame without dict)",
    );
    upstream_decompress_using_dict_exact(&compressed[..size], &dict, &src);

    let size = cctx::ZSTD_compress_usingDict(
        cctx_ptr,
        second.as_mut_ptr().cast(),
        second.len(),
        src.as_ptr().cast(),
        src.len(),
        dict.as_ptr().cast(),
        dict.len(),
        5,
    );
    check_result(size, "ZSTD_compress_usingDict");
    assert_eq!(frame_first_block_type(&second[..size]), 2);
    assert!(size < src.len(), "usingDict compression failed to shrink the source");
    upstream_decompress_using_dict_exact(&second[..size], &dict, &src);
    let isolated_size = with_temp_upstream_lib_env("/definitely/missing/libzstd.so.1", || {
        let isolated_size = cctx::ZSTD_compress_usingDict(
            cctx_ptr,
            compressed.as_mut_ptr().cast(),
            compressed.len(),
            src.as_ptr().cast(),
            src.len(),
            dict.as_ptr().cast(),
            dict.len(),
            5,
        );
        check_result(
            isolated_size,
            "ZSTD_compress_usingDict(independent from SAFE_TEST_UPSTREAM_LIB)",
        );
        assert_eq!(frame_first_block_type(&compressed[..isolated_size]), 2);
        assert!(
            isolated_size < src.len(),
            "usingDict compression stopped shrinking the source when upstream env was invalid"
        );
        isolated_size
    });
    upstream_decompress_using_dict_exact(&compressed[..isolated_size], &dict, &src);

    {
        let by_copy = cdict::ZSTD_createCDict_advanced(
            dict.as_ptr().cast(),
            dict.len(),
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy,
            ZSTD_dictContentType_e::ZSTD_dct_fullDict,
            params::ZSTD_getCParams(5, ZSTD_CONTENTSIZE_UNKNOWN, dict.len()),
            ZSTD_customMem::default(),
        );
        let by_ref_raw = cdict::ZSTD_createCDict_advanced(
            dict.as_ptr().cast(),
            dict.len(),
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef,
            ZSTD_dictContentType_e::ZSTD_dct_rawContent,
            params::ZSTD_getCParams(5, ZSTD_CONTENTSIZE_UNKNOWN, dict.len()),
            ZSTD_customMem::default(),
        );
        assert!(!by_copy.is_null());
        assert!(!by_ref_raw.is_null());
        assert!(cdict::ZSTD_sizeof_CDict(by_ref_raw) < cdict::ZSTD_sizeof_CDict(by_copy));

        let raw_size = cdict::ZSTD_compress_usingCDict(
            cctx_ptr,
            compressed.as_mut_ptr().cast(),
            compressed.len(),
            src.as_ptr().cast(),
            src.len(),
            by_ref_raw,
        );
        check_result(raw_size, "ZSTD_compress_usingCDict(raw advanced)");
        assert_eq!(
            zstd::common::frame::ZSTD_getDictID_fromFrame(compressed.as_ptr().cast(), raw_size),
            0
        );

        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(loadDictionary_advanced raw)",
        );
        check_result(
            cdict::ZSTD_CCtx_loadDictionary_advanced(
                cctx_ptr,
                dict.as_ptr().cast(),
                dict.len(),
                ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef,
                ZSTD_dictContentType_e::ZSTD_dct_rawContent,
            ),
            "ZSTD_CCtx_loadDictionary_advanced(raw)",
        );
        let raw_loaded_size = cctx::ZSTD_compress2(
            cctx_ptr,
            second.as_mut_ptr().cast(),
            second.len(),
            src.as_ptr().cast(),
            src.len(),
        );
        check_result(raw_loaded_size, "ZSTD_compress2(loadDictionary_advanced raw)");
        assert_eq!(
            zstd::common::frame::ZSTD_getDictID_fromFrame(
                second.as_ptr().cast(),
                raw_loaded_size,
            ),
            0
        );

        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(loadDictionary_advanced full)",
        );
        check_result(
            cdict::ZSTD_CCtx_loadDictionary_advanced(
                cctx_ptr,
                dict.as_ptr().cast(),
                dict.len(),
                ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy,
                ZSTD_dictContentType_e::ZSTD_dct_fullDict,
            ),
            "ZSTD_CCtx_loadDictionary_advanced(full)",
        );
        let full_loaded_size = cctx::ZSTD_compress2(
            cctx_ptr,
            second.as_mut_ptr().cast(),
            second.len(),
            src.as_ptr().cast(),
            src.len(),
        );
        check_result(full_loaded_size, "ZSTD_compress2(loadDictionary_advanced full)");
        assert_eq!(
            zstd::common::frame::ZSTD_getDictID_fromFrame(
                second.as_ptr().cast(),
                full_loaded_size,
            ),
            dict_id
        );

        cdict::ZSTD_freeCDict(by_ref_raw);
        cdict::ZSTD_freeCDict(by_copy);
    }

    check_result(
        cctx::ZSTD_CCtx_reset(
            cctx_ptr,
            ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
        ),
        "ZSTD_CCtx_reset(legacy dict)",
    );
    check_result(
        cctx::ZSTD_compressBegin_usingDict(cctx_ptr, dict.as_ptr().cast(), dict.len(), 5),
        "ZSTD_compressBegin_usingDict",
    );
    let legacy = emit_legacy_frame(cctx_ptr, &[&src[..24 * 1024], &src[24 * 1024..]]);
    assert_eq!(frame_first_block_type(&legacy), 2);
    upstream_decompress_using_dict_exact(&legacy, &dict, &src);

    {
        let prefix = &dict[dict.len().saturating_sub(32 * 1024)..];
        let prefix_src = {
            let mut bytes = prefix.to_vec();
            bytes.extend_from_slice(prefix);
            bytes.extend_from_slice(prefix);
            bytes
        };
        let mut plain = vec![0u8; cctx::ZSTD_compressBound(prefix_src.len())];
        let mut prefixed = vec![0u8; cctx::ZSTD_compressBound(prefix_src.len())];
        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(prefix/plain)",
        );
        let plain_size = cctx::ZSTD_compress2(
            cctx_ptr,
            plain.as_mut_ptr().cast(),
            plain.len(),
            prefix_src.as_ptr().cast(),
            prefix_src.len(),
        );
        check_result(plain_size, "ZSTD_compress2(plain)");

        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(prefix/ref)",
        );
        check_result(
            cctx::ZSTD_CCtx_refPrefix(cctx_ptr, prefix.as_ptr().cast(), prefix.len()),
            "ZSTD_CCtx_refPrefix",
        );
        let prefixed_size = cctx::ZSTD_compress2(
            cctx_ptr,
            prefixed.as_mut_ptr().cast(),
            prefixed.len(),
            prefix_src.as_ptr().cast(),
            prefix_src.len(),
        );
        check_result(prefixed_size, "ZSTD_compress2(prefixed)");
        assert!(
            prefixed_size < plain_size,
            "expected prefix to shrink the frame, got prefixed={prefixed_size}, plain={plain_size}"
        );
        assert!(prefixed_size > 0);
        assert_eq!(frame_first_block_type(&prefixed[..prefixed_size]), 2);
        decompress_with_prefix_exact(&prefixed[..prefixed_size], prefix, &prefix_src);

        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(prefix/clear)",
        );
        check_result(
            cctx::ZSTD_CCtx_refPrefix(cctx_ptr, prefix.as_ptr().cast(), prefix.len()),
            "ZSTD_CCtx_refPrefix(set before clear)",
        );
        check_result(
            cctx::ZSTD_CCtx_refPrefix(cctx_ptr, core::ptr::null(), 0),
            "ZSTD_CCtx_refPrefix(clear)",
        );
        let cleared_size = cctx::ZSTD_compress2(
            cctx_ptr,
            prefixed.as_mut_ptr().cast(),
            prefixed.len(),
            prefix_src.as_ptr().cast(),
            prefix_src.len(),
        );
        check_result(cleared_size, "ZSTD_compress2(prefix cleared)");
        assert_eq!(cleared_size, plain_size);
        assert_eq!(&prefixed[..cleared_size], &plain[..plain_size]);
    }

    cdict::ZSTD_freeCDict(cdict_ptr);
    cctx::ZSTD_freeCCtx(cctx_ptr);
}

#[test]
fn compress_invalid_formatted_dictionary_rejects_cdict_creation() {
    let dict = invalid_cdict_fixture();
    let params = cctx_params::ZSTD_createCCtxParams();

    assert!(params.is_null() == false);
    check_result(
        cctx_params::ZSTD_CCtxParams_init(params, 1),
        "ZSTD_CCtxParams_init",
    );

    let cdict_ptr = cdict::ZSTD_createCDict(dict.as_ptr().cast(), dict.len(), 1);
    let advanced_ptr = cdict::ZSTD_createCDict_advanced(
        dict.as_ptr().cast(),
        dict.len(),
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy,
        ZSTD_dictContentType_e::ZSTD_dct_fullDict,
        params::ZSTD_getCParams(1, ZSTD_CONTENTSIZE_UNKNOWN, dict.len()),
        ZSTD_customMem {
            customAlloc: None,
            customFree: None,
            opaque: core::ptr::null_mut(),
        },
    );
    let advanced2_ptr = cdict::ZSTD_createCDict_advanced2(
        dict.as_ptr().cast(),
        dict.len(),
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy,
        ZSTD_dictContentType_e::ZSTD_dct_fullDict,
        params,
        ZSTD_customMem {
            customAlloc: None,
            customFree: None,
            opaque: core::ptr::null_mut(),
        },
    );

    assert!(cdict_ptr.is_null());
    assert!(advanced_ptr.is_null());
    assert!(advanced2_ptr.is_null());

    cctx_params::ZSTD_freeCCtxParams(params);
}

fn compress_stream_legacy(zcs: *mut ZSTD_CStream, src: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    let mut input = ZSTD_inBuffer {
        src: src.as_ptr().cast(),
        size: src.len(),
        pos: 0,
    };

    while input.pos < input.size {
        let mut out_buf = vec![0u8; cstream::ZSTD_CStreamOutSize()];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream(zcs, &mut output, &mut input);
        check_result(remaining, "ZSTD_compressStream");
        compressed.extend_from_slice(&out_buf[..output.pos]);
    }

    loop {
        let mut out_buf = vec![0u8; cstream::ZSTD_CStreamOutSize()];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_endStream(zcs, &mut output);
        check_result(remaining, "ZSTD_endStream");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    compressed
}

#[test]
fn compress_streaming_and_parameter_helpers_roundtrip() {
    let src = sample_bytes(192 * 1024 + 37);
    let compressible = compressible_sample(96 * 1024 + 29);
    let dict = dict_fixture();
    let cdict_ptr = cdict::ZSTD_createCDict(dict.as_ptr().cast(), dict.len(), 4);
    let bounds: ZSTD_bounds = params::ZSTD_cParam_getBounds(ZSTD_cParameter::ZSTD_c_strategy);
    let window_bounds = params::ZSTD_dParam_getBounds(ZSTD_dParameter::ZSTD_d_windowLogMax);
    let cparams: ZSTD_compressionParameters =
        params::ZSTD_getCParams(4, src.len() as u64, dict.len());
    let negative_cparams: ZSTD_compressionParameters = params::ZSTD_getCParams(-5, 0, 0);
    let negative_dict_cparams: ZSTD_compressionParameters = params::ZSTD_getCParams(-5, 0, 1);
    let level_one_cparams: ZSTD_compressionParameters = params::ZSTD_getCParams(1, 0, 0);
    let adjusted = params::ZSTD_adjustCParams(cparams, src.len() as u64, dict.len());
    let adjusted_small = params::ZSTD_adjustCParams(cparams, 1024, 512);
    let full_params: ZSTD_parameters = params::ZSTD_getParams(4, src.len() as u64, dict.len());
    let zcs = cstream::ZSTD_createCStream();
    let zcs2 = cstream::ZSTD_createCStream();
    let cctx_params_ptr = cctx_params::ZSTD_createCCtxParams();
    let mut checksum_flag = 0;
    let mut ldm_flag = 0;

    assert_eq!(bounds.error, 0);
    assert_eq!(window_bounds.error, 0);
    assert!(!cctx_params_ptr.is_null());
    check_result(params::ZSTD_checkCParams(cparams), "ZSTD_checkCParams");
    check_result(
        params::ZSTD_checkCParams(adjusted),
        "ZSTD_checkCParams(adjusted)",
    );
    check_result(
        params::ZSTD_checkCParams(adjusted_small),
        "ZSTD_checkCParams(adjusted_small)",
    );
    let level_bounds = params::ZSTD_cParam_getBounds(ZSTD_cParameter::ZSTD_c_compressionLevel);
    assert_eq!(level_bounds.error, 0);
    assert_eq!(level_bounds.lowerBound, params::ZSTD_minCLevel());
    assert!(level_bounds.lowerBound < 0);
    assert_eq!(level_bounds.upperBound, params::ZSTD_maxCLevel());
    let target_length_bounds = params::ZSTD_cParam_getBounds(ZSTD_cParameter::ZSTD_c_targetLength);
    assert_eq!(target_length_bounds.upperBound, ZSTD_BLOCKSIZE_MAX as c_int);
    assert_eq!(negative_cparams.strategy, ZSTD_strategy::ZSTD_fast);
    assert_eq!(negative_cparams.targetLength, 5);
    assert_eq!(negative_cparams.windowLog, 19);
    assert_eq!(negative_cparams.chainLog, 12);
    assert_eq!(negative_cparams.hashLog, 13);
    assert_eq!(negative_cparams.searchLog, 1);
    assert_eq!(negative_cparams.minMatch, 6);
    assert_ne!(negative_dict_cparams.windowLog, negative_cparams.windowLog);
    assert!(adjusted_small.windowLog < cparams.windowLog);
    assert!(adjusted_small.hashLog <= adjusted_small.windowLog + 1);
    assert!(adjusted_small.chainLog <= adjusted_small.windowLog);
    assert_ne!(
        negative_cparams.targetLength,
        level_one_cparams.targetLength
    );
    assert_ne!(negative_cparams.hashLog, level_one_cparams.hashLog);
    assert!(params::ZSTD_minCLevel() <= params::ZSTD_defaultCLevel());
    assert!(params::ZSTD_defaultCLevel() <= params::ZSTD_maxCLevel());
    assert!(!cdict_ptr.is_null());
    assert!(cstream::ZSTD_CStreamInSize() > 0);
    assert!(cstream::ZSTD_CStreamOutSize() > 0);
    assert!(cstream::ZSTD_sizeof_CStream(zcs) > 0);
    let cctx_level_one = cctx::ZSTD_estimateCCtxSize(1);
    let cctx_level_nineteen = cctx::ZSTD_estimateCCtxSize(19);
    let cctx_small_params = cctx::ZSTD_estimateCCtxSize_usingCParams(
        params::ZSTD_getCParams(1, 16 * 1024, 0),
    );
    let cctx_large_params = cctx::ZSTD_estimateCCtxSize_usingCParams(
        params::ZSTD_getCParams(19, ZSTD_CONTENTSIZE_UNKNOWN, 0),
    );
    let cstream_level_one = cstream::ZSTD_estimateCStreamSize(1);
    let cstream_level_nineteen = cstream::ZSTD_estimateCStreamSize(19);
    check_result(
        cctx_params::ZSTD_CCtxParams_init(cctx_params_ptr, 3),
        "ZSTD_CCtxParams_init(estimate)",
    );
    let cctx_params_default = cctx::ZSTD_estimateCCtxSize_usingCCtxParams(cctx_params_ptr);
    let cstream_params_default =
        cstream::ZSTD_estimateCStreamSize_usingCCtxParams(cctx_params_ptr);
    check_result(
        cctx_params_default,
        "ZSTD_estimateCCtxSize_usingCCtxParams(default)",
    );
    check_result(
        cstream_params_default,
        "ZSTD_estimateCStreamSize_usingCCtxParams(default)",
    );
    check_result(
        cctx_params::ZSTD_CCtxParams_setParameter(
            cctx_params_ptr,
            ZSTD_cParameter::ZSTD_c_hashLog,
            20,
        ),
        "ZSTD_CCtxParams_setParameter(hashLog estimate)",
    );
    let cctx_params_window = cctx::ZSTD_estimateCCtxSize_usingCCtxParams(cctx_params_ptr);
    let cstream_params_window =
        cstream::ZSTD_estimateCStreamSize_usingCCtxParams(cctx_params_ptr);
    check_result(
        cctx_params_window,
        "ZSTD_estimateCCtxSize_usingCCtxParams(hashLog)",
    );
    check_result(
        cstream_params_window,
        "ZSTD_estimateCStreamSize_usingCCtxParams(hashLog)",
    );
    assert!(cctx_level_nineteen > cctx_level_one);
    assert!(cctx_large_params > cctx_small_params);
    assert!(cstream_level_nineteen > cstream_level_one);
    assert!(cstream_level_nineteen > cctx_level_nineteen);
    assert!(cctx_params_window > cctx_params_default);
    assert!(cstream_params_window > cstream_params_default);
    check_result(
        cctx_params::ZSTD_CCtxParams_setParameter(
            cctx_params_ptr,
            ZSTD_cParameter::ZSTD_c_nbWorkers,
            1,
        ),
        "ZSTD_CCtxParams_setParameter(nbWorkers estimate)",
    );
    expect_error(
        cctx::ZSTD_estimateCCtxSize_usingCCtxParams(cctx_params_ptr),
        "ZSTD_estimateCCtxSize_usingCCtxParams(nbWorkers)",
    );
    expect_error(
        cstream::ZSTD_estimateCStreamSize_usingCCtxParams(cctx_params_ptr),
        "ZSTD_estimateCStreamSize_usingCCtxParams(nbWorkers)",
    );
    check_result(
        cctx_params::ZSTD_CCtxParams_setParameter(
            cctx_params_ptr,
            ZSTD_cParameter::ZSTD_c_nbWorkers,
            0,
        ),
        "ZSTD_CCtxParams_setParameter(nbWorkers reset estimate)",
    );

    check_result(
        cctx::ZSTD_CCtx_setParameter(
            zcs.cast(),
            ZSTD_cParameter::ZSTD_c_enableLongDistanceMatching,
            1,
        ),
        "ZSTD_CCtx_setParameter(ldm legacy stream)",
    );
    check_result(
        cctx::ZSTD_CCtx_getParameter(
            zcs.cast(),
            ZSTD_cParameter::ZSTD_c_enableLongDistanceMatching,
            &mut ldm_flag,
        ),
        "ZSTD_CCtx_getParameter(ldm legacy stream)",
    );
    assert_eq!(ldm_flag, 1);
    check_result(
        cctx::ZSTD_CCtx_setParameter(zcs.cast(), ZSTD_cParameter::ZSTD_c_checksumFlag, 1),
        "ZSTD_CCtx_setParameter(checksum legacy stream)",
    );
    check_result(cstream::ZSTD_initCStream(zcs, 3), "ZSTD_initCStream");
    check_result(
        cctx::ZSTD_CCtx_getParameter(
            zcs.cast(),
            ZSTD_cParameter::ZSTD_c_checksumFlag,
            &mut checksum_flag,
        ),
        "ZSTD_CCtx_getParameter(checksum legacy stream)",
    );
    check_result(
        cctx::ZSTD_CCtx_getParameter(
            zcs.cast(),
            ZSTD_cParameter::ZSTD_c_enableLongDistanceMatching,
            &mut ldm_flag,
        ),
        "ZSTD_CCtx_getParameter(ldm after initCStream)",
    );
    assert_eq!(checksum_flag, 1);
    assert_eq!(ldm_flag, 1);
    let hint_len = 32 * 1024;
    let mut hint_input = ZSTD_inBuffer {
        src: compressible[..hint_len].as_ptr().cast(),
        size: hint_len,
        pos: 0,
    };
    let mut hint_output_buf = vec![0u8; cctx::ZSTD_compressBound(hint_len)];
    let mut hint_output = ZSTD_outBuffer {
        dst: hint_output_buf.as_mut_ptr().cast(),
        size: hint_output_buf.len(),
        pos: 0,
    };
    let next_hint = cstream::ZSTD_compressStream(zcs, &mut hint_output, &mut hint_input);
    check_result(next_hint, "ZSTD_compressStream(hint)");
    assert_eq!(hint_input.pos, hint_len);
    assert_eq!(next_hint, cstream::ZSTD_CStreamInSize() - hint_len);
    check_result(cstream::ZSTD_initCStream(zcs, 3), "ZSTD_initCStream(hint restart)");

    let (compressed, _flushed_output) =
        compress_stream_legacy_flush_then_end(zcs, &compressible, 97);
    assert_eq!(frame_first_block_type(&compressed), 2);
    assert!(compressed.len() < compressible.len());
    assert!(frame_has_checksum(&compressed));
    decompress_exact(&compressed, &compressible);

    check_result(cstream::ZSTD_initCStream(zcs, 3), "ZSTD_initCStream(restart)");
    let (compressed, produced_before_end) =
        compress_stream_legacy_limited(zcs, &compressible, 97);
    assert!(
        produced_before_end,
        "ZSTD_compressStream withheld all output until end"
    );
    assert_eq!(frame_first_block_type(&compressed), 2);
    assert!(compressed.len() < compressible.len());
    assert!(frame_has_checksum(&compressed));
    decompress_exact(&compressed, &compressible);

    check_result(
        cstream::ZSTD_resetCStream(zcs, 0),
        "ZSTD_resetCStream(compat zero)",
    );
    let compressed = compress_stream_legacy(zcs, &src[..64 * 1024]);
    assert!(frame_has_checksum(&compressed));
    assert_eq!(frame_content_size(&compressed), ZSTD_CONTENTSIZE_UNKNOWN);
    decompress_exact(&compressed, &src[..64 * 1024]);

    check_result(
        cstream::ZSTD_initCStream_srcSize(zcs, 4, 0),
        "ZSTD_initCStream_srcSize(compat zero)",
    );
    let compressed = compress_stream_legacy(zcs, &src[..96 * 1024]);
    assert!(frame_has_checksum(&compressed));
    assert_eq!(frame_content_size(&compressed), ZSTD_CONTENTSIZE_UNKNOWN);
    decompress_exact(&compressed, &src[..96 * 1024]);

    check_result(
        cstream::ZSTD_initCStream_srcSize(zcs, 4, src.len() as u64),
        "ZSTD_initCStream_srcSize",
    );
    let compressed = compress_stream_legacy(zcs, &src);
    assert!(frame_has_checksum(&compressed));
    assert_eq!(frame_content_size(&compressed), src.len() as u64);
    decompress_exact(&compressed, &src);

    check_result(
        cstream::ZSTD_resetCStream(zcs, ZSTD_CONTENTSIZE_UNKNOWN),
        "ZSTD_resetCStream",
    );
    let compressed = compress_stream_legacy(zcs, &src[..64 * 1024]);
    assert!(frame_has_checksum(&compressed));
    assert_eq!(frame_content_size(&compressed), ZSTD_CONTENTSIZE_UNKNOWN);
    decompress_exact(&compressed, &src[..64 * 1024]);

    check_result(
        cstream::ZSTD_initCStream_usingDict(zcs, dict.as_ptr().cast(), dict.len(), 4),
        "ZSTD_initCStream_usingDict",
    );
    let compressed = compress_stream_legacy(zcs, &src);
    assert_eq!(
        zstd::common::frame::ZSTD_getDictID_fromFrame(compressed.as_ptr().cast(), compressed.len()),
        ddict::ZSTD_getDictID_fromDict(dict.as_ptr().cast(), dict.len())
    );
    assert_eq!(frame_first_block_type(&compressed), 2);
    assert!(
        compressed.len() < src.len(),
        "legacy usingDict stream failed to shrink the source"
    );
    assert!(frame_has_checksum(&compressed));
    upstream_decompress_using_dict_exact(&compressed, &dict, &src);

    check_result(
        cctx::ZSTD_CCtx_setParameter(zcs2.cast(), ZSTD_cParameter::ZSTD_c_checksumFlag, 1),
        "ZSTD_CCtx_setParameter(checksum legacy cdict stream)",
    );
    check_result(
        cdict::ZSTD_initCStream_usingCDict(zcs2, cdict_ptr),
        "ZSTD_initCStream_usingCDict",
    );
    let compressed = compress_stream_legacy(zcs2, &src);
    assert_eq!(
        zstd::common::frame::ZSTD_getDictID_fromFrame(compressed.as_ptr().cast(), compressed.len()),
        ddict::ZSTD_getDictID_fromDict(dict.as_ptr().cast(), dict.len())
    );
    assert_eq!(frame_first_block_type(&compressed), 2);
    assert!(
        compressed.len() < src.len(),
        "legacy usingCDict stream failed to shrink the source"
    );
    assert!(frame_has_checksum(&compressed));
    upstream_decompress_using_dict_exact(&compressed, &dict, &src);

    {
        let tuned_params = cctx_params::ZSTD_createCCtxParams();
        let tuned_cctx = cctx::ZSTD_createCCtx();
        let tuned_src = dict_biased_sample(&dict, (8 * 1024) + 17, 0x55AA_11CC);
        let mut tuned_window = 0;
        assert!(!tuned_params.is_null());
        assert!(!tuned_cctx.is_null());
        check_result(
            cctx_params::ZSTD_CCtxParams_init(tuned_params, 4),
            "ZSTD_CCtxParams_init",
        );
        check_result(
            cctx_params::ZSTD_CCtxParams_setParameter(
                tuned_params,
                ZSTD_cParameter::ZSTD_c_windowLog,
                14,
            ),
            "ZSTD_CCtxParams_setParameter(windowLog)",
        );
        let tuned_cdict = cdict::ZSTD_createCDict_advanced2(
            dict.as_ptr().cast(),
            dict.len(),
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef,
            ZSTD_dictContentType_e::ZSTD_dct_fullDict,
            tuned_params,
            ZSTD_customMem::default(),
        );
        assert!(!tuned_cdict.is_null());
        check_result(
            cdict::ZSTD_compressBegin_usingCDict(tuned_cctx, tuned_cdict),
            "ZSTD_compressBegin_usingCDict(tuned)",
        );
        assert_eq!(cblock::ZSTD_getBlockSize(tuned_cctx), 1usize << 14);
        check_result(
            cctx::ZSTD_CCtx_getParameter(
                tuned_cctx,
                ZSTD_cParameter::ZSTD_c_windowLog,
                &mut tuned_window,
            ),
            "ZSTD_CCtx_getParameter(windowLog tuned cdict)",
        );
        assert_eq!(tuned_window, 14);
        let mut tuned_out = vec![0u8; cctx::ZSTD_compressBound(tuned_src.len())];
        let tuned_size = cdict::ZSTD_compress_usingCDict_advanced(
            tuned_cctx,
            tuned_out.as_mut_ptr().cast(),
            tuned_out.len(),
            tuned_src.as_ptr().cast(),
            tuned_src.len(),
            tuned_cdict,
            ZSTD_frameParameters {
                contentSizeFlag: 1,
                checksumFlag: 0,
                noDictIDFlag: 0,
            },
        );
        check_result(tuned_size, "ZSTD_compress_usingCDict_advanced(tuned)");
        upstream_decompress_using_dict_exact(&tuned_out[..tuned_size], &dict, &tuned_src);
        cdict::ZSTD_freeCDict(tuned_cdict);
        cctx_params::ZSTD_freeCCtxParams(tuned_params);
        cctx::ZSTD_freeCCtx(tuned_cctx);
    }

    check_result(
        cstream::ZSTD_initCStream_advanced(
            zcs2,
            core::ptr::null(),
            0,
            ZSTD_parameters {
                cParams: adjusted,
                fParams: ZSTD_frameParameters {
                    contentSizeFlag: 0,
                    checksumFlag: 1,
                    noDictIDFlag: 1,
                },
            },
            0,
        ),
        "ZSTD_initCStream_advanced(compat zero)",
    );
    let compressed = compress_stream_legacy(zcs2, &src[..80 * 1024]);
    assert!(frame_has_checksum(&compressed));
    assert_eq!(frame_content_size(&compressed), ZSTD_CONTENTSIZE_UNKNOWN);
    decompress_exact(&compressed, &src[..80 * 1024]);

    check_result(
        cstream::ZSTD_initCStream_advanced(
            zcs2,
            core::ptr::null(),
            0,
            ZSTD_parameters {
                cParams: adjusted,
                fParams: ZSTD_frameParameters {
                    contentSizeFlag: full_params.fParams.contentSizeFlag,
                    checksumFlag: 1,
                    noDictIDFlag: 1,
                },
            },
            src.len() as u64,
        ),
        "ZSTD_initCStream_advanced",
    );
    let compressed = compress_stream_legacy(zcs2, &src);
    assert!(frame_has_checksum(&compressed));
    decompress_exact(&compressed, &src);

    check_result(
        cdict::ZSTD_initCStream_usingCDict_advanced(
            zcs2,
            cdict_ptr,
            ZSTD_frameParameters {
                contentSizeFlag: 1,
                checksumFlag: 1,
                noDictIDFlag: 0,
            },
            src.len() as u64,
        ),
        "ZSTD_initCStream_usingCDict_advanced",
    );
    let compressed = compress_stream_legacy(zcs2, &src);
    assert_eq!(
        zstd::common::frame::ZSTD_getDictID_fromFrame(compressed.as_ptr().cast(), compressed.len()),
        ddict::ZSTD_getDictID_fromDict(dict.as_ptr().cast(), dict.len())
    );
    assert_eq!(frame_first_block_type(&compressed), 2);
    assert!(frame_has_checksum(&compressed));
    upstream_decompress_using_dict_exact(&compressed, &dict, &src);

    {
        let cctx_ptr = cctx::ZSTD_createCCtx();
        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(stream2)",
        );
        check_result(
            cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_checksumFlag, 1),
            "ZSTD_CCtx_setParameter(checksum stream2)",
        );
        let (compressed, _flushed_output) =
            compress_stream2_continue_flush_then_end(cctx_ptr, &compressible, 113);
        assert_eq!(frame_first_block_type(&compressed), 2);
        assert!(compressed.len() < compressible.len());
        assert!(frame_has_checksum(&compressed));
        decompress_exact(&compressed, &compressible);
        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(stream2 flush restart)",
        );
        check_result(
            cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_checksumFlag, 1),
            "ZSTD_CCtx_setParameter(checksum stream2 restart)",
        );
        let (compressed, produced_before_end) =
            compress_stream2_continue_then_end(cctx_ptr, &src, 113);
        assert!(
            produced_before_end,
            "ZSTD_compressStream2 withheld all output until ZSTD_e_end"
        );
        decompress_exact(&compressed, &src);
        cctx::ZSTD_freeCCtx(cctx_ptr);
    }

    cdict::ZSTD_freeCDict(cdict_ptr);
    cctx_params::ZSTD_freeCCtxParams(cctx_params_ptr);
    cstream::ZSTD_freeCStream(zcs2);
    cstream::ZSTD_freeCStream(zcs);
}
