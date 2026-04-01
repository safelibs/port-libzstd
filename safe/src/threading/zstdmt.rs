use crate::ffi::{
    compress::with_cctx_ref,
    types::{ZSTD_CCtx, ZSTD_frameProgression},
};

#[no_mangle]
pub extern "C" fn ZSTD_toFlushNow(cctx: *mut ZSTD_CCtx) -> usize {
    with_cctx_ref(cctx, |cctx| {
        Ok(cctx
            .stream
            .pending
            .len()
            .saturating_sub(cctx.stream.pending_pos))
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn ZSTD_getFrameProgression(cctx: *const ZSTD_CCtx) -> ZSTD_frameProgression {
    with_cctx_ref(cctx, |cctx| {
        let ingested = (cctx.stream.input.len() + cctx.legacy_input.len()) as u64;
        let produced = cctx.stream.pending.len() as u64;
        let flushed = cctx.stream.pending_pos as u64;
        Ok(ZSTD_frameProgression {
            ingested,
            consumed: ingested,
            produced,
            flushed,
            currentJobID: 0,
            nbActiveWorkers: u32::from(cctx.nb_workers > 0 || !cctx.thread_pool.is_null()),
        })
    })
    .unwrap_or_default()
}
