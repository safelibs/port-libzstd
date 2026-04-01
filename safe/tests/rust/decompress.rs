use std::{
    ffi::CStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use zstd::{
    common::{frame, skippable, version},
    decompress::{dctx, ddict, dstream, legacy},
    ffi::types::{
        ZSTD_DCtx, ZSTD_DDict, ZSTD_DStream, ZSTD_ErrorCode, ZSTD_ResetDirective, ZSTD_dParameter,
        ZSTD_format_e, ZSTD_frameHeader, ZSTD_inBuffer, ZSTD_nextInputType_e, ZSTD_outBuffer,
    },
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("safe crate lives under repo root")
        .to_path_buf()
}

fn golden_file(name: &str) -> Vec<u8> {
    fs::read(
        repo_root()
            .join("original/libzstd-1.5.5+dfsg2/tests/golden-decompression")
            .join(name),
    )
    .expect("read golden frame")
}

fn check_result(code: usize, what: &str) {
    assert_eq!(
        zstd::common::error::ZSTD_isError(code),
        0,
        "{what}: {}",
        unsafe {
            CStr::from_ptr(zstd::common::error::ZSTD_getErrorName(code))
                .to_string_lossy()
                .into_owned()
        }
    );
}

fn expect_error(code: usize, expected: ZSTD_ErrorCode, what: &str) {
    assert_eq!(
        zstd::common::error::ZSTD_isError(code),
        1,
        "{what} unexpectedly succeeded"
    );
    assert_eq!(
        zstd::common::error::ZSTD_getErrorCode(code),
        expected,
        "{what}: {}",
        unsafe {
            CStr::from_ptr(zstd::common::error::ZSTD_getErrorName(code))
                .to_string_lossy()
                .into_owned()
        }
    );
}

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("libzstd-safe-{label}-{stamp}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn run_zstd(args: &[&str]) {
    let status = Command::new("zstd").args(args).status().expect("run zstd");
    assert!(status.success(), "zstd {:?} failed with {status}", args);
}

fn small_single_segment_frame() -> (Vec<u8>, Vec<u8>) {
    let dir = temp_dir("small-single-segment");
    let input = dir.join("tiny.bin");
    let compressed = dir.join("tiny.zst");
    let payload = vec![0u8; 31];

    fs::write(&input, &payload).expect("write tiny input");
    run_zstd(&[
        "-q",
        "-f",
        input.to_str().expect("utf8 path"),
        "-o",
        compressed.to_str().expect("utf8 path"),
    ]);

    let compressed_bytes = fs::read(&compressed).expect("read tiny frame");
    assert!(
        compressed_bytes.len() >= 5,
        "tiny frame should include a full modern header"
    );
    assert_ne!(
        compressed_bytes[4] & 0x20,
        0,
        "expected a single-segment frame header"
    );

    (payload, compressed_bytes)
}

#[test]
fn decompress_modern_frames_and_contexts() {
    let modern = golden_file("rle-first-block.zst");
    let empty = golden_file("empty-block.zst");
    let mut window = 0;
    let mut output = vec![0u8; 1024 * 1024];
    let dctx: *mut ZSTD_DCtx = dctx::ZSTD_createDCtx();
    let clone: *mut ZSTD_DCtx = dctx::ZSTD_createDCtx();

    assert_eq!(version::ZSTD_versionNumber(), 10505);
    let version_string = unsafe { CStr::from_ptr(version::ZSTD_versionString()) }
        .to_str()
        .expect("utf8 version");
    assert_eq!(version_string, "1.5.5");
    assert_eq!(legacy::ZSTD_LEGACY_SUPPORT, 5);

    assert_eq!(
        frame::ZSTD_getFrameContentSize(modern.as_ptr().cast(), modern.len()),
        1024_u64 * 1024
    );
    assert_eq!(
        frame::ZSTD_getDecompressedSize(modern.as_ptr().cast(), modern.len()),
        1024_u64 * 1024
    );
    assert_eq!(
        frame::ZSTD_findFrameCompressedSize(modern.as_ptr().cast(), modern.len()),
        modern.len()
    );
    assert_eq!(
        frame::ZSTD_findDecompressedSize(modern.as_ptr().cast(), modern.len()),
        1024_u64 * 1024
    );
    assert_eq!(
        frame::ZSTD_decompressBound(modern.as_ptr().cast(), modern.len()),
        1024_u64 * 1024
    );
    assert_eq!(
        frame::ZSTD_getFrameContentSize(empty.as_ptr().cast(), empty.len()),
        0
    );

    let one_shot = dctx::ZSTD_decompress(
        output.as_mut_ptr().cast(),
        output.len(),
        modern.as_ptr().cast(),
        modern.len(),
    );
    assert_eq!(one_shot, output.len());
    assert!(output.iter().all(|&byte| byte == 0));

    assert!(!dctx.is_null());
    assert!(!clone.is_null());
    assert!(dctx::ZSTD_sizeof_DCtx(dctx) > 0);

    check_result(
        dctx::ZSTD_DCtx_setParameter(dctx, ZSTD_dParameter::ZSTD_d_windowLogMax, 23),
        "ZSTD_DCtx_setParameter",
    );
    check_result(
        dctx::ZSTD_DCtx_getParameter(dctx, ZSTD_dParameter::ZSTD_d_windowLogMax, &mut window),
        "ZSTD_DCtx_getParameter",
    );
    assert_eq!(window, 23);
    check_result(
        dctx::ZSTD_DCtx_setFormat(dctx, ZSTD_format_e::ZSTD_f_zstd1),
        "ZSTD_DCtx_setFormat",
    );
    check_result(
        dctx::ZSTD_DCtx_setMaxWindowSize(dctx, 1 << 23),
        "ZSTD_DCtx_setMaxWindowSize",
    );
    check_result(
        dctx::ZSTD_DCtx_refPrefix(dctx, std::ptr::null(), 0),
        "ZSTD_DCtx_refPrefix",
    );

    dctx::ZSTD_copyDCtx(clone, dctx);
    check_result(
        dctx::ZSTD_DCtx_getParameter(clone, ZSTD_dParameter::ZSTD_d_windowLogMax, &mut window),
        "ZSTD_copyDCtx",
    );
    let cloned = dctx::ZSTD_decompressDCtx(
        clone,
        output.as_mut_ptr().cast(),
        output.len(),
        modern.as_ptr().cast(),
        modern.len(),
    );
    assert_eq!(cloned, output.len());
    assert!(output.iter().all(|&byte| byte == 0));

    let result = dctx::ZSTD_decompressDCtx(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        modern.as_ptr().cast(),
        modern.len(),
    );
    assert_eq!(result, output.len());
    assert!(output.iter().all(|&byte| byte == 0));

    check_result(
        ddict::ZSTD_DCtx_loadDictionary(dctx, std::ptr::null(), 0),
        "ZSTD_DCtx_loadDictionary",
    );
    let using_dict = dctx::ZSTD_decompress_usingDict(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        modern.as_ptr().cast(),
        modern.len(),
        std::ptr::null(),
        0,
    );
    assert_eq!(using_dict, output.len());
    assert!(output.iter().all(|&byte| byte == 0));

    check_result(
        dctx::ZSTD_DCtx_reset(dctx, ZSTD_ResetDirective::ZSTD_reset_session_and_parameters),
        "ZSTD_DCtx_reset",
    );

    let header_size = frame::ZSTD_frameHeaderSize(modern.as_ptr().cast(), modern.len());
    let mut block_output = vec![0u8; output.len()];
    let mut block_offset = header_size + 3;
    let mut produced = 0usize;
    check_result(
        dctx::ZSTD_decompressBegin(dctx),
        "ZSTD_decompressBegin(block)",
    );
    assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx), 5);
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            block_output.as_mut_ptr().cast(),
            block_output.len(),
            modern[..5].as_ptr().cast(),
            5,
        ),
        0
    );
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            block_output.as_mut_ptr().cast(),
            block_output.len(),
            modern[5..header_size].as_ptr().cast(),
            header_size - 5,
        ),
        0
    );
    assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx), 3);
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            block_output.as_mut_ptr().cast(),
            block_output.len(),
            modern[header_size..header_size + 3].as_ptr().cast(),
            3,
        ),
        0
    );
    let block_size = dstream::ZSTD_nextSrcSizeToDecompress(dctx);
    let block_written = zstd::decompress::block::ZSTD_decompressBlock(
        dctx,
        block_output.as_mut_ptr().cast(),
        block_output.len(),
        modern[block_offset..block_offset + block_size]
            .as_ptr()
            .cast(),
        block_size,
    );
    check_result(block_written, "ZSTD_decompressBlock");
    assert!(block_written > 0);
    assert!(block_output[..block_written].iter().all(|&byte| byte == 0));
    block_offset += block_size;
    produced += block_written;

    while dstream::ZSTD_nextSrcSizeToDecompress(dctx) != 0 {
        let need = dstream::ZSTD_nextSrcSizeToDecompress(dctx);
        let wrote = dctx::ZSTD_decompressContinue(
            dctx,
            block_output[produced..].as_mut_ptr().cast(),
            block_output.len() - produced,
            modern[block_offset..block_offset + need].as_ptr().cast(),
            need,
        );
        check_result(wrote, "ZSTD_decompressContinue(block-finish)");
        produced += wrote;
        block_offset += need;
    }
    assert_eq!(produced, output.len());
    assert_eq!(block_offset, modern.len());
    assert!(block_output.iter().all(|&byte| byte == 0));

    let mut corrupt_frame = modern.clone();
    corrupt_frame[header_size] |= 0x06;
    check_result(
        dctx::ZSTD_decompressBegin(dctx),
        "ZSTD_decompressBegin(corrupt frame)",
    );
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            block_output.as_mut_ptr().cast(),
            block_output.len(),
            corrupt_frame[..5].as_ptr().cast(),
            5,
        ),
        0
    );
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            block_output.as_mut_ptr().cast(),
            block_output.len(),
            corrupt_frame[5..header_size].as_ptr().cast(),
            header_size - 5,
        ),
        0
    );
    let corrupt_result = dctx::ZSTD_decompressContinue(
        dctx,
        block_output.as_mut_ptr().cast(),
        block_output.len(),
        corrupt_frame[header_size..header_size + 3].as_ptr().cast(),
        3,
    );
    expect_error(
        corrupt_result,
        ZSTD_ErrorCode::ZSTD_error_corruption_detected,
        "ZSTD_decompressContinue(corrupt frame)",
    );

    ddict::ZSTD_freeDDict(std::ptr::null_mut::<ZSTD_DDict>());
    dstream::ZSTD_freeDStream(std::ptr::null_mut::<ZSTD_DStream>());
    dctx::ZSTD_freeDCtx(clone);
    dctx::ZSTD_freeDCtx(dctx);
}

