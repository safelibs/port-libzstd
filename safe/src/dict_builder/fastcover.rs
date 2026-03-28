use crate::ffi::{
    advanced::forward_extern,
    types::ZDICT_fastCover_params_t,
};
use core::ffi::{c_uint, c_void};

#[no_mangle]
pub extern "C" fn ZDICT_trainFromBuffer_fastCover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: ZDICT_fastCover_params_t,
) -> usize {
    type Fn = unsafe extern "C" fn(
        *mut c_void,
        usize,
        *const c_void,
        *const usize,
        c_uint,
        ZDICT_fastCover_params_t,
    ) -> usize;
    let Some(func) = crate::ffi::compress::load_upstream!("ZDICT_trainFromBuffer_fastCover", Fn) else {
        return crate::ffi::compress::generic_error();
    };
    // SAFETY: The loaded symbol is cached with the exact signature declared above.
    unsafe { func(dictBuffer, dictBufferCapacity, samplesBuffer, samplesSizes, nbSamples, parameters) }
}

forward_extern! {
    pub fn ZDICT_optimizeTrainFromBuffer_fastCover(
        dictBuffer: *mut c_void,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
        parameters: *mut ZDICT_fastCover_params_t,
    ) -> usize => crate::ffi::compress::generic_error()
}
