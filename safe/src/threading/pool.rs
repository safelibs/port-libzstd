use crate::{
    common::error::error_result,
    ffi::{
        compress::{to_result, with_cctx_mut, EncoderContext},
        types::{ZSTD_CCtx, ZSTD_ErrorCode, ZSTD_threadPool},
    },
};

#[derive(Debug)]
pub(crate) struct ThreadPoolState {
    workers: usize,
}

impl ThreadPoolState {
    fn new(workers: usize) -> Self {
        Self { workers }
    }

    pub(crate) fn workers(&self) -> usize {
        self.workers
    }
}

pub(crate) fn with_thread_pool_ref<T>(
    pool: *mut ZSTD_threadPool,
    f: impl FnOnce(&ThreadPoolState) -> T,
) -> Option<T> {
    if pool.is_null() {
        return None;
    }
    Some(f(unsafe { &*pool.cast::<ThreadPoolState>() }))
}

pub(crate) fn configured_worker_count(cctx: &EncoderContext) -> usize {
    if cctx.nb_workers <= 0 {
        return 0;
    }

    let configured = cctx.nb_workers as usize;
    with_thread_pool_ref(cctx.thread_pool, |pool| configured.min(pool.workers()))
        .unwrap_or(configured)
        .max(1)
}

#[no_mangle]
pub extern "C" fn ZSTD_createThreadPool(numThreads: usize) -> *mut ZSTD_threadPool {
    if numThreads == 0 {
        return core::ptr::null_mut();
    }
    Box::into_raw(Box::new(ThreadPoolState::new(numThreads))).cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeThreadPool(pool: *mut ZSTD_threadPool) {
    if pool.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(pool.cast::<ThreadPoolState>()));
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refThreadPool(
    cctx: *mut ZSTD_CCtx,
    pool: *mut ZSTD_threadPool,
) -> usize {
    if pool.is_null() {
        return to_result(with_cctx_mut(cctx, |cctx| {
            cctx.thread_pool = core::ptr::null_mut();
            Ok(0)
        }));
    }

    if with_thread_pool_ref(pool, |_| ()).is_none() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.thread_pool = pool;
        Ok(0)
    }))
}
