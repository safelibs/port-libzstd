use crate::ffi::{
    advanced::{forward_extern, forward_extern_void, null_mut},
    types::{ZSTD_CCtx, ZSTD_threadPool},
};

forward_extern! {
    pub fn ZSTD_createThreadPool(numThreads: usize) -> *mut ZSTD_threadPool => null_mut()
}

forward_extern_void! {
    pub fn ZSTD_freeThreadPool(pool: *mut ZSTD_threadPool)
}

forward_extern! {
    pub fn ZSTD_CCtx_refThreadPool(
        cctx: *mut ZSTD_CCtx,
        pool: *mut ZSTD_threadPool,
    ) -> usize => crate::ffi::compress::generic_error()
}
