use crate::ffi::{decompress::api, types::ZSTD_ErrorCode};
use core::ffi::c_char;

#[no_mangle]
pub extern "C" fn ZSTD_isError(code: usize) -> u32 {
    unsafe { (api().is_error)(code) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getErrorName(code: usize) -> *const c_char {
    unsafe { (api().get_error_name)(code) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getErrorCode(functionResult: usize) -> ZSTD_ErrorCode {
    unsafe { (api().get_error_code)(functionResult) }
}

#[no_mangle]
pub extern "C" fn ZSTD_getErrorString(code: ZSTD_ErrorCode) -> *const c_char {
    unsafe { (api().get_error_string)(code) }
}
