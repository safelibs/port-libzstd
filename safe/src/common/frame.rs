use crate::ffi::{
    decompress::api,
    types::{ZSTD_format_e, ZSTD_frameHeader},
};
use core::ffi::c_void;

pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = u64::MAX;
pub const ZSTD_CONTENTSIZE_ERROR: u64 = u64::MAX - 1;

#[no_mangle]
pub extern "C" fn ZSTD_getFrameContentSize(src: *const c_void, srcSize: usize) -> u64 {
    unsafe { (api().get_frame_content_size)(src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getDecompressedSize(src: *const c_void, srcSize: usize) -> u64 {
    unsafe { (api().get_decompressed_size)(src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_findFrameCompressedSize(src: *const c_void, srcSize: usize) -> usize {
    if !src.is_null() {
        let legacy = unsafe {
            let src = std::slice::from_raw_parts(src.cast::<u8>(), srcSize);
            crate::decompress::legacy::frame_size(src)
        };
        if let Some(size) = legacy {
            return size;
        }
    }
    unsafe { (api().find_frame_compressed_size)(src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_findDecompressedSize(src: *const c_void, srcSize: usize) -> u64 {
    unsafe { (api().find_decompressed_size)(src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBound(src: *const c_void, srcSize: usize) -> u64 {
    unsafe { (api().decompress_bound)(src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    unsafe { (api().frame_header_size)(src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getFrameHeader(
    zfhPtr: *mut ZSTD_frameHeader,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    unsafe { (api().get_frame_header)(zfhPtr, src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getFrameHeader_advanced(
    zfhPtr: *mut ZSTD_frameHeader,
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    unsafe { (api().get_frame_header_advanced)(zfhPtr, src, srcSize, format) }
}

#[no_mangle]
pub extern "C" fn ZSTD_isFrame(buffer: *const c_void, size: usize) -> u32 {
    if !buffer.is_null() {
        let is_legacy = unsafe {
            let buffer = std::slice::from_raw_parts(buffer.cast::<u8>(), size);
            crate::decompress::legacy::is_legacy_frame(buffer)
        };
        if is_legacy {
            return 1;
        }
    }
    unsafe { (api().is_frame)(buffer, size) }
}

#[no_mangle]
pub extern "C" fn ZSTD_isSkippableFrame(buffer: *const c_void, size: usize) -> u32 {
    unsafe { (api().is_skippable_frame)(buffer, size) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressionMargin(src: *const c_void, srcSize: usize) -> usize {
    unsafe { (api().decompression_margin)(src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromFrame(src: *const c_void, srcSize: usize) -> u32 {
    unsafe { (api().get_dict_id_from_frame)(src, srcSize) }
}
