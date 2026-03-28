use crate::ffi::types::{ZSTD_CCtx, ZSTD_frameProgression};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_toFlushNow"]
    fn internal_ZSTD_toFlushNow(cctx: *mut ZSTD_CCtx) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_getFrameProgression"]
    fn internal_ZSTD_getFrameProgression(cctx: *const ZSTD_CCtx) -> ZSTD_frameProgression;
}

#[no_mangle]
pub extern "C" fn ZSTD_toFlushNow(cctx: *mut ZSTD_CCtx) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_toFlushNow(cctx) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getFrameProgression(cctx: *const ZSTD_CCtx) -> ZSTD_frameProgression {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_getFrameProgression(cctx) }
}
