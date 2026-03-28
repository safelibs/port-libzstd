use crate::ffi::{
    compress::null_cctx,
    types::{
        ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CStream, ZSTD_EndDirective,
        ZSTD_compressionParameters, ZSTD_customMem, ZSTD_inBuffer, ZSTD_outBuffer,
        ZSTD_parameters,
    },
};
use core::ffi::{c_int, c_void};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_createCStream"]
    fn internal_ZSTD_createCStream() -> *mut ZSTD_CStream;
    #[link_name = "libzstd_safe_internal_ZSTD_freeCStream"]
    fn internal_ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_initCStream"]
    fn internal_ZSTD_initCStream(zcs: *mut ZSTD_CStream, compressionLevel: c_int) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_initCStream_srcSize"]
    fn internal_ZSTD_initCStream_srcSize(
        zcs: *mut ZSTD_CStream,
        compressionLevel: c_int,
        pledgedSrcSize: u64,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_initCStream_usingDict"]
    fn internal_ZSTD_initCStream_usingDict(
        zcs: *mut ZSTD_CStream,
        dict: *const c_void,
        dictSize: usize,
        compressionLevel: c_int,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_initCStream_advanced"]
    fn internal_ZSTD_initCStream_advanced(
        zcs: *mut ZSTD_CStream,
        dict: *const c_void,
        dictSize: usize,
        params: ZSTD_parameters,
        pledgedSrcSize: u64,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_resetCStream"]
    fn internal_ZSTD_resetCStream(zcs: *mut ZSTD_CStream, pledgedSrcSize: u64) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_compressStream"]
    fn internal_ZSTD_compressStream(
        zcs: *mut ZSTD_CStream,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_compressStream2"]
    fn internal_ZSTD_compressStream2(
        cctx: *mut ZSTD_CCtx,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
        endOp: ZSTD_EndDirective,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_flushStream"]
    fn internal_ZSTD_flushStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_endStream"]
    fn internal_ZSTD_endStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CStreamInSize"]
    fn internal_ZSTD_CStreamInSize() -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CStreamOutSize"]
    fn internal_ZSTD_CStreamOutSize() -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_sizeof_CStream"]
    fn internal_ZSTD_sizeof_CStream(zcs: *const ZSTD_CStream) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_estimateCStreamSize_usingCParams"]
    fn internal_ZSTD_estimateCStreamSize_usingCParams(
        cParams: ZSTD_compressionParameters,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_createCStream_advanced"]
    fn internal_ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CStream;
    #[link_name = "libzstd_safe_internal_ZSTD_compressStream2_simpleArgs"]
    fn internal_ZSTD_compressStream2_simpleArgs(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: usize,
        dstPos: *mut usize,
        src: *const c_void,
        srcSize: usize,
        srcPos: *mut usize,
        endOp: ZSTD_EndDirective,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_estimateCStreamSize"]
    fn internal_ZSTD_estimateCStreamSize(compressionLevel: c_int) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_estimateCStreamSize_usingCCtxParams"]
    fn internal_ZSTD_estimateCStreamSize_usingCCtxParams(
        params: *const ZSTD_CCtx_params,
    ) -> usize;
}

#[no_mangle]
pub extern "C" fn ZSTD_createCStream() -> *mut ZSTD_CStream {
    let stream = unsafe { internal_ZSTD_createCStream() };
    if stream.is_null() {
        null_cctx().cast()
    } else {
        stream
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> usize {
    if zcs.is_null() {
        0
    } else {
        unsafe { internal_ZSTD_freeCStream(zcs) }
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream(zcs: *mut ZSTD_CStream, compressionLevel: c_int) -> usize {
    unsafe { internal_ZSTD_initCStream(zcs, compressionLevel) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_srcSize(
    zcs: *mut ZSTD_CStream,
    compressionLevel: c_int,
    pledgedSrcSize: u64,
) -> usize {
    unsafe { internal_ZSTD_initCStream_srcSize(zcs, compressionLevel, pledgedSrcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_usingDict(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    unsafe { internal_ZSTD_initCStream_usingDict(zcs, dict, dictSize, compressionLevel) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_advanced(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: u64,
) -> usize {
    unsafe { internal_ZSTD_initCStream_advanced(zcs, dict, dictSize, params, pledgedSrcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_resetCStream(zcs: *mut ZSTD_CStream, pledgedSrcSize: u64) -> usize {
    unsafe { internal_ZSTD_resetCStream(zcs, pledgedSrcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    unsafe { internal_ZSTD_compressStream(zcs, output, input) }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream2(
    cctx: *mut ZSTD_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    endOp: ZSTD_EndDirective,
) -> usize {
    unsafe { internal_ZSTD_compressStream2(cctx, output, input, endOp) }
}

#[no_mangle]
pub extern "C" fn ZSTD_flushStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize {
    unsafe { internal_ZSTD_flushStream(zcs, output) }
}

#[no_mangle]
pub extern "C" fn ZSTD_endStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize {
    unsafe { internal_ZSTD_endStream(zcs, output) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CStreamInSize() -> usize {
    unsafe { internal_ZSTD_CStreamInSize() }
}

#[no_mangle]
pub extern "C" fn ZSTD_CStreamOutSize() -> usize {
    unsafe { internal_ZSTD_CStreamOutSize() }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_CStream(zcs: *const ZSTD_CStream) -> usize {
    unsafe { internal_ZSTD_sizeof_CStream(zcs) }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCStreamSize_usingCParams(
    cParams: ZSTD_compressionParameters,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_estimateCStreamSize_usingCParams(cParams) }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCStream_advanced(
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CStream {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    let stream = unsafe { internal_ZSTD_createCStream_advanced(customMem) };
    if stream.is_null() {
        null_cctx().cast()
    } else {
        stream
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream2_simpleArgs(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    dstPos: *mut usize,
    src: *const c_void,
    srcSize: usize,
    srcPos: *mut usize,
    endOp: ZSTD_EndDirective,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZSTD_compressStream2_simpleArgs(
            cctx, dst, dstCapacity, dstPos, src, srcSize, srcPos, endOp,
        )
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCStreamSize(compressionLevel: c_int) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_estimateCStreamSize(compressionLevel) }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCStreamSize_usingCCtxParams(
    params: *const ZSTD_CCtx_params,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_estimateCStreamSize_usingCCtxParams(params) }
}
