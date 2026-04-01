use crate::ffi::{
    compress::{to_result, with_cctx_mut},
    types::{ZSTD_CCtx, ZSTD_threadPool},
};

#[derive(Debug)]
struct ThreadPoolStub {
    _num_threads: usize,
}

#[no_mangle]
pub extern "C" fn ZSTD_createThreadPool(numThreads: usize) -> *mut ZSTD_threadPool {
    Box::into_raw(Box::new(ThreadPoolStub {
        _num_threads: numThreads.max(1),
    }))
    .cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeThreadPool(pool: *mut ZSTD_threadPool) {
    if pool.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(pool.cast::<ThreadPoolStub>()));
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refThreadPool(
    cctx: *mut ZSTD_CCtx,
    pool: *mut ZSTD_threadPool,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.thread_pool = pool;
        Ok(0)
    }))
}
