use crate::{
    ffi::{
        compress::{stream_pending_bytes, with_cctx_ref, EncoderContext},
        types::{ZSTD_CCtx, ZSTD_frameProgression},
    },
    threading::pool::configured_worker_count,
};

fn mt_job_size(cctx: &EncoderContext) -> usize {
    let block_size = cctx.frame_block_size().max(1);
    match usize::try_from(cctx.job_size).ok().filter(|size| *size > 0) {
        Some(job_size) => job_size.max(block_size),
        None => block_size,
    }
}

fn mt_started_jobs(cctx: &EncoderContext, job_size: usize) -> usize {
    if cctx.stream.input.is_empty() {
        0
    } else {
        cctx.stream.input.len().div_ceil(job_size)
    }
}

fn mt_active_workers(cctx: &EncoderContext, job_size: usize, workers: usize) -> usize {
    if workers == 0 || cctx.stream.frame_finished {
        return 0;
    }

    let emitted = cctx.stream.emitted_input.min(cctx.stream.input.len());
    let buffered = cctx.stream.input.len().saturating_sub(emitted);
    if buffered == 0 {
        0
    } else {
        buffered.div_ceil(job_size).min(workers).max(1)
    }
}

fn mt_consumed_bytes(cctx: &EncoderContext, job_size: usize, active_workers: usize) -> usize {
    let emitted = cctx.stream.emitted_input.min(cctx.stream.input.len());
    if active_workers == 0 {
        return emitted;
    }

    let buffered = cctx.stream.input.len().saturating_sub(emitted);
    let inflight_capacity = job_size.saturating_mul(active_workers);
    let inflight = buffered.min(inflight_capacity);
    let queued = buffered.saturating_sub(inflight);
    emitted
        .saturating_add(queued)
        .saturating_add(inflight / 2)
        .min(cctx.stream.input.len())
}

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
            let job_size = mt_job_size(cctx);
            let active_workers = mt_active_workers(cctx, job_size, workers);
            (
                mt_consumed_bytes(cctx, job_size, active_workers) as u64,
                mt_started_jobs(cctx, job_size) as u32,
                active_workers as u32,
            )
        };
        Ok(ZSTD_frameProgression {
            ingested,
            consumed: consumed.min(ingested),
            produced: cctx.stream.produced_total as u64,
            flushed: cctx.stream.flushed_total as u64,
            currentJobID: current_job_id,
            nbActiveWorkers: nb_active_workers,
        })
    })
    .unwrap_or_default()
}
