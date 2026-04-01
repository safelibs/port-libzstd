use crate::ffi::types::ZSTD_ErrorCode;
use core::ffi::c_char;

pub(crate) fn error_result(code: ZSTD_ErrorCode) -> usize {
    0usize.wrapping_sub(code as usize)
}

pub(crate) fn is_error_result(code: usize) -> bool {
    code > error_result(ZSTD_ErrorCode::ZSTD_error_maxCode)
}

pub(crate) fn decode_error(code: usize) -> ZSTD_ErrorCode {
    if !is_error_result(code) {
        return ZSTD_ErrorCode::ZSTD_error_no_error;
    }

    match 0usize.wrapping_sub(code) as u32 {
        1 => ZSTD_ErrorCode::ZSTD_error_GENERIC,
        10 => ZSTD_ErrorCode::ZSTD_error_prefix_unknown,
        12 => ZSTD_ErrorCode::ZSTD_error_version_unsupported,
        14 => ZSTD_ErrorCode::ZSTD_error_frameParameter_unsupported,
        16 => ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge,
        20 => ZSTD_ErrorCode::ZSTD_error_corruption_detected,
        22 => ZSTD_ErrorCode::ZSTD_error_checksum_wrong,
        24 => ZSTD_ErrorCode::ZSTD_error_literals_headerWrong,
        30 => ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted,
        32 => ZSTD_ErrorCode::ZSTD_error_dictionary_wrong,
        34 => ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed,
        40 => ZSTD_ErrorCode::ZSTD_error_parameter_unsupported,
        41 => ZSTD_ErrorCode::ZSTD_error_parameter_combination_unsupported,
        42 => ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound,
        44 => ZSTD_ErrorCode::ZSTD_error_tableLog_tooLarge,
        46 => ZSTD_ErrorCode::ZSTD_error_maxSymbolValue_tooLarge,
        48 => ZSTD_ErrorCode::ZSTD_error_maxSymbolValue_tooSmall,
        50 => ZSTD_ErrorCode::ZSTD_error_stabilityCondition_notRespected,
        60 => ZSTD_ErrorCode::ZSTD_error_stage_wrong,
        62 => ZSTD_ErrorCode::ZSTD_error_init_missing,
        64 => ZSTD_ErrorCode::ZSTD_error_memory_allocation,
        66 => ZSTD_ErrorCode::ZSTD_error_workSpace_tooSmall,
        70 => ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall,
        72 => ZSTD_ErrorCode::ZSTD_error_srcSize_wrong,
        74 => ZSTD_ErrorCode::ZSTD_error_dstBuffer_null,
        80 => ZSTD_ErrorCode::ZSTD_error_noForwardProgress_destFull,
        82 => ZSTD_ErrorCode::ZSTD_error_noForwardProgress_inputEmpty,
        100 => ZSTD_ErrorCode::ZSTD_error_frameIndex_tooLarge,
        102 => ZSTD_ErrorCode::ZSTD_error_seekableIO,
        104 => ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong,
        105 => ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong,
        106 => ZSTD_ErrorCode::ZSTD_error_sequenceProducer_failed,
        107 => ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid,
        _ => ZSTD_ErrorCode::ZSTD_error_maxCode,
    }
}

