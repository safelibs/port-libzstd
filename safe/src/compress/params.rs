use crate::ffi::{
    compress::{bounds_error, default_cparams, default_params, load_upstream},
    types::{
        ZSTD_ErrorCode, ZSTD_bounds, ZSTD_compressionParameters, ZSTD_dParameter,
        ZSTD_parameters, ZSTD_cParameter,
    },
};
use core::ffi::c_int;

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_getCParams"]
    fn internal_ZSTD_getCParams(
        compressionLevel: c_int,
        estimatedSrcSize: u64,
        dictSize: usize,
    ) -> ZSTD_compressionParameters;
    #[link_name = "libzstd_safe_internal_ZSTD_getParams"]
    fn internal_ZSTD_getParams(
        compressionLevel: c_int,
        estimatedSrcSize: u64,
        dictSize: usize,
    ) -> ZSTD_parameters;
    #[link_name = "libzstd_safe_internal_ZSTD_checkCParams"]
    fn internal_ZSTD_checkCParams(params: ZSTD_compressionParameters) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_adjustCParams"]
    fn internal_ZSTD_adjustCParams(
        cPar: ZSTD_compressionParameters,
        srcSize: u64,
        dictSize: usize,
    ) -> ZSTD_compressionParameters;
    #[link_name = "libzstd_safe_internal_ZSTD_maxCLevel"]
    fn internal_ZSTD_maxCLevel() -> c_int;
    #[link_name = "libzstd_safe_internal_ZSTD_minCLevel"]
    fn internal_ZSTD_minCLevel() -> c_int;
    #[link_name = "libzstd_safe_internal_ZSTD_defaultCLevel"]
    fn internal_ZSTD_defaultCLevel() -> c_int;
}

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
    // SAFETY: The linked helper uses the same ABI and returns the upstream result directly.
    let params = unsafe { internal_ZSTD_getCParams(compressionLevel, estimatedSrcSize, dictSize) };
    if params.windowLog == 0 {
        default_cparams()
    } else {
        params
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_getParams(
    compressionLevel: c_int,
    estimatedSrcSize: u64,
    dictSize: usize,
) -> ZSTD_parameters {
    // SAFETY: The linked helper uses the same ABI and returns the upstream result directly.
    let params = unsafe { internal_ZSTD_getParams(compressionLevel, estimatedSrcSize, dictSize) };
    if params.cParams.windowLog == 0 {
        default_params()
    } else {
        params
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_checkCParams(params: ZSTD_compressionParameters) -> usize {
    // SAFETY: The linked helper uses the same ABI and validates the parameters directly.
    unsafe { internal_ZSTD_checkCParams(params) }
}

#[no_mangle]
pub extern "C" fn ZSTD_adjustCParams(
    cPar: ZSTD_compressionParameters,
    srcSize: u64,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    // SAFETY: The linked helper uses the same ABI and returns the adjusted parameters directly.
    unsafe { internal_ZSTD_adjustCParams(cPar, srcSize, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_maxCLevel() -> c_int {
    // SAFETY: The linked helper uses the same ABI and returns the upstream maximum directly.
    unsafe { internal_ZSTD_maxCLevel() }
}

#[no_mangle]
pub extern "C" fn ZSTD_minCLevel() -> c_int {
    // SAFETY: The linked helper uses the same ABI and returns the upstream minimum directly.
    let level = unsafe { internal_ZSTD_minCLevel() };
    if level == 0 {
        0i32.wrapping_sub(ZSTD_ErrorCode::ZSTD_error_GENERIC as c_int)
    } else {
        level
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_defaultCLevel() -> c_int {
    // SAFETY: The linked helper uses the same ABI and returns the upstream default directly.
    let level = unsafe { internal_ZSTD_defaultCLevel() };
    if level == 0 {
        crate::ffi::types::ZSTD_CLEVEL_DEFAULT
    } else {
        level
    }
}
