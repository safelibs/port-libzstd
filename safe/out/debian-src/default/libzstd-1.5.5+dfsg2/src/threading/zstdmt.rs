use crate::{
    ffi::{
        compress::{stream_pending_bytes, with_cctx_ref},
        types::{ZSTD_CCtx, ZSTD_frameProgression},
    },
    threading::pool::configured_worker_count,
};

#[no_mangle]
pub extern "C" fn ZSTD_toFlushNow(cctx: *mut ZSTD_CCtx) -> usize {
    with_cctx_ref(cctx.cast_const(), |cctx| {
        if configured_worker_count(cctx) == 0 {
            return Ok(0);
        }
        Ok(stream_pending_bytes(cctx))
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn ZSTD_getFrameProgression(cctx: *const ZSTD_CCtx) -> ZSTD_frameProgression {
    with_cctx_ref(cctx, |cctx| {
        let ingested = cctx.stream.input.len() as u64;
        let workers = configured_worker_count(cctx);
        let (consumed, current_job_id, nb_active_workers) = if workers == 0 {
            (
                cctx.stream.emitted_input.min(cctx.stream.input.len()) as u64,
                0,
                0,
            )
        } else {
            let active_workers = cctx.stream.mt_active_jobs.min(workers);
            let current_job_id = if cctx.stream.mt_started_jobs == 0 {
                0
            } else {
                cctx.stream.mt_started_jobs.saturating_add(1)
            };
            (
                cctx.stream.emitted_input.min(cctx.stream.input.len()) as u64,
                current_job_id as u32,
                active_workers as u32,
            )
        };
        Ok(ZSTD_frameProgression {
            ingested,
            consumed: consumed.min(ingested),
            produced: cctx.stream.flushed_total as u64,
            flushed: cctx.stream.flushed_total as u64,
            currentJobID: current_job_id,
            nbActiveWorkers: nb_active_workers,
        })
    })
    .unwrap_or_default()
}
