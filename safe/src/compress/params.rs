use crate::ffi::{
    compress::{bounds_error, default_cparams, default_params, generic_error, load_upstream},
    types::{
        ZSTD_ErrorCode, ZSTD_bounds, ZSTD_compressionParameters, ZSTD_dParameter,
        ZSTD_parameters, ZSTD_cParameter,
    },
};
use core::ffi::c_int;

#[no_mangle]
pub extern "C" fn ZSTD_cParam_getBounds(cParam: ZSTD_cParameter) -> ZSTD_bounds {
    type Fn = unsafe extern "C" fn(ZSTD_cParameter) -> ZSTD_bounds;
    match load_upstream!("ZSTD_cParam_getBounds", Fn) {
        Some(func) => unsafe { func(cParam) },
        None => bounds_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_dParam_getBounds(dParam: ZSTD_dParameter) -> ZSTD_bounds {
    type Fn = unsafe extern "C" fn(ZSTD_dParameter) -> ZSTD_bounds;
    match load_upstream!("ZSTD_dParam_getBounds", Fn) {
        Some(func) => unsafe { func(dParam) },
        None => bounds_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_getCParams(
    compressionLevel: c_int,
    estimatedSrcSize: u64,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    type Fn = unsafe extern "C" fn(c_int, u64, usize) -> ZSTD_compressionParameters;
    match load_upstream!("ZSTD_getCParams", Fn) {
        Some(func) => unsafe { func(compressionLevel, estimatedSrcSize, dictSize) },
        None => default_cparams(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_getParams(
    compressionLevel: c_int,
    estimatedSrcSize: u64,
    dictSize: usize,
) -> ZSTD_parameters {
    type Fn = unsafe extern "C" fn(c_int, u64, usize) -> ZSTD_parameters;
    match load_upstream!("ZSTD_getParams", Fn) {
        Some(func) => unsafe { func(compressionLevel, estimatedSrcSize, dictSize) },
        None => default_params(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_checkCParams(params: ZSTD_compressionParameters) -> usize {
    type Fn = unsafe extern "C" fn(ZSTD_compressionParameters) -> usize;
    match load_upstream!("ZSTD_checkCParams", Fn) {
        Some(func) => unsafe { func(params) },
        None => generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_adjustCParams(
    cPar: ZSTD_compressionParameters,
    srcSize: u64,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    type Fn = unsafe extern "C" fn(ZSTD_compressionParameters, u64, usize) -> ZSTD_compressionParameters;
    match load_upstream!("ZSTD_adjustCParams", Fn) {
        Some(func) => unsafe { func(cPar, srcSize, dictSize) },
        None => cPar,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_maxCLevel() -> c_int {
    type Fn = unsafe extern "C" fn() -> c_int;
    match load_upstream!("ZSTD_maxCLevel", Fn) {
        Some(func) => unsafe { func() },
        None => 22,
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_minCLevel() -> c_int {
    type Fn = unsafe extern "C" fn() -> c_int;
    match load_upstream!("ZSTD_minCLevel", Fn) {
        Some(func) => unsafe { func() },
        None => 0i32.wrapping_sub(ZSTD_ErrorCode::ZSTD_error_GENERIC as c_int),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_defaultCLevel() -> c_int {
    type Fn = unsafe extern "C" fn() -> c_int;
    match load_upstream!("ZSTD_defaultCLevel", Fn) {
        Some(func) => unsafe { func() },
        None => crate::ffi::types::ZSTD_CLEVEL_DEFAULT,
    }
}
