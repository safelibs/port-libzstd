use crate::ffi::types::ZDICT_cover_params_t;
use core::ffi::{c_uint, c_void};

unsafe extern "C" {
    #[link_name = "libzstd_safe_internal_ZDICT_trainFromBuffer_cover"]
    fn internal_ZDICT_trainFromBuffer_cover(
        dictBuffer: *mut c_void,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
        parameters: ZDICT_cover_params_t,
    ) -> usize;
    #[link_name = "libzstd_safe_internal_ZDICT_optimizeTrainFromBuffer_cover"]
    fn internal_ZDICT_optimizeTrainFromBuffer_cover(
        dictBuffer: *mut c_void,
        dictBufferCapacity: usize,
        samplesBuffer: *const c_void,
        samplesSizes: *const usize,
        nbSamples: c_uint,
        parameters: *mut ZDICT_cover_params_t,
    ) -> usize;
}

#[no_mangle]
pub extern "C" fn ZDICT_trainFromBuffer_cover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: ZDICT_cover_params_t,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZDICT_trainFromBuffer_cover(
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
pub extern "C" fn ZDICT_optimizeTrainFromBuffer_cover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: *mut ZDICT_cover_params_t,
) -> usize {
    // SAFETY: The linked helper uses the same ABI and takes the arguments unchanged.
    unsafe {
        internal_ZDICT_optimizeTrainFromBuffer_cover(
            dictBuffer,
            dictBufferCapacity,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            parameters,
        )
    }
}
