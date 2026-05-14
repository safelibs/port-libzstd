use crate::{
    common::error::error_result,
    ffi::{
        compress::{to_result, with_cctx_mut, EncoderContext},
        types::{ZSTD_CCtx, ZSTD_ErrorCode, ZSTD_threadPool},
    },
    threading::job_queue::{JobError, JobHandle, JobQueue},
};

#[derive(Debug)]
pub(crate) struct ThreadPoolState {
    workers: usize,
    queue: JobQueue,
}

impl ThreadPoolState {
    fn new(workers: usize) -> Self {
        Self {
            workers,
            queue: JobQueue::new(workers),
        }
    }

    pub(crate) fn workers(&self) -> usize {
        self.workers
    }

    pub(crate) fn submit<F, T>(&self, job: F) -> Result<JobHandle<T>, JobError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.queue.submit(job)
    }
}

impl Clone for ThreadPoolState {
    fn clone(&self) -> Self {
        Self::new(self.workers)
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
    if let Some(workers) =
        with_thread_pool_ref(cctx.thread_pool, |pool| configured.min(pool.workers()))
    {
        return workers.max(1);
    }

    cctx.owned_thread_pool
        .as_ref()
        .map_or(configured, |pool| configured.min(pool.workers()))
        .max(1)
}

pub(crate) fn submit_job<F, T>(
    cctx: &mut EncoderContext,
    job: F,
) -> Result<JobHandle<T>, ZSTD_ErrorCode>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let workers = configured_worker_count(cctx);
    if workers == 0 {
        return Err(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    if !cctx.thread_pool.is_null() {
        let pool = unsafe { &*cctx.thread_pool.cast::<ThreadPoolState>() };
        return pool
            .submit(job)
            .map_err(|_| ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    let requested = usize::try_from(cctx.nb_workers).unwrap_or(1).max(1);
    let pool = cctx
        .owned_thread_pool
        .get_or_insert_with(|| ThreadPoolState::new(requested));
    if pool.workers() != workers {
        *pool = ThreadPoolState::new(workers);
    }

    pool.submit(job)
        .map_err(|_| ZSTD_ErrorCode::ZSTD_error_GENERIC)
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
            cctx.owned_thread_pool = None;
            Ok(0)
        }));
    }

    if with_thread_pool_ref(pool, |_| ()).is_none() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.thread_pool = pool;
        cctx.owned_thread_pool = None;
        Ok(0)
    }))
}
