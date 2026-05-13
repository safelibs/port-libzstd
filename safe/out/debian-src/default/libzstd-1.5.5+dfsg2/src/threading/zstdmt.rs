use crate::{
    ffi::{
        compress::{mt_job_size, stream_pending_bytes, with_cctx_ref, EncoderContext},
        types::{ZSTD_CCtx, ZSTD_frameProgression},
    },
    threading::pool::configured_worker_count,
};

fn mt_progression_job_size(cctx: &EncoderContext) -> usize {
    let block_size = cctx.frame_block_size().max(1);
    match usize::try_from(cctx.job_size).ok().filter(|size| *size > 0) {
        Some(job_size) => job_size.max(block_size),
        None => mt_job_size(cctx),
    }
}

fn mt_started_jobs(cctx: &EncoderContext, job_size: usize) -> usize {
    let ingested = cctx.stream.input.len();
    let full_jobs = ingested / job_size;
    if cctx.stream.frame_finished && ingested % job_size != 0 {
        full_jobs.saturating_add(1)
    } else {
        full_jobs
    }
}

fn mt_active_workers(cctx: &EncoderContext, job_size: usize, workers: usize) -> usize {
    if workers == 0 {
        return 0;
    }

    let started_jobs = mt_started_jobs(cctx, job_size);
    if started_jobs == 0 {
        0
    } else {
        let emitted = cctx.stream.emitted_input.min(cctx.stream.input.len());
        let has_unfinished_work = cctx.stream.mt_handoff_pending
            || stream_pending_bytes(cctx) != 0
            || emitted < cctx.stream.input.len()
            || !cctx.stream.frame_finished;
        if has_unfinished_work {
            started_jobs.min(workers).max(1)
        } else {
            0
        }
    }
}

fn mt_consumed_bytes(cctx: &EncoderContext, job_size: usize, active_workers: usize) -> usize {
    let started_bytes = cctx
        .stream
        .input
        .len()
        .min(mt_started_jobs(cctx, job_size).saturating_mul(job_size));
    if active_workers == 0 {
        return cctx
            .stream
            .emitted_input
            .min(started_bytes)
            .min(cctx.stream.input.len());
    }

    started_bytes
        .saturating_sub(job_size.saturating_mul(active_workers) / 2)
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
            let job_size = mt_progression_job_size(cctx);
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
