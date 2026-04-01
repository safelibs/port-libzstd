use crate::{
    common::error::error_result,
    decompress::frame::{self, ZSTD_SKIPPABLEHEADERSIZE},
    ffi::types::ZSTD_ErrorCode,
};
use core::ffi::{c_uint, c_void};

#[no_mangle]
pub extern "C" fn ZSTD_readSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    magicVariant: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    if src.len() < ZSTD_SKIPPABLEHEADERSIZE || !frame::is_skippable_frame(src) {
        return error_result(ZSTD_ErrorCode::ZSTD_error_frameParameter_unsupported);
    }

    let payload_size =
        u32::from_le_bytes(src[4..8].try_into().expect("slice length checked")) as usize;
    let total_size = payload_size + ZSTD_SKIPPABLEHEADERSIZE;
    if total_size > src.len() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }
    if payload_size > dstCapacity {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    if !magicVariant.is_null() {
        let magic = u32::from_le_bytes(src[..4].try_into().expect("slice length checked"));
        // SAFETY: The caller provided a valid optional output pointer.
        unsafe {
            *magicVariant = magic - frame::ZSTD_MAGIC_SKIPPABLE_START;
        }
    }
    if payload_size > 0 && !dst.is_null() {
        // SAFETY: The caller provides a writable buffer with `dstCapacity >= payload_size`.
        unsafe {
            core::ptr::copy_nonoverlapping(
                src[ZSTD_SKIPPABLEHEADERSIZE..].as_ptr(),
                dst.cast::<u8>(),
                payload_size,
            );
        }
    }
    payload_size
}

#[no_mangle]
pub extern "C" fn ZSTD_writeSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    magicVariant: c_uint,
) -> usize {
    let total_size = ZSTD_SKIPPABLEHEADERSIZE.saturating_add(srcSize);
    if dst.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstBuffer_null);
    }
    if total_size > dstCapacity {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    if srcSize != 0 && src.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    }

    let dst = unsafe { core::slice::from_raw_parts_mut(dst.cast::<u8>(), dstCapacity) };
    let magic = frame::ZSTD_MAGIC_SKIPPABLE_START.saturating_add(magicVariant);
    dst[..4].copy_from_slice(&magic.to_le_bytes());
    dst[4..8].copy_from_slice(&(srcSize as u32).to_le_bytes());
    if srcSize != 0 {
        let src = unsafe { core::slice::from_raw_parts(src.cast::<u8>(), srcSize) };
        dst[ZSTD_SKIPPABLEHEADERSIZE..total_size].copy_from_slice(src);
    }
    total_size
}
