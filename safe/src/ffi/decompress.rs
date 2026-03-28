use crate::ffi::types::{
    ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream, ZSTD_ErrorCode, ZSTD_ResetDirective, ZSTD_dParameter,
    ZSTD_format_e, ZSTD_frameHeader, ZSTD_inBuffer, ZSTD_nextInputType_e, ZSTD_outBuffer,
};
use core::ffi::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *mut c_char;
}

const RTLD_NOW: c_int = 0x0002;
const RTLD_LOCAL: c_int = 0x0000;

const LIBZSTD_CANDIDATES: &[&[u8]] = &[
    b"/lib/x86_64-linux-gnu/libzstd.so.1\0",
    b"/usr/lib/x86_64-linux-gnu/libzstd.so.1\0",
    b"/usr/lib64/libzstd.so.1\0",
    b"libzstd.so.1\0",
];

macro_rules! system_zstd_api {
    ($($field:ident : $ty:ty => $symbol:literal,)+) => {
        pub(crate) struct SystemZstdApi {
            _handle: *mut c_void,
            $(pub $field: $ty,)+
        }

        unsafe impl Send for SystemZstdApi {}
        unsafe impl Sync for SystemZstdApi {}

        fn load_system_api() -> Result<SystemZstdApi, String> {
            let handle = open_system_library()?;
            unsafe {
                Ok(SystemZstdApi {
                    _handle: handle,
                    $($field: load_symbol::<$ty>(handle, $symbol)?,)+
                })
            }
        }
    };
}

