use crate::{
    common::error::error_result,
    ffi::{
        compress::{
            adjust_cparams, check_cparams, cparam_bounds, dparam_bounds, get_cparams, get_params,
            max_clevel, min_clevel,
        },
        types::{
            ZSTD_bounds, ZSTD_cParameter, ZSTD_compressionParameters, ZSTD_dParameter,
            ZSTD_parameters, ZSTD_CLEVEL_DEFAULT,
        },
    },
};
use core::ffi::c_int;

#[no_mangle]
pub extern "C" fn ZSTD_cParam_getBounds(cParam: ZSTD_cParameter) -> ZSTD_bounds {
    cparam_bounds(cParam)
}

#[no_mangle]
pub extern "C" fn ZSTD_dParam_getBounds(dParam: ZSTD_dParameter) -> ZSTD_bounds {
    dparam_bounds(dParam)
}

#[no_mangle]
pub extern "C" fn ZSTD_getCParams(
    compressionLevel: c_int,
    estimatedSrcSize: u64,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    get_cparams(compressionLevel, estimatedSrcSize, dictSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_getParams(
    compressionLevel: c_int,
    estimatedSrcSize: u64,
    dictSize: usize,
) -> ZSTD_parameters {
    get_params(compressionLevel, estimatedSrcSize, dictSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_checkCParams(params: ZSTD_compressionParameters) -> usize {
    match check_cparams(params) {
        Ok(()) => 0,
        Err(error) => error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_adjustCParams(
    cPar: ZSTD_compressionParameters,
    srcSize: u64,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    adjust_cparams(cPar, srcSize, dictSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_maxCLevel() -> c_int {
    max_clevel()
}

#[no_mangle]
pub extern "C" fn ZSTD_minCLevel() -> c_int {
    min_clevel()
}

#[no_mangle]
pub extern "C" fn ZSTD_defaultCLevel() -> c_int {
    ZSTD_CLEVEL_DEFAULT
}
