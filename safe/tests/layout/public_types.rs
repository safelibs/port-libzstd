#![allow(non_camel_case_types)]

use std::mem::{align_of, size_of};

use zstd::ffi::types::{
    ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CDict, ZSTD_DCtx, ZSTD_DDict, ZSTD_Sequence,
    ZSTD_bounds, ZSTD_customMem, ZSTD_frameHeader, ZSTD_frameProgression, ZSTD_inBuffer,
    ZSTD_outBuffer, ZSTD_threadPool,
};

fn emit<T>(name: &str) {
    println!("{name} size={} align={}", size_of::<T>(), align_of::<T>());
}

fn main() {
    emit::<ZSTD_inBuffer>("ZSTD_inBuffer");
    emit::<ZSTD_outBuffer>("ZSTD_outBuffer");
    emit::<ZSTD_customMem>("ZSTD_customMem");
    emit::<ZSTD_frameHeader>("ZSTD_frameHeader");
    emit::<ZSTD_Sequence>("ZSTD_Sequence");
    emit::<ZSTD_bounds>("ZSTD_bounds");
    emit::<ZSTD_frameProgression>("ZSTD_frameProgression");

    emit::<*mut ZSTD_CCtx>("ZSTD_CCtx*");
    emit::<*mut ZSTD_DCtx>("ZSTD_DCtx*");
    emit::<*mut ZSTD_CDict>("ZSTD_CDict*");
    emit::<*mut ZSTD_DDict>("ZSTD_DDict*");
    emit::<*mut ZSTD_CCtx_params>("ZSTD_CCtx_params*");
    emit::<*mut ZSTD_threadPool>("ZSTD_threadPool*");
}
