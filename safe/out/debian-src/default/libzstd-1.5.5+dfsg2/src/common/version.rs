use core::ffi::c_char;

const VERSION_NUMBER: u32 = 10_505;
static VERSION_STRING: &[u8] = b"1.5.5\0";

#[no_mangle]
pub extern "C" fn ZSTD_versionNumber() -> u32 {
    VERSION_NUMBER
}

#[no_mangle]
pub extern "C" fn ZSTD_versionString() -> *const c_char {
    VERSION_STRING.as_ptr().cast()
}
