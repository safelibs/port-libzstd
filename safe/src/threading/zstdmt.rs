use crate::ffi::{
    advanced::forward_extern,
    types::{ZSTD_CCtx, ZSTD_frameProgression},
};

forward_extern! {
    pub fn ZSTD_toFlushNow(cctx: *mut ZSTD_CCtx) -> usize => 0
}

forward_extern! {
    pub fn ZSTD_getFrameProgression(
        cctx: *const ZSTD_CCtx,
    ) -> ZSTD_frameProgression => ZSTD_frameProgression::default()
}
