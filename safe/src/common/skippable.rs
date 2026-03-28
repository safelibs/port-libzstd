use crate::{
    common::error::error_result,
    decompress::frame::{self, ZSTD_SKIPPABLEHEADERSIZE},
    ffi::types::ZSTD_ErrorCode,
};
use core::ffi::{c_uint, c_void};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_writeSkippableFrame"]
    fn internal_ZSTD_writeSkippableFrame(
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        magicVariant: c_uint,
    ) -> usize;
}

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

    let payload_size = u32::from_le_bytes(src[4..8].try_into().expect("slice length checked")) as usize;
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
        unsafe { *magicVariant = magic - frame::ZSTD_MAGIC_SKIPPABLE_START; }
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
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_writeSkippableFrame(dst, dstCapacity, src, srcSize, magicVariant) }
}
