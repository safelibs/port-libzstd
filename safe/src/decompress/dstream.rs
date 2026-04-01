use crate::{
    common::error::error_result,
    decompress::{
        block::{BLOCK_HEADER_SIZE, BLOCK_SIZE_MAX},
        frame,
    },
    ffi::{
        decompress,
        types::{
            ZSTD_DCtx, ZSTD_DStream, ZSTD_ErrorCode, ZSTD_customMem, ZSTD_format_e, ZSTD_inBuffer,
            ZSTD_nextInputType_e, ZSTD_outBuffer,
        },
    },
};
use core::ffi::c_void;

fn estimate_dstream_size(window_size: usize) -> Result<usize, ZSTD_ErrorCode> {
    let block_size = window_size.min(BLOCK_SIZE_MAX);
    let out_buffer_size =
        decompress::decoding_buffer_size_min(window_size as u64, frame::ZSTD_CONTENTSIZE_UNKNOWN)?;
    crate::decompress::dctx::ZSTD_estimateDCtxSize()
        .checked_add(block_size)
        .and_then(|size| size.checked_add(out_buffer_size))
        .ok_or(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)
}

fn custom_mem_supported(custom_mem: ZSTD_customMem) -> bool {
    custom_mem.customAlloc.is_none() && custom_mem.customFree.is_none()
}

#[no_mangle]
pub extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DStream {
    decompress::create_dctx().cast()
}

#[no_mangle]
pub extern "C" fn ZSTD_freeDStream(zds: *mut ZSTD_DStream) -> usize {
    decompress::free_dctx(zds.cast())
}

#[no_mangle]
pub extern "C" fn ZSTD_initDStream(zds: *mut ZSTD_DStream) -> usize {
    match decompress::with_dctx_mut(zds, |zds| {
        zds.reset_session();
        zds.ref_ddict(core::ptr::null())?;
        Ok(frame::starting_input_length(zds.format))
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_resetDStream(zds: *mut ZSTD_DStream) -> usize {
    match decompress::with_dctx_mut(zds, |zds| {
        zds.reset_session();
        Ok(frame::starting_input_length(zds.format))
    }) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressStream(
    zds: *mut ZSTD_DStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    if output.is_null() || input.is_null() {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    // SAFETY: The caller provided valid `ZSTD_inBuffer`/`ZSTD_outBuffer` pointers.
    let (output, input) = unsafe { (&mut *output, &mut *input) };
    match decompress::with_dctx_mut(zds, |zds| decompress::stream_decompress(zds, output, input)) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_DStreamInSize() -> usize {
    BLOCK_SIZE_MAX + BLOCK_HEADER_SIZE
}

#[no_mangle]
pub extern "C" fn ZSTD_DStreamOutSize() -> usize {
    BLOCK_SIZE_MAX
}

#[no_mangle]
pub extern "C" fn ZSTD_nextSrcSizeToDecompress(dctx: *mut ZSTD_DCtx) -> usize {
    decompress::next_src_size_to_decompress(dctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_nextInputType(dctx: *mut ZSTD_DCtx) -> ZSTD_nextInputType_e {
    decompress::next_input_type(dctx)
}

#[no_mangle]
pub extern "C" fn ZSTD_decodingBufferSize_min(windowSize: u64, frameContentSize: u64) -> usize {
    match decompress::decoding_buffer_size_min(windowSize, frameContentSize) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_sizeof_DStream(zds: *const ZSTD_DStream) -> usize {
    decompress::sizeof_dctx(zds.cast())
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDStreamSize_fromFrame(src: *const c_void, srcSize: usize) -> usize {
    let Some(src) = decompress::optional_src_slice(src, srcSize) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_srcBuffer_wrong);
    };

    let header = match frame::parse_frame_header(src, ZSTD_format_e::ZSTD_f_zstd1) {
        Ok(frame::HeaderProbe::Need(_)) => {
            return error_result(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
        }
        Ok(frame::HeaderProbe::Header(header)) => header,
        Err(code) => return error_result(code),
    };

    let window_size_max = 1_u64 << frame::ZSTD_WINDOWLOG_MAX;
    if header.windowSize > window_size_max {
        return error_result(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge);
    }

    match usize::try_from(header.windowSize)
        .map_err(|_| ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)
        .and_then(estimate_dstream_size)
    {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_estimateDStreamSize(windowSize: usize) -> usize {
    match estimate_dstream_size(windowSize) {
        Ok(size) => size,
        Err(code) => error_result(code),
    }
}

#[no_mangle]
pub extern "C" fn ZSTD_decompressStream_simpleArgs(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    dstPos: *mut usize,
    src: *const c_void,
    srcSize: usize,
    srcPos: *mut usize,
) -> usize {
    if dstPos.is_null() || srcPos.is_null() {
        return error_result(crate::ffi::types::ZSTD_ErrorCode::ZSTD_error_GENERIC);
    }

    let mut output = crate::ffi::types::ZSTD_outBuffer {
        dst,
        size: dstCapacity,
        // SAFETY: The caller provided a valid `dstPos` pointer.
        pos: unsafe { *dstPos },
    };
    let mut input = crate::ffi::types::ZSTD_inBuffer {
        src,
        size: srcSize,
        // SAFETY: The caller provided a valid `srcPos` pointer.
        pos: unsafe { *srcPos },
    };
    let ret = ZSTD_decompressStream(dctx, &mut output, &mut input);
    // SAFETY: The caller provided valid `dstPos` and `srcPos` pointers.
    unsafe {
        *dstPos = output.pos;
        *srcPos = input.pos;
    }
    ret
}

#[no_mangle]
pub extern "C" fn ZSTD_createDStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DStream {
    if !custom_mem_supported(customMem) {
        return core::ptr::null_mut();
    }
    decompress::create_dctx().cast()
}
