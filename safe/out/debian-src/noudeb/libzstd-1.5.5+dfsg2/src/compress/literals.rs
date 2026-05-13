#![allow(dead_code)]
#![allow(non_camel_case_types)]

use crate::{
    common::error::error_result,
    ffi::types::{ZSTD_ErrorCode, ZSTD_strategy},
};
use core::ffi::c_void;

#[repr(C)]
pub(crate) struct ZSTD_hufCTables_t {
    _private: [u8; 0],
}

fn copy_literals(
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    if src_size == 0 {
        return 0;
    }
    if dst.is_null() || src.is_null() || src_size > dst_capacity {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    let dst = unsafe { core::slice::from_raw_parts_mut(dst.cast::<u8>(), dst_capacity) };
    let src = unsafe { core::slice::from_raw_parts(src.cast::<u8>(), src_size) };
    dst[..src_size].copy_from_slice(src);
    src_size
}

pub(crate) fn no_compress_literals(
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    copy_literals(dst, dst_capacity, src, src_size)
}

pub(crate) fn compress_rle_literals_block(
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    if src_size == 0 {
        return 0;
    }
    if dst.is_null() || src.is_null() || dst_capacity == 0 {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }

    let dst = unsafe { core::slice::from_raw_parts_mut(dst.cast::<u8>(), dst_capacity) };
    let src = unsafe { core::slice::from_raw_parts(src.cast::<u8>(), src_size) };
    dst[0] = src[0];
    1
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compress_literals(
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
    _entropy_workspace: *mut c_void,
    _entropy_workspace_size: usize,
    _prev_huf: *const ZSTD_hufCTables_t,
    _next_huf: *mut ZSTD_hufCTables_t,
    _strategy: ZSTD_strategy,
    disable_literal_compression: bool,
    _suspect_uncompressible: bool,
    _bmi2: bool,
) -> usize {
    if disable_literal_compression {
        return no_compress_literals(dst, dst_capacity, src, src_size);
    }

    if src_size > 1 && !src.is_null() {
        let src = unsafe { core::slice::from_raw_parts(src.cast::<u8>(), src_size) };
        if src.iter().all(|byte| *byte == src[0]) {
            return compress_rle_literals_block(dst, dst_capacity, src.as_ptr().cast(), src_size);
        }
    }

    no_compress_literals(dst, dst_capacity, src, src_size)
}
