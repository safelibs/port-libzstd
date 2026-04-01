use crate::{
    common::error::error_result,
    ffi::{
        decompress::{decompress_block_body, with_dctx_mut},
        types::{ZSTD_DCtx, ZSTD_ErrorCode},
    },
};
use core::ffi::c_void;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BlockType {
    Raw,
    Rle,
    Compressed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BlockHeader {
    pub last_block: bool,
    pub block_type: BlockType,
    pub content_size: usize,
}

pub(crate) const BLOCK_HEADER_SIZE: usize = 3;
pub(crate) const BLOCK_SIZE_MAX: usize = 1 << 17;

pub(crate) fn parse_block_header(src: &[u8]) -> Result<BlockHeader, ZSTD_ErrorCode> {
    if src.len() < BLOCK_HEADER_SIZE {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    let raw = u32::from_le_bytes([src[0], src[1], src[2], 0]);
    let block_type = match (raw >> 1) & 0x3 {
        0 => BlockType::Raw,
        1 => BlockType::Rle,
        2 => BlockType::Compressed,
        _ => return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected),
    };
    let content_size = ((raw >> 3) & 0x1F_FFFF) as usize;

    if content_size > BLOCK_SIZE_MAX {
        return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected);
    }

    Ok(BlockHeader {
        last_block: (raw & 1) != 0,
        block_type,
        content_size,
    })
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBlock(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src) = crate::ffi::decompress::optional_src_slice(src, srcSize) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match with_dctx_mut(dctx, |dctx| {
        decompress_block_body(dctx, dst, dstCapacity, src)
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}
