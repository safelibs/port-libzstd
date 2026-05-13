use crate::{
    common::error::error_result,
    compress::cctx_params::context_from_cctx_params,
    ffi::{
        compress::{
            adjust_cparams, cdict_size_estimate, cdict_size_estimate_advanced, create_cdict,
            create_cdict_with_settings, free_cdict, load_dictionary, load_dictionary_advanced,
            null_cdict, optional_src_slice, sizeof_cdict, to_result, validate_custom_mem,
            with_cctx_mut, with_cctx_ref, with_cdict_ref, write_frame_to_dst,
        },
        types::{
            ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CDict, ZSTD_CStream, ZSTD_customMem,
            ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e, ZSTD_frameParameters,
        },
    },
};
use core::ffi::{c_int, c_void};

#[no_mangle]
pub extern "C" fn ZSTD_createCDict(
    dictBuffer: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> *mut ZSTD_CDict {
    let Some(dict) = optional_src_slice(dictBuffer, dictSize) else {
        return null_cdict();
    };
    create_cdict(dict, compressionLevel)
}

#[no_mangle]
pub extern "C" fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> usize {
    free_cdict(cdict)
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromCDict(cdict: *const ZSTD_CDict) -> u32 {
    with_cdict_ref(cdict, |cdict| Ok(cdict.dict_id)).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_loadDictionary(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        load_dictionary(cctx, dict, dictSize, cctx.compression_level).map(|_| 0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_refCDict(cctx: *mut ZSTD_CCtx, cdict: *const ZSTD_CDict) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        if cdict.is_null() {
            cctx.set_dict(None);
        } else {
            let dict = with_cdict_ref(cdict, |cdict| Ok(cdict.clone()))?;
            cctx.apply_cdict(dict);
        }
        Ok(0)
    }))
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
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_ref(cctx, |base| {
        let dict = with_cdict_ref(cdict, |cdict| Ok(cdict.clone()))?;
        let mut temp = base.clone();
        temp.apply_cdict(dict);
        write_frame_to_dst(&temp, dst, dstCapacity, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_usingCDict(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        zcs.reset(crate::ffi::types::ZSTD_ResetDirective::ZSTD_reset_session_only);
        zcs.stream_mode = true;
        if cdict.is_null() {
            zcs.set_dict(None);
        } else {
            let dict = with_cdict_ref(cdict, |cdict| Ok(cdict.clone()))?;
            zcs.apply_cdict(dict);
        }
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_usingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        let dict = with_cdict_ref(cdict, |cdict| Ok(cdict.clone()))?;
        cctx.reset(crate::ffi::types::ZSTD_ResetDirective::ZSTD_reset_session_only);
        cctx.apply_cdict(dict);
        crate::ffi::compress::legacy_begin(cctx);
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_compressBegin_usingCDict_advanced(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        let dict = with_cdict_ref(cdict, |cdict| Ok(cdict.clone()))?;
        cctx.reset(crate::ffi::types::ZSTD_ResetDirective::ZSTD_reset_session_only);
        cctx.apply_cdict(dict);
        cctx.fparams = fParams;
        crate::ffi::compress::legacy_begin(cctx);
        cctx.pledged_src_size = pledgedSrcSize;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_initCStream_usingCDict_advanced(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    to_result(with_cctx_mut(zcs.cast(), |zcs| {
        zcs.reset(crate::ffi::types::ZSTD_ResetDirective::ZSTD_reset_session_only);
        zcs.stream_mode = true;
        zcs.pledged_src_size = pledgedSrcSize;
        zcs.fparams = fParams;
        if cdict.is_null() {
            zcs.set_dict(None);
        } else {
            let dict = with_cdict_ref(cdict, |cdict| Ok(cdict.clone()))?;
            zcs.apply_cdict(dict);
        }
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> usize {
    sizeof_cdict(cdict)
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
    let Some(src) = optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    to_result(with_cctx_ref(cctx, |base| {
        let dict = with_cdict_ref(cdict, |cdict| Ok(cdict.clone()))?;
        let mut temp = base.clone();
        temp.apply_cdict(dict);
        temp.fparams = fParams;
        write_frame_to_dst(&temp, dst, dstCapacity, src)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_createCDict_advanced(
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: crate::ffi::types::ZSTD_compressionParameters,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    if !validate_custom_mem(customMem) {
        return null_cdict();
    }
    let Some(dict) = optional_src_slice(dict, dictSize) else {
        return null_cdict();
    };
    create_cdict_with_settings(
        dict,
        crate::ffi::types::ZSTD_CLEVEL_DEFAULT,
        adjust_cparams(
            cParams,
            crate::ffi::types::ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
        ),
        false,
        false,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        crate::ffi::types::ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters,
        false,
        0,
        false,
        dictLoadMethod,
        dictContentType,
    )
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_loadDictionary_byReference(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTD_CCtx_loadDictionary_advanced(
        cctx,
        dict,
        dictSize,
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef,
        ZSTD_dictContentType_e::ZSTD_dct_auto,
    )
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCDictSize_advanced(
    dictSize: usize,
    _cParams: crate::ffi::types::ZSTD_compressionParameters,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    cdict_size_estimate_advanced(dictSize, dictLoadMethod)
}

#[no_mangle]
pub extern "C" fn ZSTD_createCDict_byReference(
    dictBuffer: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> *mut ZSTD_CDict {
    let Some(dict) = optional_src_slice(dictBuffer, dictSize) else {
        return null_cdict();
    };
    create_cdict_with_settings(
        dict,
        compressionLevel,
        crate::ffi::compress::get_cparams(
            compressionLevel,
            crate::ffi::types::ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
        ),
        false,
        false,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        crate::ffi::types::ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters,
        false,
        0,
        false,
        ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef,
        ZSTD_dictContentType_e::ZSTD_dct_auto,
    )
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateCDictSize(dictSize: usize, _compressionLevel: c_int) -> usize {
    cdict_size_estimate(dictSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_CCtx_loadDictionary_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    to_result(with_cctx_mut(cctx, |cctx| {
        load_dictionary_advanced(
            cctx,
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
            cctx.compression_level,
        )?;
        Ok(0)
    }))
}

#[no_mangle]
pub extern "C" fn ZSTD_createCDict_advanced2(
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    _cctxParams: *const ZSTD_CCtx_params,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    if !validate_custom_mem(customMem) {
        return null_cdict();
    }
    let Some(dict) = optional_src_slice(dict, dictSize) else {
        return null_cdict();
    };
    let Some(ctx) = context_from_cctx_params(_cctxParams) else {
        return null_cdict();
    };
    create_cdict_with_settings(
        dict,
        ctx.compression_level,
        adjust_cparams(
            ctx.cparams,
            crate::ffi::types::ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
        ),
        ctx.enable_long_distance_matching,
        ctx.enable_dedicated_dict_search,
        ctx.ldm_hash_log,
        ctx.ldm_min_match,
        ctx.ldm_bucket_size_log,
        ctx.ldm_hash_rate_log,
        ctx.nb_workers,
        ctx.job_size,
        ctx.overlap_log,
        ctx.rsyncable,
        ctx.literal_compression_mode,
        ctx.target_cblock_size,
        ctx.src_size_hint,
        ctx.block_delimiters,
        ctx.validate_sequences,
        ctx.use_row_match_finder,
        ctx.enable_seq_producer_fallback,
        dictLoadMethod,
        dictContentType,
    )
}
