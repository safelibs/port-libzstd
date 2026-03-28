use crate::ffi::{
    decompress::api,
    types::{ZSTD_DCtx, ZSTD_DDict, ZSTD_ResetDirective, ZSTD_dParameter, ZSTD_format_e},
};
use core::ffi::{c_int, c_void};

#[no_mangle]
pub extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    unsafe { (api().create_dctx)() }
}

#[no_mangle]
pub extern "C" fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize {
    unsafe { (api().free_dctx)(dctx) }
}

#[no_mangle]
pub extern "C" fn ZSTD_copyDCtx(dctx: *mut ZSTD_DCtx, preparedDCtx: *const ZSTD_DCtx) {
    unsafe { (api().copy_dctx)(dctx, preparedDCtx) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_reset(dctx: *mut ZSTD_DCtx, reset: ZSTD_ResetDirective) -> usize {
    unsafe { (api().dctx_reset)(dctx, reset) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    compressedSize: usize,
) -> usize {
    if !dst.is_null() && !src.is_null() {
        let legacy = unsafe {
            let dst = std::slice::from_raw_parts_mut(dst.cast::<u8>(), dstCapacity);
            let src = std::slice::from_raw_parts(src.cast::<u8>(), compressedSize);
            crate::decompress::legacy::try_decompress(dst, src)
        };
        if let Some(result) = legacy {
            return match result {
                Ok(size) => size,
                Err(code) => 0usize.wrapping_sub(code as usize),
            };
        }
    }
    unsafe { (api().decompress)(dst, dstCapacity, src, compressedSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressDCtx(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    if !dst.is_null() && !src.is_null() {
        let legacy = unsafe {
            let dst = std::slice::from_raw_parts_mut(dst.cast::<u8>(), dstCapacity);
            let src = std::slice::from_raw_parts(src.cast::<u8>(), srcSize);
            crate::decompress::legacy::try_decompress(dst, src)
        };
        if let Some(result) = legacy {
            return match result {
                Ok(size) => size,
                Err(code) => 0usize.wrapping_sub(code as usize),
            };
        }
    }
    unsafe { (api().decompress_dctx)(dctx, dst, dstCapacity, src, srcSize) }
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
    unsafe { (api().decompress_using_dict)(dctx, dst, dstCapacity, src, srcSize, dict, dictSize) }
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
    unsafe { (api().decompress_using_ddict)(dctx, dst, dstCapacity, src, srcSize, ddict) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBegin(dctx: *mut ZSTD_DCtx) -> usize {
    unsafe { (api().decompress_begin)(dctx) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBegin_usingDict(
    dctx: *mut ZSTD_DCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    unsafe { (api().decompress_begin_using_dict)(dctx, dict, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressBegin_usingDDict(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) -> usize {
    unsafe { (api().decompress_begin_using_ddict)(dctx, ddict) }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressContinue(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    unsafe { (api().decompress_continue)(dctx, dst, dstCapacity, src, srcSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_setParameter(
    dctx: *mut ZSTD_DCtx,
    param: ZSTD_dParameter,
    value: c_int,
) -> usize {
    unsafe { (api().dctx_set_parameter)(dctx, param, value) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_getParameter(
    dctx: *mut ZSTD_DCtx,
    param: ZSTD_dParameter,
    value: *mut c_int,
) -> usize {
    unsafe { (api().dctx_get_parameter)(dctx, param, value) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_setFormat(dctx: *mut ZSTD_DCtx, format: ZSTD_format_e) -> usize {
    unsafe { (api().dctx_set_format)(dctx, format) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_setMaxWindowSize(dctx: *mut ZSTD_DCtx, maxWindowSize: usize) -> usize {
    unsafe { (api().dctx_set_max_window_size)(dctx, maxWindowSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_DCtx_refPrefix(
    dctx: *mut ZSTD_DCtx,
    prefix: *const c_void,
    prefixSize: usize,
) -> usize {
    unsafe { (api().dctx_ref_prefix)(dctx, prefix, prefixSize) }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_DCtx(dctx: *const ZSTD_DCtx) -> usize {
    unsafe { (api().sizeof_dctx)(dctx) }
}
