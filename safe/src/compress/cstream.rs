use crate::ffi::{
    compress::{generic_error, load_upstream, null_cctx},
    types::{
        ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CStream, ZSTD_EndDirective,
        ZSTD_compressionParameters, ZSTD_customMem, ZSTD_inBuffer, ZSTD_outBuffer,
        ZSTD_parameters,
    },
};
use core::ffi::{c_int, c_void};

unsafe extern "C" {
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
    type Fn = unsafe extern "C" fn() -> *mut ZSTD_CStream;
    match load_upstream!("ZSTD_createCStream", Fn) {
        Some(func) => unsafe { func() },
        None => null_cctx().cast(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream) -> usize;
    match load_upstream!("ZSTD_freeCStream", Fn) {
        Some(func) => unsafe { func(zcs) },
        None if zcs.is_null() => 0,
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream(zcs: *mut ZSTD_CStream, compressionLevel: c_int) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, c_int) -> usize;
    match load_upstream!("ZSTD_initCStream", Fn) {
        Some(func) => unsafe { func(zcs, compressionLevel) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_srcSize(
    zcs: *mut ZSTD_CStream,
    compressionLevel: c_int,
    pledgedSrcSize: u64,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, c_int, u64) -> usize;
    match load_upstream!("ZSTD_initCStream_srcSize", Fn) {
        Some(func) => unsafe { func(zcs, compressionLevel, pledgedSrcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_usingDict(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, *const c_void, usize, c_int) -> usize;
    match load_upstream!("ZSTD_initCStream_usingDict", Fn) {
        Some(func) => unsafe { func(zcs, dict, dictSize, compressionLevel) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_advanced(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: u64,
) -> usize {
    type Fn =
        unsafe extern "C" fn(*mut ZSTD_CStream, *const c_void, usize, ZSTD_parameters, u64) -> usize;
    match load_upstream!("ZSTD_initCStream_advanced", Fn) {
        Some(func) => unsafe { func(zcs, dict, dictSize, params, pledgedSrcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_resetCStream(zcs: *mut ZSTD_CStream, pledgedSrcSize: u64) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, u64) -> usize;
    match load_upstream!("ZSTD_resetCStream", Fn) {
        Some(func) => unsafe { func(zcs, pledgedSrcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;
    match load_upstream!("ZSTD_compressStream", Fn) {
        Some(func) => unsafe { func(zcs, output, input) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream2(
    cctx: *mut ZSTD_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    endOp: ZSTD_EndDirective,
) -> usize {
    type Fn =
        unsafe extern "C" fn(*mut ZSTD_CCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, ZSTD_EndDirective) -> usize;
    match load_upstream!("ZSTD_compressStream2", Fn) {
        Some(func) => unsafe { func(cctx, output, input, endOp) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_flushStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, *mut ZSTD_outBuffer) -> usize;
    match load_upstream!("ZSTD_flushStream", Fn) {
        Some(func) => unsafe { func(zcs, output) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_endStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, *mut ZSTD_outBuffer) -> usize;
    match load_upstream!("ZSTD_endStream", Fn) {
        Some(func) => unsafe { func(zcs, output) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CStreamInSize() -> usize {
    type Fn = unsafe extern "C" fn() -> usize;
    match load_upstream!("ZSTD_CStreamInSize", Fn) {
        Some(func) => unsafe { func() },
        None => crate::ffi::types::ZSTD_BLOCKSIZE_MAX,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CStreamOutSize() -> usize {
    type Fn = unsafe extern "C" fn() -> usize;
    match load_upstream!("ZSTD_CStreamOutSize", Fn) {
        Some(func) => unsafe { func() },
        None => crate::ffi::types::ZSTD_BLOCKSIZE_MAX + 1024,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_CStream(zcs: *const ZSTD_CStream) -> usize {
    type Fn = unsafe extern "C" fn(*const ZSTD_CStream) -> usize;
    match load_upstream!("ZSTD_sizeof_CStream", Fn) {
        Some(func) => unsafe { func(zcs) },
        None => 0,
    }
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
