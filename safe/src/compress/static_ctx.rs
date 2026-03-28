use crate::ffi::{
    advanced::{null, null_mut},
    types::{
        ZSTD_CCtx, ZSTD_CDict, ZSTD_CStream, ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream,
        ZSTD_compressionParameters, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e,
    },
};
use core::ffi::c_void;

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticCCtx"]
    fn internal_ZSTD_initStaticCCtx(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_CCtx;
    #[link_name = "libzstd_safe_internal_ZSTD_estimateDDictSize"]
    fn internal_ZSTD_estimateDDictSize(
        dictSize: usize,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticDCtx"]
    fn internal_ZSTD_initStaticDCtx(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_DCtx;
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticDDict"]
    fn internal_ZSTD_initStaticDDict(
        workspace: *mut c_void,
        workspaceSize: usize,
        dict: *const c_void,
        dictSize: usize,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
    ) -> *const ZSTD_DDict;
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticCDict"]
    fn internal_ZSTD_initStaticCDict(
        workspace: *mut c_void,
        workspaceSize: usize,
        dict: *const c_void,
        dictSize: usize,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
        cParams: ZSTD_compressionParameters,
    ) -> *const ZSTD_CDict;
    #[link_name = "libzstd_safe_internal_ZSTD_initStaticCStream"]
    fn internal_ZSTD_initStaticCStream(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_CStream;
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
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    let ctx = unsafe { internal_ZSTD_initStaticCCtx(workspace, workspaceSize) };
    if ctx.is_null() {
        null_mut()
    } else {
        ctx
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDDictSize(
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZSTD_estimateDDictSize(dictSize, dictLoadMethod) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDCtx(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DCtx {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
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
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
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
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
) -> *const ZSTD_CDict {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    let cdict = unsafe {
        internal_ZSTD_initStaticCDict(
            workspace,
            workspaceSize,
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
            cParams,
        )
    };
    if cdict.is_null() {
        null()
    } else {
        cdict
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticCStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CStream {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    let stream = unsafe { internal_ZSTD_initStaticCStream(workspace, workspaceSize) };
    if stream.is_null() {
        null_mut()
    } else {
        stream
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_initStaticDStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_DStream {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    let stream = unsafe { internal_ZSTD_initStaticDStream(workspace, workspaceSize) };
    if stream.is_null() {
        null_mut()
    } else {
        stream
    }
}