#[test]
fn decompress_streaming_and_skippable_helpers() {
    let modern = golden_file("rle-first-block.zst");
    let zds = dstream::ZSTD_createDStream();
    let mut output = vec![0u8; 1024 * 1024];
    let mut produced = 0usize;
    let mut offset = 0usize;

    assert!(!zds.is_null());
    assert!(dstream::ZSTD_DStreamInSize() > 0);
    assert!(dstream::ZSTD_DStreamOutSize() > 0);
    assert!(dstream::ZSTD_sizeof_DStream(zds) > 0);
    let window_size = 1usize << 20;
    let expected_estimate = dctx::ZSTD_estimateDCtxSize()
        + window_size.min(dstream::ZSTD_DStreamOutSize())
        + dstream::ZSTD_decodingBufferSize_min(window_size as u64, u64::MAX);
    assert_eq!(
        dstream::ZSTD_estimateDStreamSize(window_size),
        expected_estimate
    );

    let mut header = ZSTD_frameHeader::default();
    check_result(
        frame::ZSTD_getFrameHeader(&mut header, modern.as_ptr().cast(), modern.len()),
        "ZSTD_getFrameHeader",
    );
    let expected_from_frame = dctx::ZSTD_estimateDCtxSize()
        + (header.windowSize as usize).min(dstream::ZSTD_DStreamOutSize())
        + dstream::ZSTD_decodingBufferSize_min(header.windowSize, u64::MAX);
    assert_eq!(
        dstream::ZSTD_estimateDStreamSize_fromFrame(modern.as_ptr().cast(), modern.len()),
        expected_from_frame
    );
    expect_error(
        dstream::ZSTD_estimateDStreamSize_fromFrame(modern.as_ptr().cast(), 3),
        ZSTD_ErrorCode::ZSTD_error_srcSize_wrong,
        "ZSTD_estimateDStreamSize_fromFrame(truncated)",
    );
    check_result(dstream::ZSTD_initDStream(zds), "ZSTD_initDStream");

    while offset < modern.len() || produced < output.len() {
        let take = (modern.len() - offset).min(7);
        let mut input = ZSTD_inBuffer {
            src: modern[offset..].as_ptr().cast(),
            size: take,
            pos: 0,
        };
        let mut out = ZSTD_outBuffer {
            dst: output[produced..].as_mut_ptr().cast(),
            size: (output.len() - produced).min(4096),
            pos: 0,
        };
        let ret = dstream::ZSTD_decompressStream(zds, &mut out, &mut input);
        check_result(ret, "ZSTD_decompressStream");
        produced += out.pos;
        offset += input.pos;
        if ret == 0 && offset == modern.len() && produced == output.len() {
            break;
        }
        assert!(input.pos != 0 || out.pos != 0, "streaming decoder stalled");
    }

    assert_eq!(produced, output.len());
    assert!(output.iter().all(|&byte| byte == 0));

    check_result(dstream::ZSTD_resetDStream(zds), "ZSTD_resetDStream");
    check_result(
        ddict::ZSTD_initDStream_usingDict(zds, std::ptr::null(), 0),
        "ZSTD_initDStream_usingDict",
    );
    check_result(
        ddict::ZSTD_initDStream_usingDDict(zds, std::ptr::null()),
        "ZSTD_initDStream_usingDDict",
    );

    dstream::ZSTD_freeDStream(zds);

    let mut payload = [0u8; 5];
    let mut variant = 0u32;
    let skippable: [u8; 13] = [
        0x53, 0x2A, 0x4D, 0x18, 0x05, 0x00, 0x00, 0x00, b's', b'a', b'f', b'e', b'!',
    ];
    assert_eq!(
        frame::ZSTD_isFrame(skippable.as_ptr().cast(), skippable.len()),
        1
    );
    assert_eq!(
        frame::ZSTD_isSkippableFrame(skippable.as_ptr().cast(), skippable.len()),
        1
    );
    assert_eq!(
        frame::ZSTD_findFrameCompressedSize(skippable.as_ptr().cast(), skippable.len()),
        skippable.len()
    );
    let read = skippable::ZSTD_readSkippableFrame(
        payload.as_mut_ptr().cast(),
        payload.len(),
        &mut variant,
        skippable.as_ptr().cast(),
        skippable.len(),
    );
    check_result(read, "ZSTD_readSkippableFrame");
    assert_eq!(read, payload.len());
    assert_eq!(variant, 3);
    assert_eq!(&payload, b"safe!");

    let dctx = dctx::ZSTD_createDCtx();
    assert!(!dctx.is_null());
    check_result(
        dctx::ZSTD_decompressBegin(dctx),
        "ZSTD_decompressBegin(skippable)",
    );
    assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx), 5);
    assert_eq!(
        dstream::ZSTD_nextInputType(dctx),
        ZSTD_nextInputType_e::ZSTDnit_frameHeader
    );
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            std::ptr::null_mut(),
            0,
            skippable[..5].as_ptr().cast(),
            5,
        ),
        0
    );
    assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx), 3);
    assert_eq!(
        dstream::ZSTD_nextInputType(dctx),
        ZSTD_nextInputType_e::ZSTDnit_skippableFrame
    );
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            std::ptr::null_mut(),
            0,
            skippable[5..8].as_ptr().cast(),
            3,
        ),
        0
    );
    assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx), payload.len());
    assert_eq!(
        dstream::ZSTD_nextInputType(dctx),
        ZSTD_nextInputType_e::ZSTDnit_skippableFrame
    );
    assert_eq!(
        dctx::ZSTD_decompressContinue(
            dctx,
            std::ptr::null_mut(),
            0,
            skippable[8..].as_ptr().cast(),
            payload.len(),
        ),
        0
    );
    assert_eq!(dstream::ZSTD_nextSrcSizeToDecompress(dctx), 0);
    dctx::ZSTD_freeDCtx(dctx);
}

