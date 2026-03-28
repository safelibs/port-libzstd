use crate::{
    common::error::error_result,
    decompress::{
        block::{BLOCK_HEADER_SIZE, BLOCK_SIZE_MAX},
        frame,
    },
    ffi::{
        decompress,
        types::{
            ZSTD_DCtx, ZSTD_DStream, ZSTD_customMem, ZSTD_inBuffer,
            ZSTD_nextInputType_e, ZSTD_outBuffer,
        },
    },
};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DStream {
    decompress::create_dctx().cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeDStream(zds: *mut ZSTD_DStream) -> usize {
    decompress::free_dctx(zds.cast())
}

#[no_mangle]
pub extern "C" fn ZSTD_initDStream(zds: *mut ZSTD_DStream) -> usize {
    match decompress::with_dctx_mut(zds, |zds| {
        zds.reset_session();
        zds.ref_ddict(core::ptr::null())?;
        Ok(frame::starting_input_length(zds.format))
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_resetDStream(zds: *mut ZSTD_DStream) -> usize {
    match decompress::with_dctx_mut(zds, |zds| {
        zds.reset_session();
        Ok(frame::starting_input_length(zds.format))
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressStream(
    zds: *mut ZSTD_DStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    if output.is_null() || input.is_null() {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    // SAFETY: The caller provided valid `ZSTD_inBuffer`/`ZSTD_outBuffer` pointers.
    let (output, input) = unsafe { (&mut *output, &mut *input) };
    match decompress::with_dctx_mut(zds, |zds| decompress::stream_decompress(zds, output, input)) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DStreamInSize() -> usize {
    BLOCK_SIZE_MAX + BLOCK_HEADER_SIZE
}

#[no_mangle]
pub extern "C" fn ZSTD_DStreamOutSize() -> usize {
    BLOCK_SIZE_MAX
}

#[no_mangle]
pub extern "C" fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> usize {
    decompress::next_src_size_to_decompress(dctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_nextInputType(dctx: *mut ZSTD_DCtx) -> ZSTD_nextInputType_e {
    decompress::next_input_type(dctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_decodingBufferSize_min(windowSize: u64, frameContentSize: u64) -> usize {
    match decompress::decoding_buffer_size_min(windowSize, frameContentSize) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_DStream(zds: *const ZSTD_DStream) -> usize {
    decompress::sizeof_dctx(zds.cast())
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDStreamSize_fromFrame(
    src: *const c_void,
    srcSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*const c_void, usize) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_estimateDStreamSize_fromFrame", Fn) {
        Some(func) => unsafe { func(src, srcSize) },
        None => error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_GENERIC),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDStreamSize(windowSize: usize) -> usize {
    type Fn = unsafe extern "C" fn(usize) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_estimateDStreamSize", Fn) {
        Some(func) => unsafe { func(windowSize) },
        None => error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_GENERIC),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressStream_simpleArgs(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    dstPos: *mut usize,
    src: *const c_void,
    srcSize: usize,
    srcPos: *mut usize,
) -> usize {
    type Fn = unsafe extern "C" fn(
        *mut ZSTD_DCtx,
        *mut c_void,
        usize,
        *mut usize,
        *const c_void,
        usize,
        *mut usize,
    ) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_decompressStream_simpleArgs", Fn) {
        Some(func) => unsafe { func(dctx, dst, dstCapacity, dstPos, src, srcSize, srcPos) },
        None => error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_GENERIC),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createDStream_advanced(
    _customMem: ZSTD_customMem,
) -> *mut ZSTD_DStream {
    decompress::create_dctx().cast()
}
