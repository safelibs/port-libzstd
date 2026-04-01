use crate::{
    common::error::error_result,
    ffi::{
        compress::{encode_block_body, to_result, with_cctx_mut, with_cctx_ref},
        decompress::{
            insert_uncompressed_block, optional_src_slice as optional_block_slice, with_dctx_mut,
        },
        types::{ZSTD_CCtx, ZSTD_DCtx, ZSTD_BLOCKSIZE_MAX},
    },
};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_getBlockSize(cctx: *const ZSTD_CCtx) -> usize {
    with_cctx_ref(cctx, |cctx| Ok(cctx.frame_block_size())).unwrap_or(ZSTD_BLOCKSIZE_MAX)
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBlock(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src) = crate::ffi::compress::optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_mut(cctx, |cctx| {
        if src.len() > cctx.frame_block_size() {
            return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
        }
        encode_block_body(cctx, dst, dstCapacity, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_insertBlock(
    dctx: *mut ZSTD_DCtx,
    blockStart: *const c_void,
    blockSize: usize,
) -> usize {
    let Some(block) = optional_block_slice(blockStart, blockSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match with_dctx_mut(dctx, |dctx| insert_uncompressed_block(dctx, block)) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}
