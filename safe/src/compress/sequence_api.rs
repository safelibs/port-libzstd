use crate::{
    common::error::error_result,
    ffi::{
        compress::{
            compress_sequences_to_dst, emit_sequences, optional_src_slice, sequence_bound,
            set_sequence_producer, to_result, with_cctx_mut, with_cctx_ref,
        },
        types::{ZSTD_CCtx, ZSTD_ErrorCode, ZSTD_Sequence, ZSTD_sequenceProducer_F},
    },
};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_sequenceBound(srcSize: usize) -> usize {
    sequence_bound(srcSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_compressSequences(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstSize: usize,
    _inSeqs: *const ZSTD_Sequence,
    _inSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_ref(cctx, |cctx| {
        compress_sequences_to_dst(cctx, dst, dstSize, _inSeqs, _inSeqsSize, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_generateSequences(
    zc: *mut ZSTD_CCtx,
    outSeqs: *mut ZSTD_Sequence,
    outSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    to_result(with_cctx_ref(zc, |cctx| {
        emit_sequences(cctx, outSeqs, outSeqsSize, src, srcSize)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_registerSequenceProducer(
    cctx: *mut ZSTD_CCtx,
    sequenceProducerState: *mut c_void,
    sequenceProducer: Option<ZSTD_sequenceProducer_F>,
) {
    let _ = with_cctx_mut(cctx, |cctx| {
        set_sequence_producer(cctx, sequenceProducerState, sequenceProducer);
        Ok(0)
    });
}

#[no_mangle]
pub extern "C" fn ZSTD_mergeBlockDelimiters(
    sequences: *mut ZSTD_Sequence,
    seqsSize: usize,
) -> usize {
    if seqsSize != 0 && sequences.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
    }
    if seqsSize == 0 {
        return 0;
    }

    let sequences = unsafe { core::slice::from_raw_parts_mut(sequences, seqsSize) };
    let mut input = 0usize;
    let mut output = 0usize;

    while input < seqsSize {
        let sequence = sequences[input];
        if sequence.offset == 0 && sequence.matchLength == 0 {
            if input + 1 < seqsSize {
                sequences[input + 1].litLength = sequences[input + 1]
                    .litLength
                    .saturating_add(sequence.litLength);
            }
        } else {
            sequences[output] = sequence;
            output += 1;
        }
        input += 1;
    }

    output
}
