use crate::ffi::{
    advanced::{null, null_mut},
    compress::{
        default_cparams, init_static_cctx, init_static_cdict, normalize_cparams, optional_src_slice,
    },
    decompress::{init_static_dctx, init_static_ddict},
    types::{
        ZSTD_CCtx, ZSTD_CDict, ZSTD_CStream, ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream,
        ZSTD_compressionParameters, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e,
        ZSTD_sequenceFormat_e, ZSTD_CLEVEL_DEFAULT,
    },
};
use core::ffi::{c_int, c_void};

fn has_workspace(workspace: *mut c_void, workspace_size: usize) -> bool {
    !workspace.is_null() && workspace_size != 0
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticCCtx(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CCtx {
    if !has_workspace(workspace, workspaceSize) {
        return null_mut();
    }
    init_static_cctx(workspace, workspaceSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDDictSize(
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    crate::common::alloc::base_size::<crate::ffi::decompress::DecoderDictionary>().saturating_add(
        match dictLoadMethod {
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byCopy => dictSize,
            ZSTD_dictLoadMethod_e::ZSTD_dlm_byRef => 0,
        },
    )
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDCtx(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DCtx {
    if !has_workspace(workspace, workspaceSize) {
        return null_mut();
    }
    init_static_dctx(workspace, workspaceSize)
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDDict(
    workspace: *mut c_void,
    workspaceSize: usize,
    dict: *const c_void,
    dictSize: usize,
    _dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> *const ZSTD_DDict {
    if !has_workspace(workspace, workspaceSize) {
        return null();
    }
    let Some(dict) = optional_src_slice(dict, dictSize) else {
        return null();
    };
    match init_static_ddict(
        workspace,
        workspaceSize,
        dict,
        _dictLoadMethod,
        dictContentType,
    ) {
        Ok(ddict) => ddict.cast_const(),
        Err(_) => null(),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticCDict(
    workspace: *mut c_void,
    workspaceSize: usize,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
) -> *const ZSTD_CDict {
    if !has_workspace(workspace, workspaceSize) {
        return null();
    }
    let Some(dict) = optional_src_slice(dict, dictSize) else {
        return null();
    };

    let cparams = if cParams == ZSTD_compressionParameters::default() {
        default_cparams()
    } else {
        normalize_cparams(cParams)
    };
    let compression_level = (cparams.strategy as c_int).max(ZSTD_CLEVEL_DEFAULT);
    init_static_cdict(
        workspace,
        workspaceSize,
        dict,
        compression_level,
        cparams,
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
        ZSTD_sequenceFormat_e::ZSTD_sf_noBlockDelimiters,
        false,
        0,
        false,
        dictLoadMethod,
        dictContentType,
    )
    .cast_const()
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticCStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CStream {
    if !has_workspace(workspace, workspaceSize) {
        return null_mut();
    }
    init_static_cctx(workspace, workspaceSize).cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DStream {
    if !has_workspace(workspace, workspaceSize) {
        return null_mut();
    }
    init_static_dctx(workspace, workspaceSize).cast()
}
