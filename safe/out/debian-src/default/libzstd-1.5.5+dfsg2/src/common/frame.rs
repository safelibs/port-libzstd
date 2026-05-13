use crate::{
    common::error::error_result,
    decompress::frame,
    ffi::types::{ZSTD_format_e, ZSTD_frameHeader},
};
use core::ffi::c_void;

pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = u64::MAX;
pub const ZSTD_CONTENTSIZE_ERROR: u64 = u64::MAX - 1;

#[no_mangle]
pub extern "C" fn ZSTD_getFrameContentSize(src: *const c_void, srcSize: usize) -> u64 {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return ZSTD_CONTENTSIZE_ERROR;
    };
    frame::get_frame_content_size(src)
}

#[no_mangle]
pub extern "C" fn ZSTD_getDecompressedSize(src: *const c_void, srcSize: usize) -> u64 {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return 0;
    };
    frame::get_decompressed_size(src)
}

#[no_mangle]
pub extern "C" fn ZSTD_findFrameCompressedSize(src: *const c_void, srcSize: usize) -> usize {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match frame::find_frame_compressed_size(src) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_findDecompressedSize(src: *const c_void, srcSize: usize) -> u64 {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return ZSTD_CONTENTSIZE_ERROR;
    };
    frame::find_decompressed_size(src)
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBound(src: *const c_void, srcSize: usize) -> u64 {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return ZSTD_CONTENTSIZE_ERROR;
    };
    frame::decompress_bound(src)
}

#[no_mangle]
pub extern "C" fn ZSTD_frameHeaderSize(src: *const c_void, srcSize: usize) -> usize {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match frame::parse_frame_header(src, ZSTD_format_e::ZSTD_f_zstd1) {
        Ok(frame::HeaderProbe::Need(size)) => size,
        Ok(frame::HeaderProbe::Header(header)) => header.headerSize as usize,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_getFrameHeader(
    zfhPtr: *mut ZSTD_frameHeader,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_getFrameHeader_advanced(zfhPtr, src, srcSize, ZSTD_format_e::ZSTD_f_zstd1)
}

#[no_mangle]
pub extern "C" fn ZSTD_getFrameHeader_advanced(
    zfhPtr: *mut ZSTD_frameHeader,
    src: *const c_void,
    srcSize: usize,
    format: ZSTD_format_e,
) -> usize {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match frame::parse_frame_header(src, format) {
        Ok(frame::HeaderProbe::Need(size)) => size,
        Ok(frame::HeaderProbe::Header(header)) => {
            if !zfhPtr.is_null() {
                // SAFETY: The caller provided a valid writable header pointer.
                unsafe {
                    *zfhPtr = header;
                }
            }
            0
        }
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_isFrame(buffer: *const c_void, size: usize) -> u32 {
    let Some(buffer) = crate::ffi::decompress::optional_src_slice(buffer, size) else {
        return 0;
    };
    frame::is_frame(buffer) as u32
}

#[no_mangle]
pub extern "C" fn ZSTD_isSkippableFrame(buffer: *const c_void, size: usize) -> u32 {
    let Some(buffer) = crate::ffi::decompress::optional_src_slice(buffer, size) else {
        return 0;
    };
    frame::is_skippable_frame(buffer) as u32
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressionMargin(src: *const c_void, srcSize: usize) -> usize {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match frame::decompression_margin(src) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromFrame(src: *const c_void, srcSize: usize) -> u32 {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return 0;
    };
    frame::get_dict_id_from_frame(src)
}
