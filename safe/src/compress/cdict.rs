use crate::ffi::{
    compress::{generic_error, load_upstream, null_cdict},
    types::{
        ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CDict, ZSTD_CStream,
        ZSTD_compressionParameters, ZSTD_customMem, ZSTD_dictContentType_e,
        ZSTD_dictLoadMethod_e, ZSTD_frameParameters,
    },
};
use core::ffi::{c_int, c_void};

#[no_mangle]
pub extern "C" fn ZSTD_createCDict(
    dictBuffer: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> *mut ZSTD_CDict {
    type Fn = unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut ZSTD_CDict;
    match load_upstream!("ZSTD_createCDict", Fn) {
        Some(func) => unsafe { func(dictBuffer, dictSize, compressionLevel) },
        None => null_cdict(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CDict) -> usize;
    match load_upstream!("ZSTD_freeCDict", Fn) {
        Some(func) => unsafe { func(cdict) },
        None if cdict.is_null() => 0,
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromCDict(cdict: *const ZSTD_CDict) -> u32 {
    type Fn = unsafe extern "C" fn(*const ZSTD_CDict) -> u32;
    match load_upstream!("ZSTD_getDictID_fromCDict", Fn) {
        Some(func) => unsafe { func(cdict) },
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_loadDictionary(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *const c_void, usize) -> usize;
    match load_upstream!("ZSTD_CCtx_loadDictionary", Fn) {
        Some(func) => unsafe { func(cctx, dict, dictSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refCDict(cctx: *mut ZSTD_CCtx, cdict: *const ZSTD_CDict) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *const ZSTD_CDict) -> usize;
    match load_upstream!("ZSTD_CCtx_refCDict", Fn) {
        Some(func) => unsafe { func(cctx, cdict) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compress_usingCDict(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
) -> usize {
    type Fn =
        unsafe extern "C" fn(*mut ZSTD_CCtx, *mut c_void, usize, *const c_void, usize, *const ZSTD_CDict) -> usize;
    match load_upstream!("ZSTD_compress_usingCDict", Fn) {
        Some(func) => unsafe { func(cctx, dst, dstCapacity, src, srcSize, cdict) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_usingCDict(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CStream, *const ZSTD_CDict) -> usize;
    match load_upstream!("ZSTD_initCStream_usingCDict", Fn) {
        Some(func) => unsafe { func(zcs, cdict) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_usingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *const ZSTD_CDict) -> usize;
    match load_upstream!("ZSTD_compressBegin_usingCDict", Fn) {
        Some(func) => unsafe { func(cctx, cdict) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_usingCDict_advanced(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    type Fn =
        unsafe extern "C" fn(*mut ZSTD_CCtx, *const ZSTD_CDict, ZSTD_frameParameters, u64) -> usize;
    match load_upstream!("ZSTD_compressBegin_usingCDict_advanced", Fn) {
        Some(func) => unsafe { func(cctx, cdict, fParams, pledgedSrcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_usingCDict_advanced(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    type Fn =
        unsafe extern "C" fn(*mut ZSTD_CStream, *const ZSTD_CDict, ZSTD_frameParameters, u64) -> usize;
    match load_upstream!("ZSTD_initCStream_usingCDict_advanced", Fn) {
        Some(func) => unsafe { func(zcs, cdict, fParams, pledgedSrcSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> usize {
    type Fn = unsafe extern "C" fn(*const ZSTD_CDict) -> usize;
    match load_upstream!("ZSTD_sizeof_CDict", Fn) {
        Some(func) => unsafe { func(cdict) },
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compress_usingCDict_advanced(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
) -> usize {
    type Fn = unsafe extern "C" fn(
        *mut ZSTD_CCtx,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const ZSTD_CDict,
        ZSTD_frameParameters,
    ) -> usize;
    match load_upstream!("ZSTD_compress_usingCDict_advanced", Fn) {
        Some(func) => unsafe { func(cctx, dst, dstCapacity, src, srcSize, cdict, fParams) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCDict_advanced(
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    type Fn = unsafe extern "C" fn(
        *const c_void,
        usize,
        ZSTD_dictLoadMethod_e,
        ZSTD_dictContentType_e,
        ZSTD_compressionParameters,
        ZSTD_customMem,
    ) -> *mut ZSTD_CDict;
    match load_upstream!("ZSTD_createCDict_advanced", Fn) {
        Some(func) => unsafe {
            func(dict, dictSize, dictLoadMethod, dictContentType, cParams, customMem)
        },
        None => null_cdict(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_loadDictionary_byReference(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, *const c_void, usize) -> usize;
    match load_upstream!("ZSTD_CCtx_loadDictionary_byReference", Fn) {
        Some(func) => unsafe { func(cctx, dict, dictSize) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCDictSize_advanced(
    dictSize: usize,
    cParams: ZSTD_compressionParameters,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    type Fn =
        unsafe extern "C" fn(usize, ZSTD_compressionParameters, ZSTD_dictLoadMethod_e) -> usize;
    match load_upstream!("ZSTD_estimateCDictSize_advanced", Fn) {
        Some(func) => unsafe { func(dictSize, cParams, dictLoadMethod) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCDict_byReference(
    dictBuffer: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> *mut ZSTD_CDict {
    type Fn = unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut ZSTD_CDict;
    match load_upstream!("ZSTD_createCDict_byReference", Fn) {
        Some(func) => unsafe { func(dictBuffer, dictSize, compressionLevel) },
        None => null_cdict(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCDictSize(dictSize: usize, compressionLevel: c_int) -> usize {
    type Fn = unsafe extern "C" fn(usize, c_int) -> usize;
    match load_upstream!("ZSTD_estimateCDictSize", Fn) {
        Some(func) => unsafe { func(dictSize, compressionLevel) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_loadDictionary_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    type Fn = unsafe extern "C" fn(
        *mut ZSTD_CCtx,
        *const c_void,
        usize,
        ZSTD_dictLoadMethod_e,
        ZSTD_dictContentType_e,
    ) -> usize;
    match load_upstream!("ZSTD_CCtx_loadDictionary_advanced", Fn) {
        Some(func) => unsafe { func(cctx, dict, dictSize, dictLoadMethod, dictContentType) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCDict_advanced2(
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cctxParams: *const ZSTD_CCtx_params,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    type Fn = unsafe extern "C" fn(
        *const c_void,
        usize,
        ZSTD_dictLoadMethod_e,
        ZSTD_dictContentType_e,
        *const ZSTD_CCtx_params,
        ZSTD_customMem,
    ) -> *mut ZSTD_CDict;
    match load_upstream!("ZSTD_createCDict_advanced2", Fn) {
        Some(func) => unsafe {
            func(
                dict,
                dictSize,
                dictLoadMethod,
                dictContentType,
                cctxParams,
                customMem,
            )
        },
        None => null_cdict(),
    }
}
