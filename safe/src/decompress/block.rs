use crate::ffi::{decompress::api, types::ZSTD_DCtx};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_decompressBlock(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    unsafe { (api().decompress_block)(dctx, dst, dstCapacity, src, srcSize) }
}
