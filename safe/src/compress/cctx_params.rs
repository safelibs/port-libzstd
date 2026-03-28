use crate::ffi::{
    advanced::{forward_extern, null_mut},
    types::{
        ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_cParameter, ZSTD_compressionParameters,
        ZSTD_frameParameters, ZSTD_parameters,
    },
};
use core::ffi::c_int;

forward_extern! {
    pub fn ZSTD_CCtxParams_init(
        cctxParams: *mut ZSTD_CCtx_params,
        compressionLevel: c_int,
    ) -> usize => crate::ffi::compress::generic_error()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCCtxParams(params: *mut ZSTD_CCtx_params) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx_params) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_freeCCtxParams", Fn) {
        Some(func) => unsafe { func(params) },
        None if params.is_null() => 0,
        None => crate::ffi::compress::generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setFParams(
    cctx: *mut ZSTD_CCtx,
    fparams: ZSTD_frameParameters,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, ZSTD_frameParameters) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_CCtx_setFParams", Fn) {
        Some(func) => unsafe { func(cctx, fparams) },
        None => crate::ffi::compress::generic_error(),
    }
}

forward_extern! {
    pub fn ZSTD_createCCtxParams() -> *mut ZSTD_CCtx_params => null_mut()
}

forward_extern! {
    pub fn ZSTD_CCtxParams_setParameter(
        params: *mut ZSTD_CCtx_params,
        param: ZSTD_cParameter,
        value: c_int,
    ) -> usize => crate::ffi::compress::generic_error()
}

forward_extern! {
    pub fn ZSTD_CCtx_setParametersUsingCCtxParams(
        cctx: *mut ZSTD_CCtx,
        params: *const ZSTD_CCtx_params,
    ) -> usize => crate::ffi::compress::generic_error()
}

forward_extern! {
    pub fn ZSTD_CCtxParams_getParameter(
        params: *const ZSTD_CCtx_params,
        param: ZSTD_cParameter,
        value: *mut c_int,
    ) -> usize => crate::ffi::compress::generic_error()
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_init_advanced(
    cctxParams: *mut ZSTD_CCtx_params,
    params: ZSTD_parameters,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx_params, ZSTD_parameters) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_CCtxParams_init_advanced", Fn) {
        Some(func) => unsafe { func(cctxParams, params) },
        None => crate::ffi::compress::generic_error(),
    }
}

forward_extern! {
    pub fn ZSTD_CCtxParams_reset(params: *mut ZSTD_CCtx_params) -> usize => crate::ffi::compress::generic_error()
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setCParams(
    cctx: *mut ZSTD_CCtx,
    cparams: ZSTD_compressionParameters,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, ZSTD_compressionParameters) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_CCtx_setCParams", Fn) {
        Some(func) => unsafe { func(cctx, cparams) },
        None => crate::ffi::compress::generic_error(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParams(
    cctx: *mut ZSTD_CCtx,
    params: ZSTD_parameters,
) -> usize {
    type Fn = unsafe extern "C" fn(*mut ZSTD_CCtx, ZSTD_parameters) -> usize;
    match crate::ffi::compress::load_upstream!("ZSTD_CCtx_setParams", Fn) {
        Some(func) => unsafe { func(cctx, params) },
        None => crate::ffi::compress::generic_error(),
    }
}
