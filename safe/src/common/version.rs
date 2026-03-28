use crate::ffi::decompress::api;
use core::ffi::c_char;

#[no_mangle]
pub extern "C" fn ZSTD_versionNumber() -> u32 {
    unsafe { (api().version_number)() }
}

#[no_mangle]
pub extern "C" fn ZSTD_versionString() -> *const c_char {
    unsafe { (api().version_string)() }
}
