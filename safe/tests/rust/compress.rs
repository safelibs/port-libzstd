use std::ffi::CStr;

use zstd::{
    compress::{block, cctx, cdict, cstream, params},
    decompress::{dctx, ddict},
    ffi::types::{
        ZSTD_CCtx, ZSTD_CDict, ZSTD_CStream, ZSTD_CONTENTSIZE_UNKNOWN,
        ZSTD_EndDirective, ZSTD_ResetDirective, ZSTD_bounds, ZSTD_cParameter,
        ZSTD_compressionParameters, ZSTD_dParameter, ZSTD_frameParameters,
        ZSTD_inBuffer, ZSTD_outBuffer, ZSTD_parameters,
    },
};

fn dict_fixture() -> Vec<u8> {
    sample_bytes(24 * 1024)
}

fn sample_bytes(size: usize) -> Vec<u8> {
    let fragments = [
        b"{\"tenant\":\"alpha\",\"region\":\"west\",\"kind\":\"session\",\"payload\":\"".as_slice(),
        b"{\"tenant\":\"beta\",\"region\":\"east\",\"kind\":\"metric\",\"payload\":\"".as_slice(),
        b"{\"tenant\":\"gamma\",\"region\":\"north\",\"kind\":\"record\",\"payload\":\"".as_slice(),
    ];
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    let mut out = Vec::with_capacity(size);
    let mut state = 0x1234_5678u32;
    while out.len() < size {
        let fragment = fragments[(state as usize) % fragments.len()];
        for &byte in fragment {
            if out.len() == size {
                break;
            }
            out.push(byte);
        }
        for _ in 0..96 {
            if out.len() == size {
                break;
            }
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            out.push(alphabet[(state as usize) % alphabet.len()]);
        }
        if out.len() < size {
            out.push(b'"');
        }
        if out.len() < size {
            out.push(b'}');
        }
        if out.len() < size {
            out.push(b'\n');
        }
    }
    out
}

fn dict_biased_sample(dict: &[u8], size: usize, seed: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(size);
    let mut cursor = (seed as usize) % dict.len();
    while out.len() < size {
        let mut chunk = 64 + ((seed as usize + out.len()) % 96);
        if chunk > dict.len() {
            chunk = dict.len();
        }
        if cursor + chunk > dict.len() {
            cursor = (cursor + 131 + ((seed as usize) % 29)) % dict.len();
            if cursor + chunk > dict.len() {
                chunk = dict.len() - cursor;
            }
        }
        let remaining = size - out.len();
        chunk = chunk.min(remaining);
        out.extend_from_slice(&dict[cursor..cursor + chunk]);
        if chunk > 12 {
            let pos = out.len() - chunk;
            out[pos + 3] ^= 0x11;
            out[pos + (chunk / 2)] ^= 0x5A;
        }
        if out.len() < size {
            out.push(b'\n');
        }
        cursor = (cursor + 97 + ((seed as usize) % 23)) % dict.len();
    }
    out
}

fn check_result(code: usize, what: &str) {
    assert_eq!(zstd::common::error::ZSTD_isError(code), 0, "{what}: {}", unsafe {
        CStr::from_ptr(zstd::common::error::ZSTD_getErrorName(code))
            .to_string_lossy()
            .into_owned()
    });
}

fn expect_error(code: usize, what: &str) {
    assert_eq!(zstd::common::error::ZSTD_isError(code), 1, "{what} unexpectedly succeeded");
}

fn decompress_exact(compressed: &[u8], expected: &[u8]) {
    let mut decoded = vec![0u8; expected.len()];
    let decoded_size = dctx::ZSTD_decompress(
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        compressed.as_ptr().cast(),
        compressed.len(),
    );
    check_result(decoded_size, "ZSTD_decompress");
    assert_eq!(decoded_size, expected.len());
    assert_eq!(decoded, expected);
}

fn decompress_using_dict_once(compressed: &[u8], dict: &[u8], expected: &[u8]) {
    let dctx_ptr = dctx::ZSTD_createDCtx();
    let mut decoded = vec![0u8; expected.len()];
    let decoded_size = dctx::ZSTD_decompress_usingDict(
        dctx_ptr,
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        compressed.as_ptr().cast(),
        compressed.len(),
        dict.as_ptr().cast(),
        dict.len(),
    );
    dctx::ZSTD_freeDCtx(dctx_ptr);
    check_result(decoded_size, "ZSTD_decompress_usingDict");
    assert_eq!(decoded, expected);
}

