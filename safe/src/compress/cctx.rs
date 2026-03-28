use crate::ffi::{
    compress::{generic_error, load_upstream, null_cctx},
    types::{
        ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_ResetDirective, ZSTD_cParameter,
        ZSTD_compressionParameters, ZSTD_customMem, ZSTD_dictContentType_e,
        ZSTD_parameters,
    },
};
use core::ffi::{c_int, c_void};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_createCCtx"]
    fn internal_ZSTD_createCCtx() -> *mut ZSTD_CCtx;
    #[link_name = "libzstd_safe_internal_ZSTD_freeCCtx"]
    fn internal_ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_compressBound"]
    fn internal_ZSTD_compressBound(srcSize: usize) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_refPrefix_advanced"]
    fn internal_ZSTD_CCtx_refPrefix_advanced(
        cctx: *mut ZSTD_CCtx,
        prefix: *const c_void,
        prefixSize: usize,
        dictContentType: ZSTD_dictContentType_e,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_estimateCCtxSize"]
    fn internal_ZSTD_estimateCCtxSize(compressionLevel: c_int) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_estimateCCtxSize_usingCCtxParams"]
    fn internal_ZSTD_estimateCCtxSize_usingCCtxParams(params: *const ZSTD_CCtx_params) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_compress_advanced"]
    fn internal_ZSTD_compress_advanced(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        dict: *const c_void,
        dictSize: usize,
        params: ZSTD_parameters,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_estimateCCtxSize_usingCParams"]
    fn internal_ZSTD_estimateCCtxSize_usingCParams(
        cParams: ZSTD_compressionParameters,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_createCCtx_advanced"]
    fn internal_ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_reset"]
    fn internal_ZSTD_CCtx_reset(cctx: *mut ZSTD_CCtx, reset: ZSTD_ResetDirective) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_setParameter"]
    fn internal_ZSTD_CCtx_setParameter(
        cctx: *mut ZSTD_CCtx,
        param: ZSTD_cParameter,
        value: c_int,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_setPledgedSrcSize"]
    fn internal_ZSTD_CCtx_setPledgedSrcSize(
        cctx: *mut ZSTD_CCtx,
        pledgedSrcSize: u64,
    ) -> usize;
}

#[no_mangle]
pub extern "C" fn ZSTD_createCCtx() -> *mut ZSTD_CCtx {
    let cctx = unsafe { internal_ZSTD_createCCtx() };
    if cctx.is_null() {
        null_cctx()
    } else {
        cctx
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> usize {
    if cctx.is_null() {
        0
    } else {
        unsafe { internal_ZSTD_freeCCtx(cctx) }
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBound(srcSize: usize) -> usize {
    unsafe { internal_ZSTD_compressBound(srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_compress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    compressionLevel: c_int,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int) -> usize;
    match load_upstream!("ZSTD_compress", Fn) {
        Some(func) => unsafe { func(dst, dstCapacity, src, srcSize, compressionLevel) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressCCtx(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    compressionLevel: c_int,
) -> usize {
    type Fn =
        unsafe extern "C" fn(*mut ZSTD_CCtx, *mut c_void, usize, *const c_void, usize, c_int) -> usize;
    match load_upstream!("ZSTD_compressCCtx", Fn) {
        Some(func) => unsafe { func(cctx, dst, dstCapacity, src, srcSize, compressionLevel) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compress2(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *mut c_void, usize, *const c_void, usize) -> usize;
    match load_upstream!("ZSTD_compress2", Fn) {
        Some(func) => unsafe { func(cctx, dst, dstCapacity, src, srcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_copyCCtx(
    cctx: *mut ZSTD_CCtx,
    preparedCCtx: *const ZSTD_CCtx,
    pledgedSrcSize: u64,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *const ZSTD_CCtx, u64) -> usize;
    match load_upstream!("ZSTD_copyCCtx", Fn) {
        Some(func) => unsafe { func(cctx, preparedCCtx, pledgedSrcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_reset(cctx: *mut ZSTD_CCtx, reset: ZSTD_ResetDirective) -> usize {
    unsafe { internal_ZSTD_CCtx_reset(cctx, reset) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParameter(
    cctx: *mut ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: c_int,
) -> usize {
    unsafe { internal_ZSTD_CCtx_setParameter(cctx, param, value) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_getParameter(
    cctx: *mut ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, ZSTD_cParameter, *mut c_int) -> usize;
    match load_upstream!("ZSTD_CCtx_getParameter", Fn) {
        Some(func) => unsafe { func(cctx, param, value) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setPledgedSrcSize(cctx: *mut ZSTD_CCtx, pledgedSrcSize: u64) -> usize {
    unsafe { internal_ZSTD_CCtx_setPledgedSrcSize(cctx, pledgedSrcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_compress_usingDict(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    type Fn = unsafe extern "C" fn(
        *mut ZSTD_CCtx,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        usize,
        c_int,
    ) -> usize;
    match load_upstream!("ZSTD_compress_usingDict", Fn) {
        Some(func) => unsafe {
            func(
                cctx,
                dst,
                dstCapacity,
                src,
                srcSize,
                dict,
                dictSize,
                compressionLevel,
            )
        },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refPrefix(
    cctx: *mut ZSTD_CCtx,
    prefix: *const c_void,
    prefixSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *const c_void, usize) -> usize;
    match load_upstream!("ZSTD_CCtx_refPrefix", Fn) {
        Some(func) => unsafe { func(cctx, prefix, prefixSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin(cctx: *mut ZSTD_CCtx, compressionLevel: c_int) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, c_int) -> usize;
    match load_upstream!("ZSTD_compressBegin", Fn) {
        Some(func) => unsafe { func(cctx, compressionLevel) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_usingDict(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *const c_void, usize, c_int) -> usize;
    match load_upstream!("ZSTD_compressBegin_usingDict", Fn) {
        Some(func) => unsafe { func(cctx, dict, dictSize, compressionLevel) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: u64,
) -> usize {
    type Fn =
        unsafe extern "C" fn(*mut ZSTD_CCtx, *const c_void, usize, ZSTD_parameters, u64) -> usize;
    match load_upstream!("ZSTD_compressBegin_advanced", Fn) {
        Some(func) => unsafe { func(cctx, dict, dictSize, params, pledgedSrcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressContinue(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *mut c_void, usize, *const c_void, usize) -> usize;
    match load_upstream!("ZSTD_compressContinue", Fn) {
        Some(func) => unsafe { func(cctx, dst, dstCapacity, src, srcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressEnd(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *mut c_void, usize, *const c_void, usize) -> usize;
    match load_upstream!("ZSTD_compressEnd", Fn) {
        Some(func) => unsafe { func(cctx, dst, dstCapacity, src, srcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_CCtx(cctx: *const ZSTD_CCtx) -> usize {
    type Fn = unsafe extern "C" fn(*const ZSTD_CCtx) -> usize;
    match load_upstream!("ZSTD_sizeof_CCtx", Fn) {
        Some(func) => unsafe { func(cctx) },
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refPrefix_advanced(
    cctx: *mut ZSTD_CCtx,
    prefix: *const c_void,
    prefixSize: usize,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtx_refPrefix_advanced(cctx, prefix, prefixSize, dictContentType) }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCCtxSize(compressionLevel: c_int) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_estimateCCtxSize(compressionLevel) }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCCtxSize_usingCCtxParams(
    params: *const ZSTD_CCtx_params,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_estimateCCtxSize_usingCCtxParams(params) }
}

#[no_mangle]
pub extern "C" fn ZSTD_compress_advanced(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZSTD_compress_advanced(
            cctx,
            dst,
            dstCapacity,
            src,
            srcSize,
            dict,
            dictSize,
            params,
        )
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCCtxSize_usingCParams(
    cParams: ZSTD_compressionParameters,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_estimateCCtxSize_usingCParams(cParams) }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    let cctx = unsafe { internal_ZSTD_createCCtx_advanced(customMem) };
    if cctx.is_null() {
        null_cctx()
    } else {
        cctx
    }
}
