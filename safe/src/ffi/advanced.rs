pub(crate) fn null_mut<T>() -> *mut T {
    core::ptr::null_mut()
}

pub(crate) fn null<T>() -> *const T {
    core::ptr::null()
}
