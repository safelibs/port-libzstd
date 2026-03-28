use crate::{
    common::error::error_result,
    decompress::{frame, fse},
    ffi::{
        decompress::{self, DictionaryUse},
        types::{
            ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream, ZSTD_customMem,
            ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e,
        },
    },
};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_createDDict(dictBuffer: *const c_void, dictSize: usize) -> *mut ZSTD_DDict {
    let Some(dict) = decompress::optional_src_slice(dictBuffer, dictSize) else {
        return core::ptr::null_mut();
    };
    if dict.is_empty() {
        return core::ptr::null_mut();
    }
    decompress::create_ddict(dict).unwrap_or(core::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> usize {
    decompress::free_ddict(ddict)
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_loadDictionary(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(dctx, |dctx| {
        dctx.load_dictionary(dict_bytes, DictionaryUse::Indefinitely)
    }) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_refDDict(dctx: *mut ZSTD_DCtx, ddict: *const ZSTD_DDict) -> usize {
    match decompress::with_dctx_mut(dctx, |dctx| dctx.ref_ddict(ddict.cast())) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initDStream_usingDict(
    zds: *mut ZSTD_DStream,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(zds, |zds| {
        zds.reset_session();
        zds.load_dictionary(dict_bytes, DictionaryUse::Indefinitely)?;
        Ok(frame::starting_input_length(zds.format))
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initDStream_usingDDict(
    zds: *mut ZSTD_DStream,
    ddict: *const ZSTD_DDict,
) -> usize {
    match decompress::with_dctx_mut(zds, |zds| {
        zds.reset_session();
        zds.ref_ddict(ddict.cast())?;
        Ok(frame::starting_input_length(zds.format))
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromDict(dict: *const c_void, dictSize: usize) -> u32 {
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return 0;
    };
    fse::formatted_dict_id(dict_bytes)
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> u32 {
    decompress::get_dict_id_from_ddict(ddict)
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_DDict(ddict: *const ZSTD_DDict) -> usize {
    decompress::sizeof_ddict(ddict)
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_loadDictionary_byReference(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(dctx, |dctx| {
        dctx.load_dictionary(dict_bytes, DictionaryUse::Indefinitely)
    }) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_createDDict_byReference(
    dictBuffer: *const c_void,
    dictSize: usize,
) -> *mut ZSTD_DDict {
    let Some(dict) = decompress::optional_src_slice(dictBuffer, dictSize) else {
        return core::ptr::null_mut();
    };
    if dict.is_empty() {
        return core::ptr::null_mut();
    }
    decompress::create_ddict(dict).unwrap_or(core::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn ZSTD_createDDict_advanced(
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_DDict {
    let _ = (dictLoadMethod, dictContentType, customMem);
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return core::ptr::null_mut();
    };
    if dict_bytes.is_empty() {
        return core::ptr::null_mut();
    }
    decompress::create_ddict(dict_bytes).unwrap_or(core::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_loadDictionary_advanced(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    let _ = (dictLoadMethod, dictContentType);
    let Some(dict_bytes) = decompress::optional_src_slice(dict, dictSize) else {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };
    match decompress::with_dctx_mut(dctx, |dctx| {
        dctx.load_dictionary(dict_bytes, DictionaryUse::Indefinitely)
    }) {
        Ok(()) => 0,
        Err(code) => error_result(code),
    }
}
