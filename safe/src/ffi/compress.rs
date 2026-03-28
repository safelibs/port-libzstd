use crate::{
    common::error::error_result,
    ffi::types::{
        ZSTD_CCtx, ZSTD_CDict, ZSTD_ErrorCode, ZSTD_bounds,
        ZSTD_compressionParameters, ZSTD_parameters,
    },
};
use core::ffi::{c_char, c_int, c_void};
use std::sync::OnceLock;

const PRIMARY_LIBZSTD_PATH: &[u8] =
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../original/libzstd-1.5.5+dfsg2/lib/libzstd.so.1.5.5\0"
    )
    .as_bytes();
const FALLBACK_LIBZSTD_PATH: &[u8] = b"libzstd.so.1\0";

const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;
const RTLD_DEEPBIND: c_int = 0x00008;

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

fn upstream_handle() -> Option<*mut c_void> {
    static HANDLE: OnceLock<Option<usize>> = OnceLock::new();

    HANDLE
        .get_or_init(|| {
            let flags = RTLD_NOW | RTLD_LOCAL | RTLD_DEEPBIND;
            for path in [PRIMARY_LIBZSTD_PATH, FALLBACK_LIBZSTD_PATH] {
                // SAFETY: `path` is NUL-terminated and valid for the duration of the program.
                let handle = unsafe { dlopen(path.as_ptr().cast(), flags) };
                if !handle.is_null() {
                    return Some(handle as usize);
                }
            }
            None
        })
        .map(|handle| handle as *mut c_void)
}

pub(crate) unsafe fn load_symbol<T: Copy>(name: &[u8]) -> Option<T> {
    let handle = upstream_handle()?;
    // SAFETY: `name` is required to be NUL-terminated by callers.
    let symbol = unsafe { dlsym(handle, name.as_ptr().cast()) };
    if symbol.is_null() {
        return None;
    }
    // SAFETY: `symbol` points to a function with the exact requested signature.
    Some(unsafe { core::mem::transmute_copy(&symbol) })
}

pub(crate) fn generic_error() -> usize {
    error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC)
}

pub(crate) fn null_cctx() -> *mut ZSTD_CCtx {
    core::ptr::null_mut()
}

pub(crate) fn null_cdict() -> *mut ZSTD_CDict {
    core::ptr::null_mut()
}

pub(crate) fn bounds_error() -> ZSTD_bounds {
    ZSTD_bounds {
        error: generic_error(),
        lowerBound: 0,
        upperBound: 0,
    }
}

pub(crate) fn default_cparams() -> ZSTD_compressionParameters {
    ZSTD_compressionParameters::default()
}

pub(crate) fn default_params() -> ZSTD_parameters {
    ZSTD_parameters::default()
}

macro_rules! load_upstream {
    ($name:literal, $ty:ty) => {{
        static CELL: ::std::sync::OnceLock<Option<$ty>> = ::std::sync::OnceLock::new();
        *CELL.get_or_init(|| {
            // SAFETY: The requested symbol name is tied to the exact function pointer type used here.
            unsafe { $crate::ffi::compress::load_symbol::<$ty>(concat!($name, "\0").as_bytes()) }
        })
    }};
}

pub(crate) use load_upstream;
