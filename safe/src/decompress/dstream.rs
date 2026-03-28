use crate::ffi::{
    decompress::api,
    types::{ZSTD_DStream, ZSTD_inBuffer, ZSTD_nextInputType_e, ZSTD_outBuffer},
};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DStream {
    unsafe { (api().create_dstream)() }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeDStream(zds: *mut ZSTD_DStream) -> usize {
    unsafe { (api().free_dstream)(zds) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initDStream(zds: *mut ZSTD_DStream) -> usize {
    unsafe { (api().init_dstream)(zds) }
}

#[no_mangle]
pub extern "C" fn ZSTD_resetDStream(zds: *mut ZSTD_DStream) -> usize {
    unsafe { (api().reset_dstream)(zds) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressStream(
    zds: *mut ZSTD_DStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    unsafe { (api().decompress_stream)(zds, output, input) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DStreamInSize() -> usize {
    unsafe { (api().dstream_in_size)() }
}

#[no_mangle]
pub extern "C" fn ZSTD_DStreamOutSize() -> usize {
    unsafe { (api().dstream_out_size)() }
}

#[no_mangle]
pub extern "C" fn ZSTD_nextSrcSizeToDecompress(dctx: *mut crate::ffi::types::ZSTD_DCtx) -> usize {
    unsafe { (api().next_src_size_to_decompress)(dctx) }
}

#[no_mangle]
pub extern "C" fn ZSTD_nextInputType(
    dctx: *mut crate::ffi::types::ZSTD_DCtx,
) -> ZSTD_nextInputType_e {
    unsafe { (api().next_input_type)(dctx) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decodingBufferSize_min(
    windowSize: u64,
    frameContentSize: u64,
) -> usize {
    unsafe { (api().decoding_buffer_size_min)(windowSize, frameContentSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_DStream(zds: *const ZSTD_DStream) -> usize {
    unsafe { (api().sizeof_dstream)(zds) }
}

#[allow(dead_code)]
fn _unused(_ptr: *const c_void) {}