fn decompress_using_ddict_once(compressed: &[u8], ddict_ptr: *const zstd::ffi::types::ZSTD_DDict, expected: &[u8]) {
    let dctx_ptr = dctx::ZSTD_createDCtx();
    let mut decoded = vec![0u8; expected.len()];
    let decoded_size = dctx::ZSTD_decompress_usingDDict(
        dctx_ptr,
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        compressed.as_ptr().cast(),
        compressed.len(),
        ddict_ptr,
    );
    dctx::ZSTD_freeDCtx(dctx_ptr);
    check_result(decoded_size, "ZSTD_decompress_usingDDict");
    assert_eq!(decoded, expected);
}

#[test]
fn compress_one_shot_context_and_block_api_roundtrip() {
    let src = sample_bytes(256 * 1024 + 19);
    let bound = cctx::ZSTD_compressBound(src.len());
    let mut compressed = vec![0u8; bound];
    let mut second = vec![0u8; bound];
    let mut third = vec![0u8; bound];
    let cctx_ptr: *mut ZSTD_CCtx = cctx::ZSTD_createCCtx();
    let clone: *mut ZSTD_CCtx = cctx::ZSTD_createCCtx();
    let copy_src: *mut ZSTD_CCtx = cctx::ZSTD_createCCtx();
    let mut level = 0;

    assert!(bound >= src.len());

    let size = cctx::ZSTD_compress(
        compressed.as_mut_ptr().cast(),
        compressed.len(),
        src.as_ptr().cast(),
        src.len(),
        1,
    );
    check_result(size, "ZSTD_compress");
    decompress_exact(&compressed[..size], &src);

    assert!(!cctx_ptr.is_null());
    assert!(!clone.is_null());
    assert!(cctx::ZSTD_sizeof_CCtx(cctx_ptr) > 0);
    check_result(
        cctx::ZSTD_CCtx_setParameter(
            cctx_ptr,
            ZSTD_cParameter::ZSTD_c_compressionLevel,
            5,
        ),
        "ZSTD_CCtx_setParameter(level)",
    );
    check_result(
        cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_checksumFlag, 1),
        "ZSTD_CCtx_setParameter(checksum)",
    );
    check_result(
        cctx::ZSTD_CCtx_getParameter(
            cctx_ptr,
            ZSTD_cParameter::ZSTD_c_compressionLevel,
            &mut level,
        ),
        "ZSTD_CCtx_getParameter(level)",
    );
    assert_eq!(level, 5);
    check_result(
        cctx::ZSTD_CCtx_setPledgedSrcSize(cctx_ptr, src.len() as u64),
        "ZSTD_CCtx_setPledgedSrcSize",
    );
    check_result(
        cctx::ZSTD_compressBegin(copy_src, 1),
        "ZSTD_compressBegin(copy_src)",
    );
    check_result(
        cctx::ZSTD_copyCCtx(clone, copy_src, ZSTD_CONTENTSIZE_UNKNOWN),
        "ZSTD_copyCCtx",
    );

    let size = cctx::ZSTD_compress2(
        cctx_ptr,
        second.as_mut_ptr().cast(),
        second.len(),
        src.as_ptr().cast(),
        src.len(),
    );
    check_result(size, "ZSTD_compress2");
    decompress_exact(&second[..size], &src);

    let size = cctx::ZSTD_compress2(
        clone,
        third.as_mut_ptr().cast(),
        third.len(),
        src.as_ptr().cast(),
        src.len(),
    );
    check_result(size, "ZSTD_compress2(clone)");
    decompress_exact(&third[..size], &src);

    let size = cctx::ZSTD_compressCCtx(
        cctx_ptr,
        second.as_mut_ptr().cast(),
        second.len(),
        src.as_ptr().cast(),
        src.len(),
        3,
    );
    check_result(size, "ZSTD_compressCCtx");
    decompress_exact(&second[..size], &src);

    {
        let block_src = vec![b'A'; 32 * 1024];
        let mut block_compressed = vec![0u8; cctx::ZSTD_compressBound(block_src.len())];

        check_result(cctx::ZSTD_compressBegin(cctx_ptr, 1), "ZSTD_compressBegin");
        assert!(block::ZSTD_getBlockSize(cctx_ptr) >= block_src.len());
        let block_size = block::ZSTD_compressBlock(
            cctx_ptr,
            block_compressed.as_mut_ptr().cast(),
            block_compressed.len(),
            block_src.as_ptr().cast(),
            block_src.len(),
        );
        check_result(block_size, "ZSTD_compressBlock");
        assert!(block_size > 0);
        assert!(block_size < block_compressed.len());
    }

    cctx::ZSTD_freeCCtx(clone);
    cctx::ZSTD_freeCCtx(copy_src);
    cctx::ZSTD_freeCCtx(cctx_ptr);
}

