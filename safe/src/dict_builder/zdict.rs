use crate::ffi::{
    advanced::{forward_extern, zdict_unknown_error_name},
    types::{ZDICT_legacy_params_t, ZDICT_params_t},
};
use core::ffi::{c_char, c_uint, c_void};

forward_extern! {
    pub fn ZDICT_addEntropyTablesFromBuffer(
        dictBuffer: *mut c_void,
        dictContentSize: usize,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
    ) -> usize => crate::ffi::compress::generic_error()
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
    type Fn = unsafe extern "C" fn(
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        *const usize,
        c_uint,
        ZDICT_params_t,
    ) -> usize;
    let Some(func) = crate::ffi::compress::load_upstream!("ZDICT_finalizeDictionary", Fn) else {
        return crate::ffi::compress::generic_error();
    };
    // SAFETY: The loaded symbol is cached with the exact signature declared above.
    unsafe {
        func(
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
    type Fn = unsafe extern "C" fn(
        *mut c_void,
        usize,
        *const c_void,
        *const usize,
        c_uint,
        ZDICT_legacy_params_t,
    ) -> usize;
    let Some(func) = crate::ffi::compress::load_upstream!("ZDICT_trainFromBuffer_legacy", Fn) else {
        return crate::ffi::compress::generic_error();
    };
    // SAFETY: The loaded symbol is cached with the exact signature declared above.
    unsafe { func(dictBuffer, dictBufferCapacity, samplesBuffer, samplesSizes, nbSamples, parameters) }
}

forward_extern! {
    pub fn ZDICT_getDictHeaderSize(
        dictBuffer: *const c_void,
        dictSize: usize,
    ) -> usize => crate::ffi::compress::generic_error()
}

forward_extern! {
    pub fn ZDICT_getDictID(
        dictBuffer: *const c_void,
        dictSize: usize,
    ) -> c_uint => 0
}

forward_extern! {
    pub fn ZDICT_trainFromBuffer(
        dictBuffer: *mut c_void,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
    ) -> usize => crate::ffi::compress::generic_error()
}

forward_extern! {
    pub fn ZDICT_isError(errorCode: usize) -> c_uint => 1
}

forward_extern! {
    pub fn ZDICT_getErrorName(errorCode: usize) -> *const c_char => zdict_unknown_error_name()
}
