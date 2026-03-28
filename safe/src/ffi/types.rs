use core::ffi::{c_int, c_uint, c_void};

#[repr(C)]
pub struct ZSTD_CCtx {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ZSTD_DCtx {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ZSTD_CDict {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ZSTD_DDict {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ZSTD_CCtx_params {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ZSTD_threadPool {
    _private: [u8; 0],
}

pub type ZSTD_CStream = ZSTD_CCtx;
pub type ZSTD_DStream = ZSTD_DCtx;

pub type ZSTD_allocFunction =
    Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>;
pub type ZSTD_freeFunction =
    Option<unsafe extern "C" fn(opaque: *mut c_void, address: *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZSTD_bounds {
    pub error: usize,
    pub lowerBound: c_int,
    pub upperBound: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void,
    pub size: usize,
    pub pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void,
    pub size: usize,
    pub pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZSTD_frameType_e {
    ZSTD_frame = 0,
    ZSTD_skippableFrame = 1,
}

impl Default for ZSTD_frameType_e {
    fn default() -> Self {
        Self::ZSTD_frame
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZSTD_frameHeader {
    pub frameContentSize: u64,
    pub windowSize: u64,
    pub blockSizeMax: c_uint,
    pub frameType: ZSTD_frameType_e,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _reserved1: c_uint,
    pub _reserved2: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZSTD_Sequence {
    pub offset: c_uint,
    pub litLength: c_uint,
    pub matchLength: c_uint,
    pub rep: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZSTD_frameProgression {
    pub ingested: u64,
    pub consumed: u64,
    pub produced: u64,
    pub flushed: u64,
    pub currentJobID: c_uint,
    pub nbActiveWorkers: c_uint,
}