#[test]
fn compress_dictionary_and_prefix_helpers_roundtrip() {
    let dict = dict_fixture();
    let src = dict_biased_sample(&dict, 64 * 1024 + 131, 0x12345);
    let dict_id = ddict::ZSTD_getDictID_fromDict(dict.as_ptr().cast(), dict.len());
    let cdict_ptr: *mut ZSTD_CDict = cdict::ZSTD_createCDict(dict.as_ptr().cast(), dict.len(), 5);
    let ddict_ptr = ddict::ZSTD_createDDict(dict.as_ptr().cast(), dict.len());
    let cctx_ptr = cctx::ZSTD_createCCtx();
    let dctx_ptr = dctx::ZSTD_createDCtx();
    let bound = cctx::ZSTD_compressBound(src.len());
    let mut compressed = vec![0u8; bound];
    let mut second = vec![0u8; bound];
    let mut decoded = vec![0u8; src.len().max(dict.len() * 4)];

    assert!(!cdict_ptr.is_null());
    assert!(!ddict_ptr.is_null());
    assert_eq!(cdict::ZSTD_getDictID_fromCDict(cdict_ptr), dict_id);

    let size = cdict::ZSTD_compress_usingCDict(
        cctx_ptr,
        compressed.as_mut_ptr().cast(),
        compressed.len(),
        src.as_ptr().cast(),
        src.len(),
        cdict_ptr,
    );
    check_result(size, "ZSTD_compress_usingCDict");
    assert_eq!(
        zstd::common::frame::ZSTD_getDictID_fromFrame(compressed.as_ptr().cast(), size),
        dict_id
    );
    expect_error(
        dctx::ZSTD_decompress(
            decoded.as_mut_ptr().cast(),
            decoded.len(),
            compressed.as_ptr().cast(),
            size,
        ),
        "ZSTD_decompress(dictionary frame without dict)",
    );

    let decoded_size = dctx::ZSTD_decompress_usingDDict(
        dctx_ptr,
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        compressed.as_ptr().cast(),
        size,
        ddict_ptr,
    );
    check_result(decoded_size, "ZSTD_decompress_usingDDict");
    assert_eq!(&decoded[..decoded_size], src.as_slice());

    let decoded_size = dctx::ZSTD_decompress_usingDict(
        dctx_ptr,
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        compressed.as_ptr().cast(),
        size,
        dict.as_ptr().cast(),
        dict.len(),
    );
    check_result(decoded_size, "ZSTD_decompress_usingDict");
    assert_eq!(&decoded[..decoded_size], src.as_slice());

    let size = cctx::ZSTD_compress_usingDict(
        cctx_ptr,
        second.as_mut_ptr().cast(),
        second.len(),
        src.as_ptr().cast(),
        src.len(),
        dict.as_ptr().cast(),
        dict.len(),
        5,
    );
    check_result(size, "ZSTD_compress_usingDict");
    let decoded_size = dctx::ZSTD_decompress_usingDDict(
        dctx_ptr,
        decoded.as_mut_ptr().cast(),
        decoded.len(),
        second.as_ptr().cast(),
        size,
        ddict_ptr,
    );
    check_result(decoded_size, "ZSTD_decompress_usingDDict(raw)");
    assert_eq!(&decoded[..decoded_size], src.as_slice());

    {
        let prefix = &dict[dict.len().saturating_sub(32 * 1024)..];
        let prefix_src = {
            let mut bytes = prefix.to_vec();
            bytes.extend_from_slice(prefix);
            bytes.extend_from_slice(prefix);
            bytes
        };
        let mut plain = vec![0u8; cctx::ZSTD_compressBound(prefix_src.len())];
        let mut prefixed = vec![0u8; cctx::ZSTD_compressBound(prefix_src.len())];

        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(prefix/plain)",
        );
        let plain_size = cctx::ZSTD_compress2(
            cctx_ptr,
            plain.as_mut_ptr().cast(),
            plain.len(),
            prefix_src.as_ptr().cast(),
            prefix_src.len(),
        );
        check_result(plain_size, "ZSTD_compress2(plain)");

        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(prefix/ref)",
        );
        check_result(
            cctx::ZSTD_CCtx_refPrefix(cctx_ptr, prefix.as_ptr().cast(), prefix.len()),
            "ZSTD_CCtx_refPrefix",
        );
        let prefixed_size = cctx::ZSTD_compress2(
            cctx_ptr,
            prefixed.as_mut_ptr().cast(),
            prefixed.len(),
            prefix_src.as_ptr().cast(),
            prefix_src.len(),
        );
        check_result(prefixed_size, "ZSTD_compress2(prefixed)");
        assert!(prefixed_size < plain_size);

        check_result(
            dctx::ZSTD_DCtx_reset(
                dctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_DCtx_reset(prefix)",
        );
        check_result(
            dctx::ZSTD_DCtx_refPrefix(dctx_ptr, prefix.as_ptr().cast(), prefix.len()),
            "ZSTD_DCtx_refPrefix",
        );
        let decoded_size = dctx::ZSTD_decompressDCtx(
            dctx_ptr,
            decoded.as_mut_ptr().cast(),
            prefix_src.len(),
            prefixed.as_ptr().cast(),
            prefixed_size,
        );
        check_result(decoded_size, "ZSTD_decompressDCtx(prefix)");
        assert_eq!(&decoded[..decoded_size], prefix_src.as_slice());
    }

    cdict::ZSTD_freeCDict(cdict_ptr);
    ddict::ZSTD_freeDDict(ddict_ptr);
    cctx::ZSTD_freeCCtx(cctx_ptr);
    dctx::ZSTD_freeDCtx(dctx_ptr);
}

