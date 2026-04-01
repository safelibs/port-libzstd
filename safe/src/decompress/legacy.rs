use crate::{
    common::error::{decode_error, is_error_result},
    ffi::types::{ZSTD_ErrorCode, ZSTD_inBuffer, ZSTD_outBuffer},
};
use core::ffi::c_void;

// Legacy v0.5-v0.7 decode is the only decompression-side C bridge left; modern
// decode stays within the Rust-owned shared library.

pub const ZSTD_LEGACY_SUPPORT: u32 = 5;

unsafe extern "C" {
    fn libzstd_safe_legacy_support() -> u32;
    fn libzstd_safe_is_legacy(src: *const c_void, srcSize: usize) -> u32;
    fn libzstd_safe_get_decompressed_size_legacy(src: *const c_void, srcSize: usize) -> u64;
    fn libzstd_safe_decompress_legacy(
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        compressedSize: usize,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn libzstd_safe_find_frame_compressed_size_legacy(src: *const c_void, srcSize: usize) -> usize;
    fn libzstd_safe_find_decompressed_bound_legacy(src: *const c_void, srcSize: usize) -> u64;
    fn libzstd_safe_free_legacy_stream(legacyContext: *mut c_void, version: u32) -> usize;
    fn libzstd_safe_init_legacy_stream(
        legacyContext: *mut *mut c_void,
        prevVersion: u32,
        newVersion: u32,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
    fn libzstd_safe_decompress_legacy_stream(
        legacyContext: *mut c_void,
        version: u32,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> usize;
}

pub(crate) fn supported_version(src: &[u8]) -> Option<u32> {
    if src.len() < 4 {
        return None;
    }
    debug_assert_eq!(
        // SAFETY: Local wrapper has no side effects and takes no arguments.
        unsafe { libzstd_safe_legacy_support() },
        ZSTD_LEGACY_SUPPORT
    );
    // SAFETY: `src` is a valid slice and we pass its pointer/length unchanged.
    let version = unsafe { libzstd_safe_is_legacy(src.as_ptr().cast(), src.len()) };
    match version {
        5..=7 => Some(version),
        _ => None,
    }
}

pub(crate) fn get_decompressed_size(src: &[u8]) -> u64 {
    // SAFETY: `src` is a valid slice and we pass its pointer/length unchanged.
    unsafe { libzstd_safe_get_decompressed_size_legacy(src.as_ptr().cast(), src.len()) }
}

pub(crate) fn find_frame_compressed_size(src: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    // SAFETY: `src` is a valid slice and we pass its pointer/length unchanged.
    let result =
        unsafe { libzstd_safe_find_frame_compressed_size_legacy(src.as_ptr().cast(), src.len()) };
    if result == 0 || !is_error_result(result) {
        return Ok(result);
    }
    Err(decode_error(result))
}

pub(crate) fn find_decompressed_bound(src: &[u8]) -> u64 {
    // SAFETY: `src` is a valid slice and we pass its pointer/length unchanged.
    unsafe { libzstd_safe_find_decompressed_bound_legacy(src.as_ptr().cast(), src.len()) }
}

pub(crate) fn decompress(
    dst: &mut [u8],
    src: &[u8],
    dict: Option<&[u8]>,
) -> Result<usize, ZSTD_ErrorCode> {
    let (dict_ptr, dict_len) = dict
        .map(|bytes| (bytes.as_ptr().cast(), bytes.len()))
        .unwrap_or((core::ptr::null(), 0));
    // SAFETY: The slices remain alive for the duration of the call and point to valid buffers.
    let result = unsafe {
        libzstd_safe_decompress_legacy(
            dst.as_mut_ptr().cast(),
            dst.len(),
            src.as_ptr().cast(),
            src.len(),
            dict_ptr,
            dict_len,
        )
    };
    if crate::common::error::is_error_result(result) {
        Err(decode_error(result))
    } else {
        Ok(result)
    }
}

pub(crate) fn free_stream_context(
    legacy_context: *mut c_void,
    version: u32,
) -> Result<(), ZSTD_ErrorCode> {
    if legacy_context.is_null() || version == 0 {
        return Ok(());
    }
    // SAFETY: The context and version are owned by the caller and originate from the matching init call.
    let result = unsafe { libzstd_safe_free_legacy_stream(legacy_context, version) };
    if result == 0 || !is_error_result(result) {
        Ok(())
    } else {
        Err(decode_error(result))
    }
}

pub(crate) fn init_stream_context(
    legacy_context: &mut *mut c_void,
    prev_version: u32,
    new_version: u32,
    dict: Option<&[u8]>,
) -> Result<(), ZSTD_ErrorCode> {
    let (dict_ptr, dict_len) = dict
        .map(|bytes| (bytes.as_ptr().cast(), bytes.len()))
        .unwrap_or((core::ptr::null(), 0));
    // SAFETY: The pointers remain valid for the duration of the call and `legacy_context` is a valid out-parameter.
    let result = unsafe {
        libzstd_safe_init_legacy_stream(
            legacy_context,
            prev_version,
            new_version,
            dict_ptr,
            dict_len,
        )
    };
    if result == 0 || !is_error_result(result) {
        Ok(())
    } else {
        Err(decode_error(result))
    }
}

pub(crate) fn decompress_stream(
    legacy_context: *mut c_void,
    version: u32,
    output: &mut ZSTD_outBuffer,
    input: &mut ZSTD_inBuffer,
) -> Result<usize, ZSTD_ErrorCode> {
    // SAFETY: The context pointer was created by `init_stream_context` and the buffers point to caller-owned memory.
    let result = unsafe {
        libzstd_safe_decompress_legacy_stream(
            legacy_context,
            version,
            output as *mut ZSTD_outBuffer,
            input as *mut ZSTD_inBuffer,
        )
    };
    if result == 0 || !is_error_result(result) {
        Ok(result)
    } else {
        Err(decode_error(result))
    }
}
