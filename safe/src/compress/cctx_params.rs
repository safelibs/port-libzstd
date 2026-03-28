use crate::ffi::{
    advanced::null_mut,
    types::{
        ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_cParameter, ZSTD_compressionParameters,
        ZSTD_frameParameters, ZSTD_parameters,
    },
};
use core::ffi::c_int;

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_CCtxParams_init"]
    fn internal_ZSTD_CCtxParams_init(
        cctxParams: *mut ZSTD_CCtx_params,
        compressionLevel: c_int,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_freeCCtxParams"]
    fn internal_ZSTD_freeCCtxParams(params: *mut ZSTD_CCtx_params) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_setFParams"]
    fn internal_ZSTD_CCtx_setFParams(
        cctx: *mut ZSTD_CCtx,
        fparams: ZSTD_frameParameters,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_createCCtxParams"]
    fn internal_ZSTD_createCCtxParams() -> *mut ZSTD_CCtx_params;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtxParams_setParameter"]
    fn internal_ZSTD_CCtxParams_setParameter(
        params: *mut ZSTD_CCtx_params,
        param: ZSTD_cParameter,
        value: c_int,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_setParametersUsingCCtxParams"]
    fn internal_ZSTD_CCtx_setParametersUsingCCtxParams(
        cctx: *mut ZSTD_CCtx,
        params: *const ZSTD_CCtx_params,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtxParams_getParameter"]
    fn internal_ZSTD_CCtxParams_getParameter(
        params: *const ZSTD_CCtx_params,
        param: ZSTD_cParameter,
        value: *mut c_int,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtxParams_init_advanced"]
    fn internal_ZSTD_CCtxParams_init_advanced(
        cctxParams: *mut ZSTD_CCtx_params,
        params: ZSTD_parameters,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtxParams_reset"]
    fn internal_ZSTD_CCtxParams_reset(params: *mut ZSTD_CCtx_params) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_setCParams"]
    fn internal_ZSTD_CCtx_setCParams(
        cctx: *mut ZSTD_CCtx,
        cparams: ZSTD_compressionParameters,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_CCtx_setParams"]
    fn internal_ZSTD_CCtx_setParams(cctx: *mut ZSTD_CCtx, params: ZSTD_parameters) -> usize;
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_init(
    cctxParams: *mut ZSTD_CCtx_params,
    compressionLevel: c_int,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtxParams_init(cctxParams, compressionLevel) }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCCtxParams(params: *mut ZSTD_CCtx_params) -> usize {
    if params.is_null() {
        return 0;
    }
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_freeCCtxParams(params) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setFParams(
    cctx: *mut ZSTD_CCtx,
    fparams: ZSTD_frameParameters,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtx_setFParams(cctx, fparams) }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCCtxParams() -> *mut ZSTD_CCtx_params {
    // SAFETY: The linked helper uses the same ABI and takes no arguments.
    let params = unsafe { internal_ZSTD_createCCtxParams() };
    if params.is_null() {
        null_mut()
    } else {
        params
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_setParameter(
    params: *mut ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    value: c_int,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtxParams_setParameter(params, param, value) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParametersUsingCCtxParams(
    cctx: *mut ZSTD_CCtx,
    params: *const ZSTD_CCtx_params,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtx_setParametersUsingCCtxParams(cctx, params) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_getParameter(
    params: *const ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtxParams_getParameter(params, param, value) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_init_advanced(
    cctxParams: *mut ZSTD_CCtx_params,
    params: ZSTD_parameters,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtxParams_init_advanced(cctxParams, params) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_reset(params: *mut ZSTD_CCtx_params) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZSTD_CCtxParams_reset(params) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setCParams(
    cctx: *mut ZSTD_CCtx,
    cparams: ZSTD_compressionParameters,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtx_setCParams(cctx, cparams) }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParams(
    cctx: *mut ZSTD_CCtx,
    params: ZSTD_parameters,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_CCtx_setParams(cctx, params) }
}
