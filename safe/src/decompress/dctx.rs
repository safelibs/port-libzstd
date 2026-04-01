use crate::{
    common::error::error_result,
    decompress::frame::{self, DictionaryRef},
    ffi::{
        decompress::{self, DictionaryUse},
        types::{
            ZSTD_DCtx, ZSTD_DDict, ZSTD_ResetDirective, ZSTD_customMem, ZSTD_dParameter,
            ZSTD_dictContentType_e, ZSTD_format_e,
        },
    },
};
use core::ffi::{c_int, c_void};

fn custom_mem_supported(custom_mem: ZSTD_customMem) -> bool {
    custom_mem.customAlloc.is_none() && custom_mem.customFree.is_none()
}

fn decode_into(
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
    dict: DictionaryRef<'_>,
    format: ZSTD_format_e,
    max_window_size: usize,
) -> usize {
    let Some(src) = decompress::optional_src_slice(src, src_size) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };

    match frame::decode_all_frames(src, dict, format, max_window_size) {
        Ok(decoded) => frame::copy_decoded_to_ptr(&decoded, dst, dst_capacity),
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    decompress::create_dctx()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    decompress::free_dctx(dctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_copyDCtx(dctx: *mut ZSTD_DCtx, preparedDCtx: *const ZSTD_DCtx) {
    let Ok(snapshot) = decompress::with_dctx_ref(preparedDCtx, |prepared| Ok(prepared.clone()))
    else {
        return;
    };
    let _ = decompress::with_dctx_mut(dctx, |target| {
        target.copy_from(&snapshot);
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_reset(dctx: *mut ZSTD_DCtx, reset: ZSTD_ResetDirective) -> usize {
    let result = decompress::with_dctx_mut(dctx, |dctx| {
        match reset {
            ZSTD_ResetDirective::ZSTD_reset_session_only => {
                dctx.reset_session();
            }
            ZSTD_ResetDirective::ZSTD_reset_parameters => {
                if !dctx.can_set_parameters() {
                    return Err(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_stage_wrong);
                }
                dctx.reset_parameters();
            }
            ZSTD_ResetDirective::ZSTD_reset_session_and_parameters => {
                dctx.reset_session();
                dctx.reset_parameters();
            }
        }
        Ok(())
    });
    match result {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    compressedSize: usize,
) -> usize {
    decode_into(
        dst,
        dstCapacity,
        src,
        compressedSize,
        DictionaryRef::None,
        ZSTD_format_e::ZSTD_f_zstd1,
        (1usize << frame::ZSTD_WINDOWLOG_LIMIT_DEFAULT) + 1,
    )
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressDCtx(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    match decompress::with_dctx_mut(dctx, |dctx| {
        let decoded = decode_into(
            dst,
            dstCapacity,
            src,
            srcSize,
            dctx.resolved_dict()?,
            dctx.format,
            dctx.max_window_size,
        );
        frame::decode_error_result(decoded)?;
        dctx.clear_once_dict();
        Ok(decoded)
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompress_usingDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_ref(dctx, |dctx| {
        if dict_bytes.is_empty() {
            Ok(decode_into(
                dst,
                dstCapacity,
                src,
                srcSize,
                DictionaryRef::None,
                dctx.format,
                dctx.max_window_size,
            ))
        } else {
            let prepared = decompress::DecoderDictionary::from_bytes_with_content_type(
                dict_bytes,
                ZSTD_dictContentType_e::ZSTD_dct_auto,
            )?;
            Ok(decode_into(
                dst,
                dstCapacity,
                src,
                srcSize,
                prepared.as_dictionary_ref(),
                dctx.format,
                dctx.max_window_size,
            ))
        }
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompress_usingDDict(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    ddict: *const ZSTD_DDict,
) -> usize {
    match decompress::with_dctx_ref(dctx, |dctx| Ok((dctx.format, dctx.max_window_size))) {
        Ok((format, max_window_size)) if ddict.is_null() => decode_into(
            dst,
            dstCapacity,
            src,
            srcSize,
            DictionaryRef::None,
            format,
            max_window_size,
        ),
        Ok((format, max_window_size)) => match decompress::with_ddict_ref(ddict, |prepared| {
            Ok(decode_into(
                dst,
                dstCapacity,
                src,
                srcSize,
                prepared.as_dictionary_ref(),
                format,
                max_window_size,
            ))
        }) {
            Ok(size) => size,
            Err(code) => error_result(code),
        },
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBegin(dctx: *mut ZSTD_DCtx) -> usize {
    match decompress::with_dctx_mut(dctx, |dctx| {
        dctx.reset_session();
        decompress::begin_bufferless(dctx);
        Ok(())
    }) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBegin_usingDict(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(dctx, |dctx| {
        dctx.reset_session();
        dctx.load_dictionary(dict_bytes, DictionaryUse::Once)?;
        decompress::begin_bufferless(dctx);
        Ok(())
    }) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBegin_usingDDict(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) -> usize {
    match decompress::with_dctx_mut(dctx, |dctx| {
        dctx.reset_session();
        dctx.ref_ddict(ddict.cast())?;
        decompress::begin_bufferless(dctx);
        Ok(())
    }) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressContinue(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let Some(src_bytes) = decompress::optional_src_slice(src, srcSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(dctx, |dctx| {
        decompress::bufferless_continue(dctx, dst, dstCapacity, src_bytes, false)
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_setParameter(
    dctx: *mut ZSTD_DCtx,
    param: ZSTD_dParameter,
    value: c_int,
) -> usize {
    match decompress::with_dctx_mut(dctx, |dctx| dctx.set_parameter(param, value)) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_getParameter(
    dctx: *mut ZSTD_DCtx,
    param: ZSTD_dParameter,
    value: *mut c_int,
) -> usize {
    if value.is_null() {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    match decompress::with_dctx_ref(dctx, |dctx| dctx.get_parameter(param)) {
        Ok(current) => {
            // SAFETY: The caller provided a valid writable `int*`.
            unsafe {
                *value = current;
            }
            0
        }
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_setFormat(dctx: *mut ZSTD_DCtx, format: ZSTD_format_e) -> usize {
    match decompress::with_dctx_mut(dctx, |dctx| dctx.set_format(format)) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_setMaxWindowSize(dctx: *mut ZSTD_DCtx, maxWindowSize: usize) -> usize {
    match decompress::with_dctx_mut(dctx, |dctx| dctx.set_max_window_size(maxWindowSize)) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_refPrefix(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefixSize: usize,
) -> usize {
    let Some(prefix_bytes) = decompress::optional_src_slice(prefix, prefixSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(dctx, |dctx| dctx.ref_prefix(prefix_bytes)) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_DCtx(dctx: *const ZSTD_DCtx) -> usize {
    decompress::sizeof_dctx(dctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDCtxSize() -> usize {
    crate::common::alloc::base_size::<crate::ffi::decompress::DecoderContext>()
}

#[no_mangle]
pub extern "C" fn ZSTD_createDCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    if !custom_mem_supported(customMem) {
        return core::ptr::null_mut();
    }
    decompress::create_dctx()
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_refPrefix_advanced(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefixSize: usize,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    let Some(prefix_bytes) = decompress::optional_src_slice(prefix, prefixSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(dctx, |dctx| {
        dctx.ref_prefix_with_content_type(prefix_bytes, dictContentType)
    }) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}