fn compress_stream_legacy(zcs: *mut ZSTD_CStream, src: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    let mut input = ZSTD_inBuffer {
        src: src.as_ptr().cast(),
        size: src.len(),
        pos: 0,
    };

    while input.pos < input.size {
        let mut out_buf = vec![0u8; cstream::ZSTD_CStreamOutSize()];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream(zcs, &mut output, &mut input);
        check_result(remaining, "ZSTD_compressStream");
        compressed.extend_from_slice(&out_buf[..output.pos]);
    }

    loop {
        let mut out_buf = vec![0u8; cstream::ZSTD_CStreamOutSize()];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_endStream(zcs, &mut output);
        check_result(remaining, "ZSTD_endStream");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    compressed
}

fn compress_stream2_end(cctx: *mut ZSTD_CCtx, src: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    let mut input = ZSTD_inBuffer {
        src: src.as_ptr().cast(),
        size: src.len(),
        pos: 0,
    };

    loop {
        let mut out_buf = vec![0u8; cstream::ZSTD_CStreamOutSize()];
        let mut output = ZSTD_outBuffer {
            dst: out_buf.as_mut_ptr().cast(),
            size: out_buf.len(),
            pos: 0,
        };
        let remaining = cstream::ZSTD_compressStream2(
            cctx,
            &mut output,
            &mut input,
            ZSTD_EndDirective::ZSTD_e_end,
        );
        check_result(remaining, "ZSTD_compressStream2");
        compressed.extend_from_slice(&out_buf[..output.pos]);
        if remaining == 0 {
            break;
        }
    }

    compressed
}

#[test]
fn compress_streaming_and_parameter_helpers_roundtrip() {
    let src = sample_bytes(192 * 1024 + 37);
    let dict = dict_fixture();
    let cdict_ptr = cdict::ZSTD_createCDict(dict.as_ptr().cast(), dict.len(), 4);
    let bounds: ZSTD_bounds = params::ZSTD_cParam_getBounds(ZSTD_cParameter::ZSTD_c_strategy);
    let window_bounds = params::ZSTD_dParam_getBounds(ZSTD_dParameter::ZSTD_d_windowLogMax);
    let cparams: ZSTD_compressionParameters =
        params::ZSTD_getCParams(4, src.len() as u64, dict.len());
    let adjusted = params::ZSTD_adjustCParams(cparams, src.len() as u64, dict.len());
    let full_params: ZSTD_parameters = params::ZSTD_getParams(4, src.len() as u64, dict.len());
    let zcs = cstream::ZSTD_createCStream();
    let zcs2 = cstream::ZSTD_createCStream();

    assert_eq!(bounds.error, 0);
    assert_eq!(window_bounds.error, 0);
    check_result(params::ZSTD_checkCParams(cparams), "ZSTD_checkCParams");
    check_result(params::ZSTD_checkCParams(adjusted), "ZSTD_checkCParams(adjusted)");
    assert!(params::ZSTD_minCLevel() <= params::ZSTD_defaultCLevel());
    assert!(params::ZSTD_defaultCLevel() <= params::ZSTD_maxCLevel());
    assert!(!cdict_ptr.is_null());
    assert!(cstream::ZSTD_CStreamInSize() > 0);
    assert!(cstream::ZSTD_CStreamOutSize() > 0);
    assert!(cstream::ZSTD_sizeof_CStream(zcs) > 0);

    check_result(cstream::ZSTD_initCStream(zcs, 3), "ZSTD_initCStream");
    let compressed = compress_stream_legacy(zcs, &src);
    decompress_exact(&compressed, &src);

    check_result(
        cstream::ZSTD_initCStream_srcSize(zcs, 4, src.len() as u64),
        "ZSTD_initCStream_srcSize",
    );
    let compressed = compress_stream_legacy(zcs, &src);
    decompress_exact(&compressed, &src);

    check_result(
        cstream::ZSTD_resetCStream(zcs, ZSTD_CONTENTSIZE_UNKNOWN),
        "ZSTD_resetCStream",
    );
    let compressed = compress_stream_legacy(zcs, &src[..64 * 1024]);
    decompress_exact(&compressed, &src[..64 * 1024]);

    check_result(
        cstream::ZSTD_initCStream_usingDict(zcs, dict.as_ptr().cast(), dict.len(), 4),
        "ZSTD_initCStream_usingDict",
    );
    let compressed = compress_stream_legacy(zcs, &src);
    decompress_using_dict_once(&compressed, &dict, &src);

    check_result(
        cdict::ZSTD_initCStream_usingCDict(zcs2, cdict_ptr),
        "ZSTD_initCStream_usingCDict",
    );
    let compressed = compress_stream_legacy(zcs2, &src);
    let ddict_ptr = ddict::ZSTD_createDDict(dict.as_ptr().cast(), dict.len());
    decompress_using_ddict_once(&compressed, ddict_ptr, &src);

    check_result(
        cstream::ZSTD_initCStream_advanced(
            zcs2,
            core::ptr::null(),
            0,
            ZSTD_parameters {
                cParams: adjusted,
                fParams: ZSTD_frameParameters {
                    contentSizeFlag: full_params.fParams.contentSizeFlag,
                    checksumFlag: 1,
                    noDictIDFlag: 1,
                },
            },
            src.len() as u64,
        ),
        "ZSTD_initCStream_advanced",
    );
    let compressed = compress_stream_legacy(zcs2, &src);
    decompress_exact(&compressed, &src);

    check_result(
        cdict::ZSTD_initCStream_usingCDict_advanced(
            zcs2,
            cdict_ptr,
            ZSTD_frameParameters {
                contentSizeFlag: 1,
                checksumFlag: 1,
                noDictIDFlag: 0,
            },
            src.len() as u64,
        ),
        "ZSTD_initCStream_usingCDict_advanced",
    );
    let compressed = compress_stream_legacy(zcs2, &src);
    decompress_using_ddict_once(&compressed, ddict_ptr, &src);

    {
        let cctx_ptr = cctx::ZSTD_createCCtx();
        check_result(
            cctx::ZSTD_CCtx_reset(
                cctx_ptr,
                ZSTD_ResetDirective::ZSTD_reset_session_and_parameters,
            ),
            "ZSTD_CCtx_reset(stream2)",
        );
        check_result(
            cctx::ZSTD_CCtx_setParameter(cctx_ptr, ZSTD_cParameter::ZSTD_c_checksumFlag, 1),
            "ZSTD_CCtx_setParameter(checksum stream2)",
        );
        let compressed = compress_stream2_end(cctx_ptr, &src);
        decompress_exact(&compressed, &src);
        cctx::ZSTD_freeCCtx(cctx_ptr);
    }

    ddict::ZSTD_freeDDict(ddict_ptr);
    cdict::ZSTD_freeCDict(cdict_ptr);
    cstream::ZSTD_freeCStream(zcs2);
    cstream::ZSTD_freeCStream(zcs);
}
