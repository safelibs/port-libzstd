use crate::ffi::types::{ZSTD_CCtx, ZSTD_threadPool};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_createThreadPool"]
    fn internal_ZSTD_createThreadPool(numThreads: usize) -> *mut ZSTD_threadPool;
    #[link_name = "libzstd_safe_internal_ZSTD_freeThreadPool"]
    fn internal_ZSTD_freeThreadPool(pool: *mut ZSTD_threadPool);
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_refThreadPool"]
    fn internal_ZSTD_CCtx_refThreadPool(
        cctx: *mut ZSTD_CCtx,
        pool: *mut ZSTD_threadPool,
    ) -> usize;
}

#[no_mangle]
pub extern "C" fn ZSTD_createThreadPool(numThreads: usize) -> *mut ZSTD_threadPool {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_createThreadPool(numThreads) }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeThreadPool(pool: *mut ZSTD_threadPool) {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_freeThreadPool(pool) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refThreadPool(
    cctx: *mut ZSTD_CCtx,
    pool: *mut ZSTD_threadPool,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtx_refThreadPool(cctx, pool) }
}
