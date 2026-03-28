use crate::ffi::{
    compress::{generic_error, load_upstream},
    types::ZSTD_CCtx,
};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_getBlockSize(cctx: *const ZSTD_CCtx) -> usize {
    type Fn = unsafe extern "C" fn(*const ZSTD_CCtx) -> usize;
    match load_upstream!("ZSTD_getBlockSize", Fn) {
        Some(func) => unsafe { func(cctx) },
        None => crate::ffi::types::ZSTD_BLOCKSIZE_MAX,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBlock(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *mut c_void, usize, *const c_void, usize) -> usize;
    match load_upstream!("ZSTD_compressBlock", Fn) {
        Some(func) => unsafe { func(cctx, dst, dstCapacity, src, srcSize) },
        None => generic_error(),
    }
}
