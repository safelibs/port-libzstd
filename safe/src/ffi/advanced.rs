use core::ffi::c_char;

pub(crate) const ZDICT_UNKNOWN_ERROR_NAME: &[u8] = b"ZDICT symbol unavailable\0";

pub(crate) fn null_mut<T>() -> *mut T {
    core::ptr::null_mut()
}

pub(crate) fn null<T>() -> *const T {
    core::ptr::null()
}

pub(crate) fn zdict_unknown_error_name() -> *const c_char {
    ZDICT_UNKNOWN_ERROR_NAME.as_ptr().cast()
}
