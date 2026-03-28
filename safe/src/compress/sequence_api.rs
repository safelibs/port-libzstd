use crate::ffi::{
    advanced::{forward_extern, forward_extern_void},
    types::{ZSTD_CCtx, ZSTD_Sequence, ZSTD_sequenceProducer_F},
};
use core::ffi::c_void;

forward_extern! {
    pub fn ZSTD_sequenceBound(srcSize: usize) -> usize => crate::ffi::compress::generic_error()
}

forward_extern! {
    pub fn ZSTD_compressSequences(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstSize: usize,
        inSeqs: *const ZSTD_Sequence,
        inSeqsSize: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize => crate::ffi::compress::generic_error()
}

forward_extern! {
    pub fn ZSTD_generateSequences(
        zc: *mut ZSTD_CCtx,
        outSeqs: *mut ZSTD_Sequence,
        outSeqsSize: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize => crate::ffi::compress::generic_error()
}

forward_extern_void! {
    pub fn ZSTD_registerSequenceProducer(
        cctx: *mut ZSTD_CCtx,
        sequenceProducerState: *mut c_void,
        sequenceProducer: Option<ZSTD_sequenceProducer_F>,
    )
}

forward_extern! {
    pub fn ZSTD_mergeBlockDelimiters(
        sequences: *mut ZSTD_Sequence,
        seqsSize: usize,
    ) -> usize => crate::ffi::compress::generic_error()
}