#[test]
fn decompress_small_single_segment_frames() {
    let (payload, compressed) = small_single_segment_frame();
    let dctx = dctx::ZSTD_createDCtx();
    let zds = dstream::ZSTD_createDStream();
    let mut output = vec![0u8; payload.len()];

    assert!(!dctx.is_null());
    assert!(!zds.is_null());
    assert_eq!(
        frame::ZSTD_getFrameContentSize(compressed.as_ptr().cast(), compressed.len()),
        payload.len() as u64
    );

    let one_shot = dctx::ZSTD_decompress(
        output.as_mut_ptr().cast(),
        output.len(),
        compressed.as_ptr().cast(),
        compressed.len(),
    );
    assert_eq!(one_shot, payload.len());
    assert_eq!(output, payload);

    output.fill(0);
    check_result(
        dctx::ZSTD_decompressBegin(dctx),
        "ZSTD_decompressBegin(tiny single-segment)",
    );
    let mut produced = 0usize;
    let mut offset = 0usize;
    while dstream::ZSTD_nextSrcSizeToDecompress(dctx) != 0 {
        let need = dstream::ZSTD_nextSrcSizeToDecompress(dctx);
        let wrote = dctx::ZSTD_decompressContinue(
            dctx,
            output[produced..].as_mut_ptr().cast(),
            output.len() - produced,
            compressed[offset..offset + need].as_ptr().cast(),
            need,
        );
        check_result(wrote, "ZSTD_decompressContinue(tiny single-segment)");
        produced += wrote;
        offset += need;
    }
    assert_eq!(offset, compressed.len());
    assert_eq!(produced, payload.len());
    assert_eq!(output, payload);

    output.fill(0);
    check_result(
        dstream::ZSTD_initDStream(zds),
        "ZSTD_initDStream(tiny single-segment)",
    );
    produced = 0;
    offset = 0;
    while offset < compressed.len() || produced < payload.len() {
        let take = (compressed.len() - offset).min(2);
        let mut input = ZSTD_inBuffer {
            src: compressed[offset..].as_ptr().cast(),
            size: take,
            pos: 0,
        };
        let mut out = ZSTD_outBuffer {
            dst: output[produced..].as_mut_ptr().cast(),
            size: (output.len() - produced).min(3),
            pos: 0,
        };
        let ret = dstream::ZSTD_decompressStream(zds, &mut out, &mut input);
        check_result(ret, "ZSTD_decompressStream(tiny single-segment)");
        produced += out.pos;
        offset += input.pos;
        if ret == 0 && offset == compressed.len() && produced == payload.len() {
            break;
        }
        assert!(
            input.pos != 0 || out.pos != 0,
            "tiny frame streaming decoder stalled"
        );
    }
    assert_eq!(offset, compressed.len());
    assert_eq!(produced, payload.len());
    assert_eq!(output, payload);

    dstream::ZSTD_freeDStream(zds);
    dctx::ZSTD_freeDCtx(dctx);
}

