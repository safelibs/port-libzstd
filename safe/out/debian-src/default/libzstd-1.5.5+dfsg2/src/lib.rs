#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod common;
pub mod compress;
pub mod decompress;
pub mod dict_builder;
pub mod ffi;
#[cfg(libzstd_threading)]
pub mod threading;

pub const ABI_SONAME: &str = "libzstd.so.1";
pub const ABI_VERSION: &str = "1.5.5";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreadingMode {
    SingleThreaded,
    MultiThreaded,
}

impl ThreadingMode {
    pub const fn current() -> Self {
        if cfg!(libzstd_threading) {
            Self::MultiThreaded
        } else {
            Self::SingleThreaded
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildConfig {
    pub threading_mode: ThreadingMode,
    pub default_artifact: &'static str,
    pub variant_suffix: &'static str,
}

pub const fn build_config() -> BuildConfig {
    BuildConfig {
        threading_mode: ThreadingMode::current(),
        default_artifact: env!("LIBZSTD_DEFAULT_ARTIFACT"),
        variant_suffix: env!("LIBZSTD_VARIANT_SUFFIX"),
    }
}