system_zstd_api! {
    version_number: unsafe extern "C" fn() -> c_uint => b"ZSTD_versionNumber\0",
    version_string: unsafe extern "C" fn() -> *const c_char => b"ZSTD_versionString\0",
    is_error: unsafe extern "C" fn(usize) -> c_uint => b"ZSTD_isError\0",
    get_error_name: unsafe extern "C" fn(usize) -> *const c_char => b"ZSTD_getErrorName\0",
    get_error_code: unsafe extern "C" fn(usize) -> ZSTD_ErrorCode => b"ZSTD_getErrorCode\0",
    get_error_string: unsafe extern "C" fn(ZSTD_ErrorCode) -> *const c_char => b"ZSTD_getErrorString\0",
    get_frame_content_size: unsafe extern "C" fn(*const c_void, usize) -> u64 => b"ZSTD_getFrameContentSize\0",
    get_decompressed_size: unsafe extern "C" fn(*const c_void, usize) -> u64 => b"ZSTD_getDecompressedSize\0",
    find_frame_compressed_size: unsafe extern "C" fn(*const c_void, usize) -> usize => b"ZSTD_findFrameCompressedSize\0",
    find_decompressed_size: unsafe extern "C" fn(*const c_void, usize) -> u64 => b"ZSTD_findDecompressedSize\0",
    decompress_bound: unsafe extern "C" fn(*const c_void, usize) -> u64 => b"ZSTD_decompressBound\0",
    frame_header_size: unsafe extern "C" fn(*const c_void, usize) -> usize => b"ZSTD_frameHeaderSize\0",
    get_frame_header: unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize) -> usize => b"ZSTD_getFrameHeader\0",
    get_frame_header_advanced: unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize, ZSTD_format_e) -> usize => b"ZSTD_getFrameHeader_advanced\0",
    is_frame: unsafe extern "C" fn(*const c_void, usize) -> c_uint => b"ZSTD_isFrame\0",
    is_skippable_frame: unsafe extern "C" fn(*const c_void, usize) -> c_uint => b"ZSTD_isSkippableFrame\0",
    read_skippable_frame: unsafe extern "C" fn(*mut c_void, usize, *mut c_uint, *const c_void, usize) -> usize => b"ZSTD_readSkippableFrame\0",
    decompression_margin: unsafe extern "C" fn(*const c_void, usize) -> usize => b"ZSTD_decompressionMargin\0",
    create_dctx: unsafe extern "C" fn() -> *mut ZSTD_DCtx => b"ZSTD_createDCtx\0",
    free_dctx: unsafe extern "C" fn(*mut ZSTD_DCtx) -> usize => b"ZSTD_freeDCtx\0",
    copy_dctx: unsafe extern "C" fn(*mut ZSTD_DCtx, *const ZSTD_DCtx) -> () => b"ZSTD_copyDCtx\0",
    dctx_reset: unsafe extern "C" fn(*mut ZSTD_DCtx, ZSTD_ResetDirective) -> usize => b"ZSTD_DCtx_reset\0",
    decompress_begin: unsafe extern "C" fn(*mut ZSTD_DCtx) -> usize => b"ZSTD_decompressBegin\0",
    decompress_begin_using_dict: unsafe extern "C" fn(*mut ZSTD_DCtx, *const c_void, usize) -> usize => b"ZSTD_decompressBegin_usingDict\0",
    decompress_begin_using_ddict: unsafe extern "C" fn(*mut ZSTD_DCtx, *const ZSTD_DDict) -> usize => b"ZSTD_decompressBegin_usingDDict\0",
    decompress_continue: unsafe extern "C" fn(*mut ZSTD_DCtx, *mut c_void, usize, *const c_void, usize) -> usize => b"ZSTD_decompressContinue\0",
    decompress_block: unsafe extern "C" fn(*mut ZSTD_DCtx, *mut c_void, usize, *const c_void, usize) -> usize => b"ZSTD_decompressBlock\0",
    next_src_size_to_decompress: unsafe extern "C" fn(*mut ZSTD_DCtx) -> usize => b"ZSTD_nextSrcSizeToDecompress\0",
    next_input_type: unsafe extern "C" fn(*mut ZSTD_DCtx) -> ZSTD_nextInputType_e => b"ZSTD_nextInputType\0",
    decoding_buffer_size_min: unsafe extern "C" fn(u64, u64) -> usize => b"ZSTD_decodingBufferSize_min\0",
    dctx_set_parameter: unsafe extern "C" fn(*mut ZSTD_DCtx, ZSTD_dParameter, c_int) -> usize => b"ZSTD_DCtx_setParameter\0",
    dctx_get_parameter: unsafe extern "C" fn(*mut ZSTD_DCtx, ZSTD_dParameter, *mut c_int) -> usize => b"ZSTD_DCtx_getParameter\0",
    dctx_set_format: unsafe extern "C" fn(*mut ZSTD_DCtx, ZSTD_format_e) -> usize => b"ZSTD_DCtx_setFormat\0",
    dctx_set_max_window_size: unsafe extern "C" fn(*mut ZSTD_DCtx, usize) -> usize => b"ZSTD_DCtx_setMaxWindowSize\0",
    dctx_load_dictionary: unsafe extern "C" fn(*mut ZSTD_DCtx, *const c_void, usize) -> usize => b"ZSTD_DCtx_loadDictionary\0",
    dctx_ref_ddict: unsafe extern "C" fn(*mut ZSTD_DCtx, *const ZSTD_DDict) -> usize => b"ZSTD_DCtx_refDDict\0",
    dctx_ref_prefix: unsafe extern "C" fn(*mut ZSTD_DCtx, *const c_void, usize) -> usize => b"ZSTD_DCtx_refPrefix\0",
    decompress: unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize => b"ZSTD_decompress\0",
    decompress_dctx: unsafe extern "C" fn(*mut ZSTD_DCtx, *mut c_void, usize, *const c_void, usize) -> usize => b"ZSTD_decompressDCtx\0",
    decompress_using_dict: unsafe extern "C" fn(*mut ZSTD_DCtx, *mut c_void, usize, *const c_void, usize, *const c_void, usize) -> usize => b"ZSTD_decompress_usingDict\0",
    decompress_using_ddict: unsafe extern "C" fn(*mut ZSTD_DCtx, *mut c_void, usize, *const c_void, usize, *const ZSTD_DDict) -> usize => b"ZSTD_decompress_usingDDict\0",
    create_ddict: unsafe extern "C" fn(*const c_void, usize) -> *mut ZSTD_DDict => b"ZSTD_createDDict\0",
    free_ddict: unsafe extern "C" fn(*mut ZSTD_DDict) -> usize => b"ZSTD_freeDDict\0",
    get_dict_id_from_dict: unsafe extern "C" fn(*const c_void, usize) -> c_uint => b"ZSTD_getDictID_fromDict\0",
    get_dict_id_from_ddict: unsafe extern "C" fn(*const ZSTD_DDict) -> c_uint => b"ZSTD_getDictID_fromDDict\0",
    get_dict_id_from_frame: unsafe extern "C" fn(*const c_void, usize) -> c_uint => b"ZSTD_getDictID_fromFrame\0",
    create_dstream: unsafe extern "C" fn() -> *mut ZSTD_DStream => b"ZSTD_createDStream\0",
    free_dstream: unsafe extern "C" fn(*mut ZSTD_DStream) -> usize => b"ZSTD_freeDStream\0",
    init_dstream: unsafe extern "C" fn(*mut ZSTD_DStream) -> usize => b"ZSTD_initDStream\0",
    reset_dstream: unsafe extern "C" fn(*mut ZSTD_DStream) -> usize => b"ZSTD_resetDStream\0",
    init_dstream_using_dict: unsafe extern "C" fn(*mut ZSTD_DStream, *const c_void, usize) -> usize => b"ZSTD_initDStream_usingDict\0",
    init_dstream_using_ddict: unsafe extern "C" fn(*mut ZSTD_DStream, *const ZSTD_DDict) -> usize => b"ZSTD_initDStream_usingDDict\0",
    decompress_stream: unsafe extern "C" fn(*mut ZSTD_DStream, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize => b"ZSTD_decompressStream\0",
    dstream_in_size: unsafe extern "C" fn() -> usize => b"ZSTD_DStreamInSize\0",
    dstream_out_size: unsafe extern "C" fn() -> usize => b"ZSTD_DStreamOutSize\0",
    sizeof_dctx: unsafe extern "C" fn(*const ZSTD_DCtx) -> usize => b"ZSTD_sizeof_DCtx\0",
    sizeof_dstream: unsafe extern "C" fn(*const ZSTD_DStream) -> usize => b"ZSTD_sizeof_DStream\0",
    sizeof_ddict: unsafe extern "C" fn(*const ZSTD_DDict) -> usize => b"ZSTD_sizeof_DDict\0",
}