#[test]
fn decompress_dictionary_roundtrip_via_cli() {
    let dir = temp_dir("dict");
    let dict = dir.join("sample.dict");
    let input = dir.join("input.txt");
    let compressed = dir.join("input.zst");
    let mut sample_paths = Vec::new();
    let sample_payloads = [
        "alpha alpha alpha alpha beta beta beta gamma gamma gamma\n",
        "alpha beta gamma delta epsilon alpha beta gamma delta epsilon\n",
        "dictionary driven decompression coverage dictionary driven decompression coverage\n",
        "repeated phrases make zstd dictionary training much happier on tiny corpora\n",
        "rust owned abi wrapper exercising dictionary decode coverage through the safe shim\n",
        "window log max dictionary id frame probe streaming and one shot decode coverage\n",
        "legacy and modern decompression paths should agree when the host library supports both\n",
        "small files but many samples are enough for the training command used in this test\n",
    ];
    for (index, payload) in sample_payloads.iter().enumerate() {
        let path = dir.join(format!("sample{index}.txt"));
        fs::write(&path, payload.repeat(128)).expect("write sample");
        sample_paths.push(path);
    }
    fs::write(
        &input,
        "alpha alpha alpha alpha beta beta beta gamma gamma gamma delta epsilon\n".repeat(256),
    )
    .expect("write input");

    let mut train_args = vec!["-q", "--train", "--maxdict=8192"];
    for sample in &sample_paths {
        train_args.push(sample.to_str().expect("utf8 path"));
    }
    train_args.push("-o");
    train_args.push(dict.to_str().expect("utf8 path"));
    run_zstd(&train_args);
    run_zstd(&[
        "-q",
        "-f",
        "-D",
        dict.to_str().expect("utf8 path"),
        input.to_str().expect("utf8 path"),
        "-o",
        compressed.to_str().expect("utf8 path"),
    ]);

    let dict_bytes = fs::read(&dict).expect("read dict");
    let input_bytes = fs::read(&input).expect("read input");
    let compressed_bytes = fs::read(&compressed).expect("read compressed");

    let dict_id = ddict::ZSTD_getDictID_fromDict(dict_bytes.as_ptr().cast(), dict_bytes.len());
    let ddict_handle = ddict::ZSTD_createDDict(dict_bytes.as_ptr().cast(), dict_bytes.len());
    let dctx = dctx::ZSTD_createDCtx();
    let mut output = vec![0u8; input_bytes.len()];

    assert_ne!(dict_id, 0);
    assert!(!ddict_handle.is_null());
    assert!(!dctx.is_null());
    assert_eq!(ddict::ZSTD_getDictID_fromDDict(ddict_handle), dict_id);
    assert_eq!(
        frame::ZSTD_getDictID_fromFrame(compressed_bytes.as_ptr().cast(), compressed_bytes.len()),
        dict_id
    );
    assert!(ddict::ZSTD_sizeof_DDict(ddict_handle) > 0);

    let using_dict = dctx::ZSTD_decompress_usingDict(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        compressed_bytes.as_ptr().cast(),
        compressed_bytes.len(),
        dict_bytes.as_ptr().cast(),
        dict_bytes.len(),
    );
    assert_eq!(using_dict, input_bytes.len());
    assert_eq!(output, input_bytes);

    check_result(
        ddict::ZSTD_DCtx_loadDictionary(dctx, dict_bytes.as_ptr().cast(), dict_bytes.len()),
        "ZSTD_DCtx_loadDictionary",
    );
    let loaded = dctx::ZSTD_decompressDCtx(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        compressed_bytes.as_ptr().cast(),
        compressed_bytes.len(),
    );
    assert_eq!(loaded, input_bytes.len());
    assert_eq!(output, input_bytes);

    check_result(
        ddict::ZSTD_DCtx_refDDict(dctx, ddict_handle),
        "ZSTD_DCtx_refDDict",
    );
    let referenced = dctx::ZSTD_decompressDCtx(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        compressed_bytes.as_ptr().cast(),
        compressed_bytes.len(),
    );
    assert_eq!(referenced, input_bytes.len());
    assert_eq!(output, input_bytes);

    let using_ddict = dctx::ZSTD_decompress_usingDDict(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        compressed_bytes.as_ptr().cast(),
        compressed_bytes.len(),
        ddict_handle,
    );
    assert_eq!(using_ddict, input_bytes.len());
    assert_eq!(output, input_bytes);

    check_result(
        ddict::ZSTD_DCtx_refDDict(dctx, ddict_handle),
        "ZSTD_DCtx_refDDict(window limit)",
    );
    check_result(
        dctx::ZSTD_DCtx_setMaxWindowSize(dctx, 1024),
        "ZSTD_DCtx_setMaxWindowSize(window limit)",
    );
    expect_error(
        dctx::ZSTD_decompressDCtx(
            dctx,
            output.as_mut_ptr().cast(),
            output.len(),
            compressed_bytes.as_ptr().cast(),
            compressed_bytes.len(),
        ),
        ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge,
        "ZSTD_decompressDCtx(window limit)",
    );
    expect_error(
        dctx::ZSTD_decompress_usingDict(
            dctx,
            output.as_mut_ptr().cast(),
            output.len(),
            compressed_bytes.as_ptr().cast(),
            compressed_bytes.len(),
            dict_bytes.as_ptr().cast(),
            dict_bytes.len(),
        ),
        ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge,
        "ZSTD_decompress_usingDict(window limit)",
    );
    expect_error(
        dctx::ZSTD_decompress_usingDDict(
            dctx,
            output.as_mut_ptr().cast(),
            output.len(),
            compressed_bytes.as_ptr().cast(),
            compressed_bytes.len(),
            ddict_handle,
        ),
        ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge,
        "ZSTD_decompress_usingDDict(window limit)",
    );
    check_result(
        dctx::ZSTD_DCtx_setMaxWindowSize(dctx, 1 << 23),
        "ZSTD_DCtx_setMaxWindowSize(reset window)",
    );

    let truncated_dict = &dict_bytes[..8];
    let corrupt_dict_result = dctx::ZSTD_decompress_usingDict(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        compressed_bytes.as_ptr().cast(),
        compressed_bytes.len(),
        truncated_dict.as_ptr().cast(),
        truncated_dict.len(),
    );
    expect_error(
        corrupt_dict_result,
        ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted,
        "ZSTD_decompress_usingDict(corrupt dict)",
    );

    let mut corrupt_dictionary_frame = compressed_bytes.clone();
    let corrupt_header_size = frame::ZSTD_frameHeaderSize(
        corrupt_dictionary_frame.as_ptr().cast(),
        corrupt_dictionary_frame.len(),
    );
    corrupt_dictionary_frame[corrupt_header_size] |= 0x06;
    let corrupt_frame_result = dctx::ZSTD_decompress_usingDDict(
        dctx,
        output.as_mut_ptr().cast(),
        output.len(),
        corrupt_dictionary_frame.as_ptr().cast(),
        corrupt_dictionary_frame.len(),
        ddict_handle,
    );
    expect_error(
        corrupt_frame_result,
        ZSTD_ErrorCode::ZSTD_error_corruption_detected,
        "ZSTD_decompress_usingDDict(corrupt frame)",
    );

    ddict::ZSTD_freeDDict(ddict_handle);
    dctx::ZSTD_freeDCtx(dctx);
    let _ = fs::remove_dir_all(dir);
}
