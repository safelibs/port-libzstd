use crate::{
    common::error::error_result,
    ffi::{
        compress::{
            create_cctx, cstream_size_estimate, emit_mt_continue_job,
            estimate_cstream_size_from_cparams, estimate_cstream_size_from_level, finalize_stream,
            flush_stream_data, flush_stream_output, free_cctx, get_cparams, load_dictionary,
            next_input_size_hint, null_cctx, stage_stream_input, stream_pending_bytes, to_result,
            validate_custom_mem, with_cctx_mut,
        },
        types::{
            ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CStream, ZSTD_EndDirective, ZSTD_ResetDirective,
            ZSTD_compressionParameters, ZSTD_customMem, ZSTD_inBuffer, ZSTD_outBuffer,
            ZSTD_parameters, ZSTD_CONTENTSIZE_UNKNOWN,
        },
    },
};
use core::ffi::{c_int, c_void};

fn normalize_legacy_pledged_src_size(pledged_src_size: u64) -> u64 {
    if pledged_src_size == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pledged_src_size
    }
}

fn normalize_legacy_advanced_pledged_src_size(
    params: ZSTD_parameters,
    pledged_src_size: u64,
) -> u64 {
    if pledged_src_size == 0 && params.fParams.contentSizeFlag == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pledged_src_size
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCStream() -> *mut ZSTD_CStream {
    create_cctx().cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> usize {
    free_cctx(zcs.cast())
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream(zcs: *mut ZSTD_CStream, compressionLevel: c_int) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        zcs.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        zcs.stream_mode = true;
        zcs.set_dict(None);
        zcs.compression_level = compressionLevel;
        zcs.cparams = get_cparams(compressionLevel, 0, 0);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_srcSize(
    zcs: *mut ZSTD_CStream,
    compressionLevel: c_int,
    pledgedSrcSize: u64,
) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        let pledged_src_size = normalize_legacy_pledged_src_size(pledgedSrcSize);
        zcs.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        zcs.stream_mode = true;
        zcs.set_dict(None);
        zcs.compression_level = compressionLevel;
        zcs.cparams = get_cparams(compressionLevel, pledged_src_size, 0);
        zcs.pledged_src_size = pledged_src_size;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_usingDict(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        zcs.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        zcs.stream_mode = true;
        zcs.compression_level = compressionLevel;
        zcs.cparams = get_cparams(compressionLevel, 0, dictSize);
        load_dictionary(zcs, dict, dictSize, compressionLevel)?;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_advanced(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: u64,
) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        let pledged_src_size = normalize_legacy_advanced_pledged_src_size(params, pledgedSrcSize);
        zcs.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        zcs.stream_mode = true;
        zcs.apply_params(params);
        zcs.compression_level = params.cParams.strategy as c_int;
        zcs.pledged_src_size = pledged_src_size;
        load_dictionary(zcs, dict, dictSize, zcs.compression_level)?;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_resetCStream(zcs: *mut ZSTD_CStream, pledgedSrcSize: u64) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        zcs.reset(ZSTD_ResetDirective::ZSTD_reset_session_only);
        zcs.stream_mode = true;
        zcs.pledged_src_size = normalize_legacy_pledged_src_size(pledgedSrcSize);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        if zcs.legacy_mode {
            return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_init_missing);
        }
        zcs.stream_mode = true;
        let _ = stage_stream_input(zcs, input, false)?;
        if zcs.pledged_src_size == ZSTD_CONTENTSIZE_UNKNOWN
            && !zcs.stream.deferred_header
            && zcs.stream.frame_started
            && zcs.stream.pending.len() >= 4
            && (zcs.dict.is_some() || zcs.prefix.is_some())
        {
            zcs.stream.pending.truncate(4);
            zcs.stream.pending_pos = zcs.stream.pending_pos.min(4);
            zcs.stream.deferred_header = true;
        }
        flush_stream_output(zcs, output)?;
        Ok(next_input_size_hint(zcs))
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream2(
    cctx: *mut ZSTD_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    endOp: ZSTD_EndDirective,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        if cctx.legacy_mode {
            return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_init_missing);
        }
        cctx.stream_mode = true;
        let mt_continue = matches!(endOp, ZSTD_EndDirective::ZSTD_e_continue);
        let consumed = stage_stream_input(cctx, input, mt_continue)?;
        match endOp {
            ZSTD_EndDirective::ZSTD_e_end => finalize_stream(cctx)?,
            ZSTD_EndDirective::ZSTD_e_flush => flush_stream_data(cctx)?,
            ZSTD_EndDirective::ZSTD_e_continue => {
                if consumed == 0 {
                    let _ = emit_mt_continue_job(cctx)?;
                }
            }
        }
        flush_stream_output(cctx, output)?;
        Ok(stream_pending_bytes(cctx))
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_flushStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        if !zcs.stream_mode {
            return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_init_missing);
        }
        flush_stream_data(zcs)?;
        flush_stream_output(zcs, output)?;
        Ok(stream_pending_bytes(zcs))
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_endStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        if !zcs.stream_mode {
            return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_init_missing);
        }
        finalize_stream(zcs)?;
        flush_stream_output(zcs, output)?;
        Ok(stream_pending_bytes(zcs))
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CStreamInSize() -> usize {
    crate::ffi::types::ZSTD_BLOCKSIZE_MAX
}

#[no_mangle]
pub extern "C" fn ZSTD_CStreamOutSize() -> usize {
    crate::ffi::compress::compress_bound(crate::ffi::types::ZSTD_BLOCKSIZE_MAX) + 16
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_CStream(zcs: *const ZSTD_CStream) -> usize {
    crate::ffi::compress::sizeof_cctx(zcs.cast())
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCStreamSize_usingCParams(
    cParams: ZSTD_compressionParameters,
) -> usize {
    match crate::ffi::compress::check_cparams(cParams) {
        Ok(()) => estimate_cstream_size_from_cparams(cParams),
        Err(error) => error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CStream {
    if !validate_custom_mem(customMem) {
        null_cctx().cast()
    } else {
        create_cctx().cast()
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_compressStream2_simpleArgs(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    dstPos: *mut usize,
    src: *const c_void,
    srcSize: usize,
    srcPos: *mut usize,
    endOp: ZSTD_EndDirective,
) -> usize {
    let mut input = ZSTD_inBuffer {
        src,
        size: srcSize,
        pos: unsafe { srcPos.as_ref().copied().unwrap_or(0) },
    };
    let mut output = ZSTD_outBuffer {
        dst,
        size: dstCapacity,
        pos: unsafe { dstPos.as_ref().copied().unwrap_or(0) },
    };
    let result = ZSTD_compressStream2(cctx, &mut output, &mut input, endOp);
    if let Some(dstPos) = unsafe { dstPos.as_mut() } {
        *dstPos = output.pos;
    }
    if let Some(srcPos) = unsafe { srcPos.as_mut() } {
        *srcPos = input.pos;
    }
    result
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCStreamSize(compressionLevel: c_int) -> usize {
    estimate_cstream_size_from_level(compressionLevel)
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCStreamSize_usingCCtxParams(
    params: *const ZSTD_CCtx_params,
) -> usize {
    to_result(cstream_size_estimate(params))
}
