use crate::{
    common::error::error_result,
    ffi::{
        compress::{
            default_params, get_cparams, get_parameter, set_parameter, to_result, with_cctx_mut,
            EncoderContext,
        },
        types::{
            ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_ErrorCode, ZSTD_cParameter,
            ZSTD_compressionParameters, ZSTD_frameParameters, ZSTD_parameters,
        },
    },
};
use core::ffi::c_int;

#[derive(Clone, Debug)]
struct CCtxParamsState {
    compression_level: c_int,
    params: ZSTD_parameters,
    nb_workers: c_int,
    job_size: c_int,
    overlap_log: c_int,
    block_delimiters: c_int,
    enable_long_distance_matching: bool,
    validate_sequences: bool,
    enable_seq_producer_fallback: bool,
}

impl Default for CCtxParamsState {
    fn default() -> Self {
        Self {
            compression_level: crate::ffi::types::ZSTD_CLEVEL_DEFAULT,
            params: default_params(),
            nb_workers: 0,
            job_size: 0,
            overlap_log: 0,
            block_delimiters: 0,
            enable_long_distance_matching: false,
            validate_sequences: false,
            enable_seq_producer_fallback: false,
        }
    }
}

impl CCtxParamsState {
    fn from_context(ctx: &EncoderContext) -> Self {
        Self {
            compression_level: ctx.compression_level,
            params: ZSTD_parameters {
                cParams: ctx.cparams,
                fParams: ctx.fparams,
            },
            nb_workers: ctx.nb_workers,
            job_size: ctx.job_size,
            overlap_log: ctx.overlap_log,
            block_delimiters: ctx.block_delimiters as c_int,
            enable_long_distance_matching: ctx.enable_long_distance_matching,
            validate_sequences: ctx.validate_sequences,
            enable_seq_producer_fallback: ctx.enable_seq_producer_fallback,
        }
    }

    fn to_context(&self) -> EncoderContext {
        let mut ctx = EncoderContext::default();
        ctx.compression_level = self.compression_level;
        ctx.cparams = self.params.cParams;
        ctx.fparams = self.params.fParams;
        ctx.nb_workers = self.nb_workers;
        ctx.job_size = self.job_size;
        ctx.overlap_log = self.overlap_log;
        ctx.block_delimiters = if self.block_delimiters == 1 {
            crate::ffi::types::ZSTD_sequenceFormat_e::ZSTD_sf_explicitBlockDelimiters
        } else {
            crate::ffi::types::ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters
        };
        ctx.enable_long_distance_matching = self.enable_long_distance_matching;
        ctx.validate_sequences = self.validate_sequences;
        ctx.enable_seq_producer_fallback = self.enable_seq_producer_fallback;
        ctx
    }

    fn apply_to_cctx(&self, cctx: &mut EncoderContext) {
        cctx.compression_level = self.compression_level;
        cctx.apply_params(self.params);
        cctx.nb_workers = self.nb_workers;
        cctx.job_size = self.job_size;
        cctx.overlap_log = self.overlap_log;
        cctx.block_delimiters = if self.block_delimiters == 1 {
            crate::ffi::types::ZSTD_sequenceFormat_e::ZSTD_sf_explicitBlockDelimiters
        } else {
            crate::ffi::types::ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters
        };
        cctx.enable_long_distance_matching = self.enable_long_distance_matching;
        cctx.validate_sequences = self.validate_sequences;
        cctx.enable_seq_producer_fallback = self.enable_seq_producer_fallback;
    }
}

fn params_ref<'a>(ptr: *const ZSTD_CCtx_params) -> Option<&'a CCtxParamsState> {
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &*ptr.cast::<CCtxParamsState>() })
}

fn params_mut<'a>(ptr: *mut ZSTD_CCtx_params) -> Option<&'a mut CCtxParamsState> {
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &mut *ptr.cast::<CCtxParamsState>() })
}

pub(crate) fn context_from_cctx_params(ptr: *const ZSTD_CCtx_params) -> Option<EncoderContext> {
    params_ref(ptr).map(CCtxParamsState::to_context)
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_init(
    cctxParams: *mut ZSTD_CCtx_params,
    compressionLevel: c_int,
) -> usize {
    let Some(cctx_params) = params_mut(cctxParams) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    *cctx_params = CCtxParamsState {
        compression_level: compressionLevel,
        params: ZSTD_parameters {
            cParams: get_cparams(
                compressionLevel,
                crate::ffi::types::ZSTD_CONTENTSIZE_UNKNOWN,
                0,
            ),
            fParams: default_params().fParams,
        },
        ..CCtxParamsState::default()
    };
    0
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCCtxParams(params: *mut ZSTD_CCtx_params) -> usize {
    if params.is_null() {
        return 0;
    }
    unsafe {
        drop(Box::from_raw(params.cast::<CCtxParamsState>()));
    }
    0
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setFParams(
    cctx: *mut ZSTD_CCtx,
    fparams: ZSTD_frameParameters,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.fparams = fparams;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_createCCtxParams() -> *mut ZSTD_CCtx_params {
    Box::into_raw(Box::new(CCtxParamsState::default())).cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_setParameter(
    params: *mut ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    value: c_int,
) -> usize {
    let Some(params) = params_mut(params) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    let mut ctx = params.to_context();
    match set_parameter(&mut ctx, param, value) {
        Ok(()) => {
            *params = CCtxParamsState::from_context(&ctx);
            0
        }
        Err(error) => error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParametersUsingCCtxParams(
    cctx: *mut ZSTD_CCtx,
    params: *const ZSTD_CCtx_params,
) -> usize {
    let Some(params) = params_ref(params) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    to_result(with_cctx_mut(cctx, |cctx| {
        params.apply_to_cctx(cctx);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_getParameter(
    params: *const ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    let Some(params) = params_ref(params) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    let ctx = params.to_context();
    match get_parameter(&ctx, param) {
        Ok(current) => {
            if let Some(value) = unsafe { value.as_mut() } {
                *value = current;
            }
            0
        }
        Err(error) => error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_init_advanced(
    cctxParams: *mut ZSTD_CCtx_params,
    params: ZSTD_parameters,
) -> usize {
    let Some(cctx_params) = params_mut(cctxParams) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    *cctx_params = CCtxParamsState {
        compression_level: crate::ffi::types::ZSTD_CLEVEL_DEFAULT,
        params,
        ..CCtxParamsState::default()
    };
    0
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtxParams_reset(params: *mut ZSTD_CCtx_params) -> usize {
    let Some(params) = params_mut(params) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    *params = CCtxParamsState::default();
    0
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setCParams(
    cctx: *mut ZSTD_CCtx,
    cparams: ZSTD_compressionParameters,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.cparams = crate::ffi::compress::normalize_cparams(cparams);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParams(cctx: *mut ZSTD_CCtx, params: ZSTD_parameters) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.apply_params(params);
        Ok(0)
    }))
}
