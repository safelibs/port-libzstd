#![allow(dead_code)]

use crate::{
    common::error::error_result,
    compress::{
        block::ZSTD_compressBlock,
        cdict::ZSTD_compressBegin_usingCDict,
        match_state::{
            get_seq_store, reset_compressed_block_state, CompressedBlockState, SeqStore,
        },
        sequences::seq_to_codes,
    },
    ffi::types::{ZSTD_CCtx, ZSTD_CDict, ZSTD_ErrorCode},
};
use core::ffi::c_void;

pub(crate) type HUF_CElt = usize;

pub(crate) fn ZSTD_compressBegin_usingCDict_deprecated(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> usize {
    ZSTD_compressBegin_usingCDict(cctx, cdict)
}

pub(crate) fn ZSTD_compressBlock_deprecated(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock(cctx, dst, dstCapacity, src, srcSize)
}

pub(crate) fn ZSTD_reset_compressedBlockState(bs: *mut c_void) {
    reset_compressed_block_state(bs.cast::<CompressedBlockState>());
}

pub(crate) fn ZSTD_getSeqStore(ctx: *const ZSTD_CCtx) -> *const c_void {
    get_seq_store(ctx).cast()
}

pub(crate) fn ZSTD_seqToCodes(seqStorePtr: *const c_void) -> i32 {
    seq_to_codes(seqStorePtr.cast::<SeqStore>())
}

pub(crate) fn ZSTD_loadCEntropy(
    _bs: *mut c_void,
    _workspace: *mut c_void,
    _dict: *const c_void,
    dictSize: usize,
) -> usize {
    if dictSize < 8 {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted);
    }
    8
}

pub(crate) fn FSE_normalizeCount(
    normalizedCounter: *mut i16,
    tableLog: u32,
    count: *const u32,
    _srcSize: usize,
    maxSymbolValue: u32,
    _useLowProbCount: u32,
) -> usize {
    if normalizedCounter.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstBuffer_null);
    }

    let normalized =
        unsafe { core::slice::from_raw_parts_mut(normalizedCounter, maxSymbolValue as usize + 1) };
    normalized.fill(0);

    if !count.is_null() && !normalized.is_empty() {
        let count = unsafe { core::slice::from_raw_parts(count, maxSymbolValue as usize + 1) };
        let mut best_index = 0usize;
        let mut best_value = 0u32;
        for (index, value) in count.iter().enumerate() {
            if *value > best_value {
                best_value = *value;
                best_index = index;
            }
        }
        normalized[best_index] = (1u32 << tableLog.min(14)) as i16;
    }

    tableLog as usize
}

pub(crate) fn FSE_writeNCount(
    buffer: *mut c_void,
    bufferSize: usize,
    _normalizedCounter: *const i16,
    _maxSymbolValue: u32,
    tableLog: u32,
) -> usize {
    if buffer.is_null() || bufferSize == 0 {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    let buffer = unsafe { core::slice::from_raw_parts_mut(buffer.cast::<u8>(), bufferSize) };
    buffer[0] = tableLog as u8;
    1
}

pub(crate) fn HUF_buildCTable_wksp(
    tree: *mut HUF_CElt,
    count: *const u32,
    maxSymbolValue: u32,
    maxNbBits: u32,
    _workspace: *mut c_void,
    _wkspSize: usize,
) -> usize {
    if tree.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstBuffer_null);
    }

    let tree = unsafe { core::slice::from_raw_parts_mut(tree, maxSymbolValue as usize + 1) };
    tree.fill(0);
    if !count.is_null() {
        let count = unsafe { core::slice::from_raw_parts(count, maxSymbolValue as usize + 1) };
        if let Some((index, _)) = count.iter().enumerate().max_by_key(|(_, value)| *value) {
            tree[index] = 1;
        }
    }
    maxNbBits as usize
}

pub(crate) fn HUF_writeCTable_wksp(
    dst: *mut c_void,
    maxDstSize: usize,
    _ctable: *const HUF_CElt,
    _maxSymbolValue: u32,
    huffLog: u32,
    _workspace: *mut c_void,
    _workspaceSize: usize,
) -> usize {
    if dst.is_null() || maxDstSize == 0 {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    let dst = unsafe { core::slice::from_raw_parts_mut(dst.cast::<u8>(), maxDstSize) };
    dst[0] = huffLog as u8;
    1
}
