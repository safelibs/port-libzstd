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
    enable_dedicated_dict_search: bool,
    ldm_hash_log: c_int,
    ldm_min_match: c_int,
    ldm_bucket_size_log: c_int,
    ldm_hash_rate_log: c_int,
    validate_sequences: bool,
    rsyncable: c_int,
    literal_compression_mode: c_int,
    target_cblock_size: c_int,
    src_size_hint: c_int,
    use_row_match_finder: c_int,
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
            enable_dedicated_dict_search: false,
            ldm_hash_log: 0,
            ldm_min_match: 0,
            ldm_bucket_size_log: 0,
            ldm_hash_rate_log: 0,
            validate_sequences: false,
            rsyncable: 0,
            literal_compression_mode: 0,
            target_cblock_size: 0,
            src_size_hint: 0,
            use_row_match_finder: 0,
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
            enable_dedicated_dict_search: ctx.enable_dedicated_dict_search,
            ldm_hash_log: ctx.ldm_hash_log,
            ldm_min_match: ctx.ldm_min_match,
            ldm_bucket_size_log: ctx.ldm_bucket_size_log,
            ldm_hash_rate_log: ctx.ldm_hash_rate_log,
            validate_sequences: ctx.validate_sequences,
            rsyncable: ctx.rsyncable,
            literal_compression_mode: ctx.literal_compression_mode,
            target_cblock_size: ctx.target_cblock_size,
            src_size_hint: ctx.src_size_hint,
            use_row_match_finder: ctx.use_row_match_finder,
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
        ctx.enable_dedicated_dict_search = self.enable_dedicated_dict_search;
        ctx.ldm_hash_log = self.ldm_hash_log;
        ctx.ldm_min_match = self.ldm_min_match;
        ctx.ldm_bucket_size_log = self.ldm_bucket_size_log;
        ctx.ldm_hash_rate_log = self.ldm_hash_rate_log;
        ctx.validate_sequences = self.validate_sequences;
        ctx.rsyncable = self.rsyncable;
        ctx.literal_compression_mode = self.literal_compression_mode;
        ctx.target_cblock_size = self.target_cblock_size;
        ctx.src_size_hint = self.src_size_hint;
        ctx.use_row_match_finder = self.use_row_match_finder;
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
        cctx.enable_dedicated_dict_search = self.enable_dedicated_dict_search;
        cctx.ldm_hash_log = self.ldm_hash_log;
        cctx.ldm_min_match = self.ldm_min_match;
        cctx.ldm_bucket_size_log = self.ldm_bucket_size_log;
        cctx.ldm_hash_rate_log = self.ldm_hash_rate_log;
        cctx.validate_sequences = self.validate_sequences;
        cctx.rsyncable = self.rsyncable;
        cctx.literal_compression_mode = self.literal_compression_mode;
        cctx.target_cblock_size = self.target_cblock_size;
        cctx.src_size_hint = self.src_size_hint;
        cctx.use_row_match_finder = self.use_row_match_finder;
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
