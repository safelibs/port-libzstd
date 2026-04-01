use crate::{
    common::error::error_result,
    ffi::{
        compress::{
            create_cctx, finalize_stream, flush_pending_to_dst, free_cctx, get_cparams,
            load_dictionary, normalize_cparams, null_cctx, optional_src_slice, sizeof_cctx,
            stage_legacy_input, stage_src_slice, to_result, validate_custom_mem, with_cctx_mut,
            with_cctx_ref, write_frame_to_dst, EncoderContext,
        },
        types::{
            ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_ErrorCode, ZSTD_ResetDirective, ZSTD_cParameter,
            ZSTD_compressionParameters, ZSTD_customMem, ZSTD_dictContentType_e, ZSTD_parameters,
        },
    },
};
use core::ffi::{c_int, c_void};

#[no_mangle]
pub extern "C" fn ZSTD_createCCtx() -> *mut ZSTD_CCtx {
    create_cctx()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> usize {
    free_cctx(cctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBound(srcSize: usize) -> usize {
    crate::ffi::compress::compress_bound(srcSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_compress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    compressionLevel: c_int,
) -> usize {
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    let mut ctx = EncoderContext::default();
    ctx.compression_level = compressionLevel;
    ctx.cparams = get_cparams(compressionLevel, src.len() as u64, 0);
    to_result(write_frame_to_dst(&ctx, dst, dstCapacity, src))
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
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_ref(cctx, |_| {
        let mut fresh = EncoderContext::default();
        fresh.compression_level = compressionLevel;
        fresh.cparams = get_cparams(compressionLevel, src.len() as u64, 0);
        write_frame_to_dst(&fresh, dst, dstCapacity, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compress2(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_ref(cctx, |cctx| {
        if cctx.sequence_producer.is_some() && !cctx.enable_seq_producer_fallback {
            return Err(ZSTD_ErrorCode::ZSTD_error_sequenceProducer_failed);
        }
        write_frame_to_dst(cctx, dst, dstCapacity, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_copyCCtx(
    cctx: *mut ZSTD_CCtx,
    preparedCCtx: *const ZSTD_CCtx,
    pledgedSrcSize: u64,
) -> usize {
    to_result(with_cctx_ref(preparedCCtx, |prepared| {
        let snapshot = prepared.clone();
        with_cctx_mut(cctx, |cctx| {
            let static_workspace_size = cctx.static_workspace_size;
            *cctx = snapshot;
            cctx.static_workspace_size = static_workspace_size;
            cctx.pledged_src_size = pledgedSrcSize;
            Ok(0)
        })
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_reset(cctx: *mut ZSTD_CCtx, reset: ZSTD_ResetDirective) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.reset(reset);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setParameter(
    cctx: *mut ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: c_int,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        crate::ffi::compress::set_parameter(cctx, param, value)?;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_getParameter(
    cctx: *mut ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    to_result(with_cctx_ref(cctx, |cctx| {
        let current = crate::ffi::compress::get_parameter(cctx, param)?;
        if let Some(value) = unsafe { value.as_mut() } {
            *value = current;
        }
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_setPledgedSrcSize(cctx: *mut ZSTD_CCtx, pledgedSrcSize: u64) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        if cctx.stream.frame_finished && cctx.stream.pending_pos == cctx.stream.pending.len() {
            cctx.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        }
        cctx.pledged_src_size = pledgedSrcSize;
        Ok(0)
    }))
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
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_ref(cctx, |base| {
        let mut temp = base.clone();
        temp.compression_level = compressionLevel;
        temp.cparams = get_cparams(compressionLevel, src.len() as u64, dictSize);
        load_dictionary(&mut temp, dict, dictSize, compressionLevel)?;
        write_frame_to_dst(&temp, dst, dstCapacity, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refPrefix(
    cctx: *mut ZSTD_CCtx,
    prefix: *const c_void,
    prefixSize: usize,
) -> usize {
    let prefix = optional_src_slice(prefix, prefixSize);
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.set_prefix(prefix, ZSTD_dictContentType_e::ZSTD_dct_auto);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin(cctx: *mut ZSTD_CCtx, compressionLevel: c_int) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        cctx.set_dict(None);
        cctx.compression_level = compressionLevel;
        cctx.cparams = get_cparams(compressionLevel, 0, 0);
        crate::ffi::compress::legacy_begin(cctx);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_usingDict(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        cctx.compression_level = compressionLevel;
        cctx.cparams = get_cparams(compressionLevel, 0, dictSize);
        load_dictionary(cctx, dict, dictSize, compressionLevel)?;
        crate::ffi::compress::legacy_begin(cctx);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: u64,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        cctx.apply_params(params);
        cctx.compression_level = params.cParams.strategy as c_int;
        load_dictionary(cctx, dict, dictSize, cctx.compression_level)?;
        crate::ffi::compress::legacy_begin(cctx);
        cctx.pledged_src_size = pledgedSrcSize;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressContinue(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_mut(cctx, |cctx| {
        if !cctx.legacy_mode {
            return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_init_missing);
        }
        if cctx.dict.is_some() || cctx.prefix.is_some() {
            stage_src_slice(cctx, src)?;
        } else {
            stage_legacy_input(cctx, src, false)?;
        }
        flush_pending_to_dst(cctx, dst, dstCapacity)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressEnd(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_mut(cctx, |cctx| {
        if !cctx.legacy_mode {
            return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_init_missing);
        }
        let result = if cctx.dict.is_some() || cctx.prefix.is_some() {
            stage_src_slice(cctx, src)?;
            finalize_stream(cctx).and_then(|_| flush_pending_to_dst(cctx, dst, dstCapacity))
        } else {
            stage_legacy_input(cctx, src, true)?;
            flush_pending_to_dst(cctx, dst, dstCapacity)
        };
        if result.is_ok() {
            cctx.clear_session();
        }
        result
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_CCtx(cctx: *const ZSTD_CCtx) -> usize {
    sizeof_cctx(cctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refPrefix_advanced(
    cctx: *mut ZSTD_CCtx,
    prefix: *const c_void,
    prefixSize: usize,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    let prefix = optional_src_slice(prefix, prefixSize);
    to_result(with_cctx_mut(cctx, |cctx| {
        cctx.set_prefix(prefix, dictContentType);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCCtxSize(compressionLevel: c_int) -> usize {
    crate::ffi::compress::estimate_cctx_size_from_level(compressionLevel)
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCCtxSize_usingCCtxParams(params: *const ZSTD_CCtx_params) -> usize {
    to_result(crate::ffi::compress::cctx_params_size(params))
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
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_ref(cctx, |base| {
        let mut temp = base.clone();
        temp.apply_params(params);
        temp.cparams = normalize_cparams(params.cParams);
        if dictSize != 0 {
            let level = temp.compression_level;
            load_dictionary(&mut temp, dict, dictSize, level)?;
        }
        write_frame_to_dst(&temp, dst, dstCapacity, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCCtxSize_usingCParams(cParams: ZSTD_compressionParameters) -> usize {
    match crate::ffi::compress::check_cparams(cParams) {
        Ok(()) => crate::ffi::compress::estimate_cctx_size_from_cparams(cParams),
        Err(error) => error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx {
    if !validate_custom_mem(customMem) {
        null_cctx()
    } else {
        create_cctx()
    }
}
