#![allow(dead_code)]

use crate::{
    common::error::error_result,
    compress::match_state::RawSequence,
    ffi::types::{ZSTD_CCtx, ZSTD_ErrorCode},
};
use core::ffi::c_void;

pub(crate) fn write_last_empty_block(dst: *mut c_void, dst_capacity: usize) -> usize {
    if dst.is_null() || dst_capacity < 3 {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    let dst = unsafe { core::slice::from_raw_parts_mut(dst.cast::<u8>(), dst_capacity) };
    dst[..3].copy_from_slice(&[1, 0, 0]);
    3
}

pub(crate) fn reference_external_sequences(
    cctx: *mut ZSTD_CCtx,
    sequences: *mut RawSequence,
    count: usize,
) -> usize {
    if count != 0 && (cctx.is_null() || sequences.is_null()) {
        return error_result(ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid);
    }
    count
}