static SYSTEM_ZSTD_API: OnceLock<Result<SystemZstdApi, String>> = OnceLock::new();

pub(crate) fn api() -> &'static SystemZstdApi {
    match SYSTEM_ZSTD_API.get_or_init(load_system_api) {
        Ok(api) => api,
        Err(message) => {
            eprintln!("libzstd-safe: failed to load system libzstd: {message}");
            std::process::abort();
        }
    }
}

fn open_system_library() -> Result<*mut c_void, String> {
    let mut last_error = String::from("no libzstd candidates attempted");
    for candidate in LIBZSTD_CANDIDATES {
        let handle = unsafe { dlopen(candidate.as_ptr().cast(), RTLD_NOW | RTLD_LOCAL) };
        if !handle.is_null() {
            return Ok(handle);
        }
        last_error = format!("{:?}: {}", display_bytes(candidate), dl_error_message());
    }
    Err(last_error)
}

unsafe fn load_symbol<T: Copy>(handle: *mut c_void, symbol: &'static [u8]) -> Result<T, String> {
    let ptr = unsafe { dlsym(handle, symbol.as_ptr().cast()) };
    if ptr.is_null() {
        return Err(format!(
            "missing symbol {}: {}",
            display_bytes(symbol),
            dl_error_message()
        ));
    }
    Ok(unsafe { core::mem::transmute_copy(&ptr) })
}

fn dl_error_message() -> String {
    let err = unsafe { dlerror() };
    if err.is_null() {
        "unknown dlopen error".to_string()
    } else {
        unsafe { std::ffi::CStr::from_ptr(err) }
            .to_string_lossy()
            .into_owned()
    }
}

fn display_bytes(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("<non-utf8>")
}
