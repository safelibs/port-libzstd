use crate::ffi::{
    decompress::api,
    types::{ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream},
};
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn ZSTD_createDDict(dictBuffer: *const c_void, dictSize: usize) -> *mut ZSTD_DDict {
    unsafe { (api().create_ddict)(dictBuffer, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> usize {
    unsafe { (api().free_ddict)(ddict) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_loadDictionary(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    unsafe { (api().dctx_load_dictionary)(dctx, dict, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_refDDict(dctx: *mut ZSTD_DCtx, ddict: *const ZSTD_DDict) -> usize {
    unsafe { (api().dctx_ref_ddict)(dctx, ddict) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initDStream_usingDict(
    zds: *mut ZSTD_DStream,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    unsafe { (api().init_dstream_using_dict)(zds, dict, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_initDStream_usingDDict(
    zds: *mut ZSTD_DStream,
    ddict: *const ZSTD_DDict,
) -> usize {
    unsafe { (api().init_dstream_using_ddict)(zds, ddict) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromDict(dict: *const c_void, dictSize: usize) -> u32 {
    unsafe { (api().get_dict_id_from_dict)(dict, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> u32 {
    unsafe { (api().get_dict_id_from_ddict)(ddict) }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_DDict(ddict: *const ZSTD_DDict) -> usize {
    unsafe { (api().sizeof_ddict)(ddict) }
}
