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
pub enum ZSTD_ResetDirective {
    ZSTD_reset_session_only = 1,
    ZSTD_reset_parameters = 2,
    ZSTD_reset_session_and_parameters = 3,
}

impl Default for ZSTD_ResetDirective {
    fn default() -> Self {
        Self::ZSTD_reset_session_only
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZSTD_dParameter {
    ZSTD_d_windowLogMax = 100,
    ZSTD_d_experimentalParam1 = 1000,
    ZSTD_d_experimentalParam2 = 1001,
    ZSTD_d_experimentalParam3 = 1002,
    ZSTD_d_experimentalParam4 = 1003,
    ZSTD_d_experimentalParam5 = 1004,
}

impl Default for ZSTD_dParameter {
    fn default() -> Self {
        Self::ZSTD_d_windowLogMax
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZSTD_format_e {
    ZSTD_f_zstd1 = 0,
    ZSTD_f_zstd1_magicless = 1,
}

impl Default for ZSTD_format_e {
    fn default() -> Self {
        Self::ZSTD_f_zstd1
    }
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZSTD_nextInputType_e {
    ZSTDnit_frameHeader = 0,
    ZSTDnit_blockHeader = 1,
    ZSTDnit_block = 2,
    ZSTDnit_lastBlock = 3,
    ZSTDnit_checksum = 4,
    ZSTDnit_skippableFrame = 5,
}

impl Default for ZSTD_nextInputType_e {
    fn default() -> Self {
        Self::ZSTDnit_frameHeader
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZSTD_ErrorCode {
    ZSTD_error_no_error = 0,
    ZSTD_error_GENERIC = 1,
    ZSTD_error_prefix_unknown = 10,
    ZSTD_error_version_unsupported = 12,
    ZSTD_error_frameParameter_unsupported = 14,
    ZSTD_error_frameParameter_windowTooLarge = 16,
    ZSTD_error_corruption_detected = 20,
    ZSTD_error_checksum_wrong = 22,
    ZSTD_error_literals_headerWrong = 24,
    ZSTD_error_dictionary_corrupted = 30,
    ZSTD_error_dictionary_wrong = 32,
    ZSTD_error_dictionaryCreation_failed = 34,
    ZSTD_error_parameter_unsupported = 40,
    ZSTD_error_parameter_combination_unsupported = 41,
    ZSTD_error_parameter_outOfBound = 42,
    ZSTD_error_tableLog_tooLarge = 44,
    ZSTD_error_maxSymbolValue_tooLarge = 46,
    ZSTD_error_maxSymbolValue_tooSmall = 48,
    ZSTD_error_stabilityCondition_notRespected = 50,
    ZSTD_error_stage_wrong = 60,
    ZSTD_error_init_missing = 62,
    ZSTD_error_memory_allocation = 64,
    ZSTD_error_workSpace_tooSmall = 66,
    ZSTD_error_dstSize_tooSmall = 70,
    ZSTD_error_srcSize_wrong = 72,
    ZSTD_error_dstBuffer_null = 74,
    ZSTD_error_noForwardProgress_destFull = 80,
    ZSTD_error_noForwardProgress_inputEmpty = 82,
    ZSTD_error_frameIndex_tooLarge = 100,
    ZSTD_error_seekableIO = 102,
    ZSTD_error_dstBuffer_wrong = 104,
    ZSTD_error_srcBuffer_wrong = 105,
    ZSTD_error_sequenceProducer_failed = 106,
    ZSTD_error_externalSequences_invalid = 107,
    ZSTD_error_maxCode = 120,
}

impl Default for ZSTD_ErrorCode {
    fn default() -> Self {
        Self::ZSTD_error_no_error
    }
}