pub(crate) fn error_string(code: ZSTD_ErrorCode) -> &'static [u8] {
    match code {
        ZSTD_ErrorCode::ZSTD_error_no_error => b"No error detected\0",
        ZSTD_ErrorCode::ZSTD_error_GENERIC => b"Error (generic)\0",
        ZSTD_ErrorCode::ZSTD_error_prefix_unknown => b"Unknown frame descriptor\0",
        ZSTD_ErrorCode::ZSTD_error_version_unsupported => b"Version not supported\0",
        ZSTD_ErrorCode::ZSTD_error_frameParameter_unsupported => b"Unsupported frame parameter\0",
        ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge => {
            b"Frame requires too much memory for decoding\0"
        }
        ZSTD_ErrorCode::ZSTD_error_corruption_detected => b"Data corruption detected\0",
        ZSTD_ErrorCode::ZSTD_error_checksum_wrong => b"Restored data doesn't match checksum\0",
        ZSTD_ErrorCode::ZSTD_error_literals_headerWrong => {
            b"Header of Literals' block doesn't respect format specification\0"
        }
        ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted => b"Dictionary is corrupted\0",
        ZSTD_ErrorCode::ZSTD_error_dictionary_wrong => b"Dictionary mismatch\0",
        ZSTD_ErrorCode::ZSTD_error_dictionaryCreation_failed => {
            b"Cannot create Dictionary from provided samples\0"
        }
        ZSTD_ErrorCode::ZSTD_error_parameter_unsupported => b"Unsupported parameter\0",
        ZSTD_ErrorCode::ZSTD_error_parameter_combination_unsupported => {
            b"Unsupported combination of parameters\0"
        }
        ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound => b"Parameter is out of bound\0",
        ZSTD_ErrorCode::ZSTD_error_tableLog_tooLarge => {
            b"tableLog requires too much memory : unsupported\0"
        }
        ZSTD_ErrorCode::ZSTD_error_maxSymbolValue_tooLarge => {
            b"Unsupported max Symbol Value : too large\0"
        }
        ZSTD_ErrorCode::ZSTD_error_maxSymbolValue_tooSmall => {
            b"Specified maxSymbolValue is too small\0"
        }
        ZSTD_ErrorCode::ZSTD_error_stabilityCondition_notRespected => {
            b"pledged buffer stability condition is not respected\0"
        }
        ZSTD_ErrorCode::ZSTD_error_stage_wrong => {
            b"Operation not authorized at current processing stage\0"
        }
        ZSTD_ErrorCode::ZSTD_error_init_missing => b"Context should be init first\0",
        ZSTD_ErrorCode::ZSTD_error_memory_allocation => b"Allocation error : not enough memory\0",
        ZSTD_ErrorCode::ZSTD_error_workSpace_tooSmall => b"workSpace buffer is not large enough\0",
        ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall => b"Destination buffer is too small\0",
        ZSTD_ErrorCode::ZSTD_error_srcSize_wrong => b"Src size is incorrect\0",
        ZSTD_ErrorCode::ZSTD_error_dstBuffer_null => b"Operation on NULL destination buffer\0",
        ZSTD_ErrorCode::ZSTD_error_noForwardProgress_destFull => {
            b"Operation made no progress over multiple calls, due to output buffer being full\0"
        }
        ZSTD_ErrorCode::ZSTD_error_noForwardProgress_inputEmpty => {
            b"Operation made no progress over multiple calls, due to input being empty\0"
        }
        ZSTD_ErrorCode::ZSTD_error_frameIndex_tooLarge => b"Frame index is too large\0",
        ZSTD_ErrorCode::ZSTD_error_seekableIO => b"An I/O error occurred when reading/seeking\0",
        ZSTD_ErrorCode::ZSTD_error_dstBuffer_wrong => b"Destination buffer is wrong\0",
        ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong => b"Source buffer is wrong\0",
        ZSTD_ErrorCode::ZSTD_error_sequenceProducer_failed => {
            b"Block-level external sequence producer returned an error code\0"
        }
        ZSTD_ErrorCode::ZSTD_error_externalSequences_invalid => {
            b"External sequences are not valid\0"
        }
        ZSTD_ErrorCode::ZSTD_error_maxCode => b"Unspecified error code\0",
    }
}

pub(crate) fn error_string_ptr(code: ZSTD_ErrorCode) -> *const c_char {
    error_string(code).as_ptr().cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_isError(code: usize) -> u32 {
    is_error_result(code) as u32
}

#[no_mangle]
pub extern "C" fn ZSTD_getErrorName(code: usize) -> *const c_char {
    error_string_ptr(decode_error(code))
}

#[no_mangle]
pub extern "C" fn ZSTD_getErrorCode(functionResult: usize) -> ZSTD_ErrorCode {
    decode_error(functionResult)
}

#[no_mangle]
pub extern "C" fn ZSTD_getErrorString(code: ZSTD_ErrorCode) -> *const c_char {
    error_string_ptr(code)
}
