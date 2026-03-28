use crate::ffi::decompress::api;
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_readSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    magicVariant: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    unsafe { (api().read_skippable_frame)(dst, dstCapacity, magicVariant, src, srcSize) }
}
