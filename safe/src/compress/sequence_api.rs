use crate::ffi::types::{ZSTD_CCtx, ZSTD_Sequence, ZSTD_sequenceProducer_F};
use core::ffi::c_void;

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_sequenceBound"]
    fn internal_ZSTD_sequenceBound(srcSize: usize) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_compressSequences"]
    fn internal_ZSTD_compressSequences(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstSize: usize,
        inSeqs: *const ZSTD_Sequence,
        inSeqsSize: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_generateSequences"]
    fn internal_ZSTD_generateSequences(
        zc: *mut ZSTD_CCtx,
        outSeqs: *mut ZSTD_Sequence,
        outSeqsSize: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_registerSequenceProducer"]
    fn internal_ZSTD_registerSequenceProducer(
        cctx: *mut ZSTD_CCtx,
        sequenceProducerState: *mut c_void,
        sequenceProducer: Option<ZSTD_sequenceProducer_F>,
    );
    #[link_name = "libzstd_safe_internal_ZSTD_mergeBlockDelimiters"]
    fn internal_ZSTD_mergeBlockDelimiters(
        sequences: *mut ZSTD_Sequence,
        seqsSize: usize,
    ) -> usize;
}

#[no_mangle]
pub extern "C" fn ZSTD_sequenceBound(srcSize: usize) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_sequenceBound(srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressSequences(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstSize: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_compressSequences(cctx, dst, dstSize, inSeqs, inSeqsSize, src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_generateSequences(
    zc: *mut ZSTD_CCtx,
    outSeqs: *mut ZSTD_Sequence,
    outSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_generateSequences(zc, outSeqs, outSeqsSize, src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_registerSequenceProducer(
    cctx: *mut ZSTD_CCtx,
    sequenceProducerState: *mut c_void,
    sequenceProducer: Option<ZSTD_sequenceProducer_F>,
) {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_registerSequenceProducer(cctx, sequenceProducerState, sequenceProducer) }
}

#[no_mangle]
pub extern "C" fn ZSTD_mergeBlockDelimiters(
    sequences: *mut ZSTD_Sequence,
    seqsSize: usize,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_mergeBlockDelimiters(sequences, seqsSize) }
}
