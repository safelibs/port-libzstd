use crate::ffi::{
    advanced::zdict_unknown_error_name,
    types::{ZDICT_legacy_params_t, ZDICT_params_t},
};
use core::ffi::{c_char, c_uint, c_void};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZDICT_addEntropyTablesFromBuffer"]
    fn internal_ZDICT_addEntropyTablesFromBuffer(
        dictBuffer: *mut c_void,
        dictContentSize: usize,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZDICT_finalizeDictionary"]
    fn internal_ZDICT_finalizeDictionary(
        dstDictBuffer: *mut c_void,
        maxDictSize: usize,
        dictContent: *const c_void,
        dictContentSize: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
        parameters: ZDICT_params_t,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZDICT_trainFromBuffer_legacy"]
    fn internal_ZDICT_trainFromBuffer_legacy(
        dictBuffer: *mut c_void,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
        parameters: ZDICT_legacy_params_t,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZDICT_getDictHeaderSize"]
    fn internal_ZDICT_getDictHeaderSize(dictBuffer: *const c_void, dictSize: usize) -> usize;
    #[link_name = "libzstd_safe_internal_ZDICT_getDictID"]
    fn internal_ZDICT_getDictID(dictBuffer: *const c_void, dictSize: usize) -> c_uint;
    #[link_name = "libzstd_safe_internal_ZDICT_trainFromBuffer"]
    fn internal_ZDICT_trainFromBuffer(
        dictBuffer: *mut c_void,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZDICT_isError"]
    fn internal_ZDICT_isError(errorCode: usize) -> c_uint;
    #[link_name = "libzstd_safe_internal_ZDICT_getErrorName"]
    fn internal_ZDICT_getErrorName(errorCode: usize) -> *const c_char;
}

#[no_mangle]
pub extern "C" fn ZDICT_addEntropyTablesFromBuffer(
    dictBuffer: *mut c_void,
    dictContentSize: usize,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZDICT_addEntropyTablesFromBuffer(
            dictBuffer,
            dictContentSize,
            dictBufferCapacity,
            samplesBuffer,
            samplesSizes,
            nbSamples,
        )
    }
}

#[no_mangle]
pub extern "C" fn ZDICT_finalizeDictionary(
    dstDictBuffer: *mut c_void,
    maxDictSize: usize,
    dictContent: *const c_void,
    dictContentSize: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: ZDICT_params_t,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZDICT_finalizeDictionary(
            dstDictBuffer,
            maxDictSize,
            dictContent,
            dictContentSize,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            parameters,
        )
    }
}

#[no_mangle]
pub extern "C" fn ZDICT_trainFromBuffer_legacy(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: ZDICT_legacy_params_t,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZDICT_trainFromBuffer_legacy(
            dictBuffer,
            dictBufferCapacity,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            parameters,
        )
    }
}

#[no_mangle]
pub extern "C" fn ZDICT_getDictHeaderSize(
    dictBuffer: *const c_void,
    dictSize: usize,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZDICT_getDictHeaderSize(dictBuffer, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZDICT_getDictID(
    dictBuffer: *const c_void,
    dictSize: usize,
) -> c_uint {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe { internal_ZDICT_getDictID(dictBuffer, dictSize) }
}

#[no_mangle]
pub extern "C" fn ZDICT_trainFromBuffer(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZDICT_trainFromBuffer(
            dictBuffer,
            dictBufferCapacity,
            samplesBuffer,
            samplesSizes,
            nbSamples,
        )
    }
}

#[no_mangle]
pub extern "C" fn ZDICT_isError(errorCode: usize) -> c_uint {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    unsafe { internal_ZDICT_isError(errorCode) }
}

#[no_mangle]
pub extern "C" fn ZDICT_getErrorName(errorCode: usize) -> *const c_char {
    // SAFETY: The linked helper uses the same ABI and takes the argument unchanged.
    let name = unsafe { internal_ZDICT_getErrorName(errorCode) };
    if name.is_null() {
        zdict_unknown_error_name()
    } else {
        name
    }
}
