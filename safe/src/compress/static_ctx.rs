use crate::ffi::{
    advanced::{null, null_mut},
    compress::{create_cctx, create_cdict, optional_src_slice},
    types::{
        ZSTD_CCtx, ZSTD_CDict, ZSTD_CStream, ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream,
        ZSTD_compressionParameters, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e,
    },
};
use core::ffi::{c_int, c_void};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_estimateDDictSize"]
    fn internal_ZSTD_estimateDDictSize(
        dictSize: usize,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticDCtx"]
    fn internal_ZSTD_initStaticDCtx(workspace: *mut c_void, workspaceSize: usize)
        -> *mut ZSTD_DCtx;
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticDDict"]
    fn internal_ZSTD_initStaticDDict(
        workspace: *mut c_void,
        workspaceSize: usize,
        dict: *const c_void,
        dictSize: usize,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
    ) -> *const ZSTD_DDict;
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticDStream"]
    fn internal_ZSTD_initStaticDStream(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_DStream;
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticCCtx(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CCtx {
    if workspace.is_null() || workspaceSize == 0 {
        null_mut()
    } else {
        create_cctx()
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDDictSize(
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    unsafe { internal_ZSTD_estimateDDictSize(dictSize, dictLoadMethod) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDCtx(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DCtx {
    let ctx = unsafe { internal_ZSTD_initStaticDCtx(workspace, workspaceSize) };
    if ctx.is_null() {
        null_mut()
    } else {
        ctx
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDDict(
    workspace: *mut c_void,
    workspaceSize: usize,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> *const ZSTD_DDict {
    let ddict = unsafe {
        internal_ZSTD_initStaticDDict(
            workspace,
            workspaceSize,
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
        )
    };
    if ddict.is_null() {
        null()
    } else {
        ddict
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticCDict(
    workspace: *mut c_void,
    workspaceSize: usize,
    dict: *const c_void,
    dictSize: usize,
    _dictLoadMethod: ZSTD_dictLoadMethod_e,
    _dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
) -> *const ZSTD_CDict {
    if workspace.is_null() || workspaceSize == 0 {
        return null();
    }
    let Some(dict) = optional_src_slice(dict, dictSize) else {
        return null();
    };
    let compression_level = (cParams.strategy as c_int).max(1);
    create_cdict(dict, compression_level).cast_const()
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticCStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CStream {
    if workspace.is_null() || workspaceSize == 0 {
        null_mut()
    } else {
        create_cctx().cast()
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DStream {
    let stream = unsafe { internal_ZSTD_initStaticDStream(workspace, workspaceSize) };
    if stream.is_null() {
        null_mut()
    } else {
        stream
    }
}
