use crate::ffi::{
    compress::{generic_error, load_upstream, null_cctx},
    types::{
        ZSTD_CCtx, ZSTD_ResetDirective, ZSTD_cParameter, ZSTD_parameters,
    },
};
use core::ffi::{c_int, c_void};

#[no_mangle]
pub extern "C" fn ZSTD_createCCtx() -> *mut ZSTD_CCtx {
    type Fn = unsafe extern "C" fn() -> *mut ZSTD_CCtx;
    match load_upstream!("ZSTD_createCCtx", Fn) {
        Some(func) => unsafe { func() },
        None => null_cctx(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx) -> usize;
    match load_upstream!("ZSTD_freeCCtx", Fn) {
        Some(func) => unsafe { func(cctx) },
        None if cctx.is_null() => 0,
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBound(srcSize: usize) -> usize {
    type Fn = unsafe extern "C" fn(usize) -> usize;
    match load_upstream!("ZSTD_compressBound", Fn) {
        Some(func) => unsafe { func(srcSize) },
        None => generic_error(),
    }
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
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, ZSTD_ResetDirective) -> usize;
    match load_upstream!("ZSTD_CCtx_reset", Fn) {
        Some(func) => unsafe { func(cctx, reset) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParameter(
    cctx: *mut ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: c_int,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, ZSTD_cParameter, c_int) -> usize;
    match load_upstream!("ZSTD_CCtx_setParameter", Fn) {
        Some(func) => unsafe { func(cctx, param, value) },
        None => generic_error(),
    }
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
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, u64) -> usize;
    match load_upstream!("ZSTD_CCtx_setPledgedSrcSize", Fn) {
        Some(func) => unsafe { func(cctx, pledgedSrcSize) },
        None => generic_error(),
    }
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
