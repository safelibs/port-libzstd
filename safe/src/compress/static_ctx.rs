use crate::ffi::{
    advanced::{forward_extern, null, null_mut},
    types::{
        ZSTD_CCtx, ZSTD_CDict, ZSTD_CStream, ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream,
        ZSTD_compressionParameters, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e,
    },
};
use core::ffi::c_void;

forward_extern! {
    pub fn ZSTD_initStaticCCtx(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_CCtx => null_mut()
}

forward_extern! {
    pub fn ZSTD_estimateDDictSize(
        dictSize: usize,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
    ) -> usize => crate::ffi::compress::generic_error()
}

forward_extern! {
    pub fn ZSTD_initStaticDCtx(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_DCtx => null_mut()
}

forward_extern! {
    pub fn ZSTD_initStaticDDict(
        workspace: *mut c_void,
        workspaceSize: usize,
        dict: *const c_void,
        dictSize: usize,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
    ) -> *const ZSTD_DDict => null()
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
    type Fn = unsafe extern "C" fn(
        *mut c_void,
        usize,
        *const c_void,
        usize,
        ZSTD_dictLoadMethod_e,
        ZSTD_dictContentType_e,
        ZSTD_compressionParameters,
    ) -> *const ZSTD_CDict;
    let Some(func) = crate::ffi::compress::load_upstream!("ZSTD_initStaticCDict", Fn) else {
        return null();
    };
    // SAFETY: The loaded symbol is cached with the exact signature declared above.
    unsafe {
        func(
            workspace,
            workspaceSize,
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
            cParams,
        )
    }
}

forward_extern! {
    pub fn ZSTD_initStaticCStream(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_CStream => null_mut()
}

forward_extern! {
    pub fn ZSTD_initStaticDStream(
        workspace: *mut c_void,
        workspaceSize: usize,
    ) -> *mut ZSTD_DStream => null_mut()
}
