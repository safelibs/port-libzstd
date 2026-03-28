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

macro_rules! forward_extern {
    ($(#[$meta:meta])* $vis:vis fn $name:ident($($arg:ident : $ty:ty),* $(,)?) -> $ret:ty => $fallback:expr) => {
        #[no_mangle]
        $(#[$meta])*
        $vis extern "C" fn $name($($arg: $ty),*) -> $ret {
            type Fn = unsafe extern "C" fn($($ty),*) -> $ret;
            let Some(func) = $crate::ffi::compress::load_upstream!(stringify!($name), Fn) else {
                return $fallback;
            };
            // SAFETY: The loaded symbol is cached with the exact signature declared above.
            unsafe { func($($arg),*) }
        }
    };
}

macro_rules! forward_extern_void {
    ($(#[$meta:meta])* $vis:vis fn $name:ident($($arg:ident : $ty:ty),* $(,)?)) => {
        #[no_mangle]
        $(#[$meta])*
        $vis extern "C" fn $name($($arg: $ty),*) {
            type Fn = unsafe extern "C" fn($($ty),*);
            let Some(func) = $crate::ffi::compress::load_upstream!(stringify!($name), Fn) else {
                return;
            };
            // SAFETY: The loaded symbol is cached with the exact signature declared above.
            unsafe { func($($arg),*) }
        }
    };
}

pub(crate) use forward_extern;
pub(crate) use forward_extern_void;
