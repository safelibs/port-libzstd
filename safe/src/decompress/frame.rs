use crate::{
    common::error::{decode_error, error_result},
    decompress::{
        block::{parse_block_header, BlockHeader, BlockType, BLOCK_HEADER_SIZE, BLOCK_SIZE_MAX},
        fse::formatted_dict_id,
        legacy,
    },
    ffi::types::{ZSTD_ErrorCode, ZSTD_format_e, ZSTD_frameHeader, ZSTD_frameType_e},
};
use oxiarc_core::error::OxiArcError;
use std::sync::{Mutex, OnceLock};
use structured_zstd::decoding::{
    BlockDecodingStrategy, Dictionary as StructuredDictionary, FrameDecoder,
};

pub(crate) const ZSTD_MAGICNUMBER: u32 = 0xFD2F_B528;
pub(crate) const ZSTD_MAGIC_SKIPPABLE_START: u32 = 0x184D_2A50;
pub(crate) const ZSTD_MAGIC_SKIPPABLE_MASK: u32 = 0xFFFF_FFF0;
pub(crate) const ZSTD_FRAMEIDSIZE: usize = 4;
pub(crate) const ZSTD_SKIPPABLEHEADERSIZE: usize = 8;
pub(crate) const ZSTD_CONTENTSIZE_UNKNOWN: u64 = u64::MAX;
pub(crate) const ZSTD_CONTENTSIZE_ERROR: u64 = u64::MAX - 1;
pub(crate) const ZSTD_WINDOWLOG_ABSOLUTEMIN: u64 = 10;
pub(crate) const ZSTD_WINDOWLOG_MAX: u64 = if core::mem::size_of::<usize>() == 4 {
    30
} else {
    31
};
pub(crate) const ZSTD_WINDOWLOG_LIMIT_DEFAULT: u64 = 27;
pub(crate) const WILDCOPY_OVERLENGTH: usize = 32;

const DID_FIELD_SIZES: [usize; 4] = [0, 1, 2, 4];
const FCS_FIELD_SIZES: [usize; 4] = [0, 2, 4, 8];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameKind {
    Modern,
    Skippable,
    Legacy(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FrameSizeInfo {
    pub compressed_size: usize,
    pub decompressed_bound: u64,
    pub nb_blocks: usize,
}

pub(crate) enum HeaderProbe {
    Need(usize),
    Header(ZSTD_frameHeader),
}

#[derive(Clone, Copy)]
pub(crate) enum DictionaryRef<'a> {
    None,
    Raw(&'a [u8]),
    Formatted(&'a [u8]),
}

fn read_u32(src: &[u8]) -> u32 {
    u32::from_le_bytes(src[..4].try_into().expect("length checked"))
}

fn read_u16(src: &[u8]) -> u16 {
    u16::from_le_bytes(src[..2].try_into().expect("length checked"))
}

fn read_u64(src: &[u8]) -> u64 {
    u64::from_le_bytes(src[..8].try_into().expect("length checked"))
}

pub(crate) fn starting_input_length(format: ZSTD_format_e) -> usize {
    match format {
        ZSTD_format_e::ZSTD_f_zstd1 => 5,
        ZSTD_format_e::ZSTD_f_zstd1_magicless => 1,
    }
}

pub(crate) fn is_skippable_magic(magic: u32) -> bool {
    (magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START
}

#[allow(dead_code)]
pub(crate) fn partial_frame_prefix_is_valid(src: &[u8], format: ZSTD_format_e) -> bool {
    const LEGACY_MAGICS: [u32; 7] = [
        0x1EB5_2FFD,
        0xFD2F_B522,
        0xFD2F_B523,
        0xFD2F_B524,
        0xFD2F_B525,
        0xFD2F_B526,
        0xFD2F_B527,
    ];

    if format == ZSTD_format_e::ZSTD_f_zstd1_magicless || src.is_empty() || src.len() >= 4 {
        return true;
    }

    if ZSTD_MAGICNUMBER.to_le_bytes().starts_with(src) {
        return true;
    }

    if match src.len() {
        1 => (0x50..=0x5F).contains(&src[0]),
        2 => (0x50..=0x5F).contains(&src[0]) && src[1] == 0x2A,
        3 => (0x50..=0x5F).contains(&src[0]) && src[1] == 0x2A && src[2] == 0x4D,
        _ => false,
    } {
        return true;
    }

    LEGACY_MAGICS
        .iter()
        .any(|magic| magic.to_le_bytes().starts_with(src))
}

pub(crate) fn classify_frame(buffer: &[u8]) -> Option<FrameKind> {
    if buffer.len() < ZSTD_FRAMEIDSIZE {
        return None;
    }
    let magic = read_u32(buffer);
    if magic == ZSTD_MAGICNUMBER {
        return Some(FrameKind::Modern);
    }
    if is_skippable_magic(magic) {
        return Some(FrameKind::Skippable);
    }
    legacy::supported_version(buffer).map(FrameKind::Legacy)
}

fn frame_header_size_internal(src: &[u8], format: ZSTD_format_e) -> Result<usize, ZSTD_ErrorCode> {
    let min_input = starting_input_length(format);
    if src.len() < min_input {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    let fhd = src[min_input - 1];
    let dict_id_code = (fhd & 0x3) as usize;
    let single_segment = ((fhd >> 5) & 1) as usize;
    let fcs_id = (fhd >> 6) as usize;

    Ok(min_input
        + (1 - single_segment)
        + DID_FIELD_SIZES[dict_id_code]
        + FCS_FIELD_SIZES[fcs_id]
        + usize::from(single_segment == 1 && fcs_id == 0))
}

fn read_skippable_frame_size(src: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    if src.len() < ZSTD_SKIPPABLEHEADERSIZE {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }
    let size = read_u32(&src[4..8]) as usize;
    let total = size
        .checked_add(ZSTD_SKIPPABLEHEADERSIZE)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_frameParameter_unsupported)?;
    if total > src.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }
    Ok(total)
}

pub(crate) fn parse_frame_header(
    src: &[u8],
    format: ZSTD_format_e,
) -> Result<HeaderProbe, ZSTD_ErrorCode> {
    let min_input = starting_input_length(format);
    if src.len() < min_input {
        if format != ZSTD_format_e::ZSTD_f_zstd1_magicless {
            if !partial_frame_prefix_is_valid(src, format) {
                return Err(ZSTD_ErrorCode::ZSTD_error_prefix_unknown);
            }
            if src.len() >= 4 {
                let magic = read_u32(src);
                if magic != ZSTD_MAGICNUMBER && !is_skippable_magic(magic) {
                    return Err(ZSTD_ErrorCode::ZSTD_error_prefix_unknown);
                }
            }
        }
        return Ok(HeaderProbe::Need(min_input));
    }

    if format != ZSTD_format_e::ZSTD_f_zstd1_magicless {
        let magic = read_u32(src);
        if magic != ZSTD_MAGICNUMBER {
            if is_skippable_magic(magic) {
                if src.len() < ZSTD_SKIPPABLEHEADERSIZE {
                    return Ok(HeaderProbe::Need(ZSTD_SKIPPABLEHEADERSIZE));
                }
                let payload_size = read_u32(&src[4..8]) as u64;
                return Ok(HeaderProbe::Header(ZSTD_frameHeader {
                    frameContentSize: payload_size,
                    frameType: ZSTD_frameType_e::ZSTD_skippableFrame,
                    ..ZSTD_frameHeader::default()
                }));
            }
            return Err(ZSTD_ErrorCode::ZSTD_error_prefix_unknown);
        }
    }

    let header_size = frame_header_size_internal(src, format)?;
    if src.len() < header_size {
        return Ok(HeaderProbe::Need(header_size));
    }

    let fhd = src[min_input - 1];
    if (fhd & 0x08) != 0 {
        return Err(ZSTD_ErrorCode::ZSTD_error_frameParameter_unsupported);
    }

    let mut pos = min_input;
    let dict_id_code = (fhd & 0x3) as usize;
    let checksum_flag = ((fhd >> 2) & 1) as u32;
    let single_segment = ((fhd >> 5) & 1) != 0;
    let fcs_id = (fhd >> 6) as usize;

    let mut window_size = 0u64;
    if !single_segment {
        let window_descriptor = src[pos];
        pos += 1;
        let window_log = (window_descriptor >> 3) as u64 + ZSTD_WINDOWLOG_ABSOLUTEMIN;
        if window_log > ZSTD_WINDOWLOG_MAX {
            return Err(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge);
        }
        window_size = 1u64 << window_log;
        window_size += (window_size >> 3) * u64::from(window_descriptor & 7);
    }

    let dict_id = match dict_id_code {
        0 => 0,
        1 => {
            let value = u32::from(src[pos]);
            pos += 1;
            value
        }
        2 => {
            let value = u32::from(read_u16(&src[pos..]));
            pos += 2;
            value
        }
        3 => {
            let value = read_u32(&src[pos..]);
            pos += 4;
            value
        }
        _ => unreachable!(),
    };

    let frame_content_size = match fcs_id {
        0 if single_segment => u64::from(src[pos]),
        0 => ZSTD_CONTENTSIZE_UNKNOWN,
        1 => u64::from(read_u16(&src[pos..])) + 256,
        2 => u64::from(read_u32(&src[pos..])),
        3 => read_u64(&src[pos..]),
        _ => unreachable!(),
    };

    if single_segment {
        window_size = frame_content_size;
    }

    Ok(HeaderProbe::Header(ZSTD_frameHeader {
        frameContentSize: frame_content_size,
        windowSize: window_size,
        blockSizeMax: window_size.min(BLOCK_SIZE_MAX as u64) as u32,
        frameType: ZSTD_frameType_e::ZSTD_frame,
        headerSize: header_size as u32,
        dictID: dict_id,
        checksumFlag: checksum_flag,
        _reserved1: 0,
        _reserved2: 0,
    }))
}

fn build_modern_frame_bytes(frame: &[u8], format: ZSTD_format_e) -> Vec<u8> {
    if format == ZSTD_format_e::ZSTD_f_zstd1 {
        return frame.to_vec();
    }

    let mut output = Vec::with_capacity(frame.len() + ZSTD_FRAMEIDSIZE);
    output.extend_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
    output.extend_from_slice(frame);
    output
}

#[allow(dead_code)]
fn encode_window_descriptor(window_size: u64) -> Result<u8, ZSTD_ErrorCode> {
    let window_size = window_size.max(1u64 << ZSTD_WINDOWLOG_ABSOLUTEMIN);
    for window_log in ZSTD_WINDOWLOG_ABSOLUTEMIN..=ZSTD_WINDOWLOG_MAX {
        let window_base = 1u64 << window_log;
        if window_size <= window_base {
            return Ok(((window_log - ZSTD_WINDOWLOG_ABSOLUTEMIN) as u8) << 3);
        }
        let step = window_base >> 3;
        let diff = window_size - window_base;
        let mantissa = diff.div_ceil(step);
        if mantissa > 7 {
            continue;
        }
        return Ok((((window_log - ZSTD_WINDOWLOG_ABSOLUTEMIN) as u8) << 3) | mantissa as u8);
    }
    Err(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)
}

#[allow(dead_code)]
pub(crate) fn build_synthetic_block_frame(
    frame_prefix: &[u8],
    header: ZSTD_frameHeader,
    format: ZSTD_format_e,
    block_body: &[u8],
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let header_size = header.headerSize as usize;
    if frame_prefix.len() < header_size + BLOCK_HEADER_SIZE {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    let mut synthetic = frame_prefix[..header_size].to_vec();
    let descriptor_index = if format == ZSTD_format_e::ZSTD_f_zstd1 {
        ZSTD_FRAMEIDSIZE
    } else {
        0
    };
    if descriptor_index >= synthetic.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }
    synthetic[descriptor_index] &= !0x04;
    let mut block_prefix = frame_prefix[header_size..].to_vec();
    let last_header = block_prefix
        .len()
        .checked_sub(BLOCK_HEADER_SIZE)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong)?;
    block_prefix[last_header] |= 1;

    synthetic.extend_from_slice(&block_prefix);
    synthetic.extend_from_slice(block_body);
    Ok(synthetic)
}

fn map_structured_error(
    error: structured_zstd::decoding::errors::FrameDecoderError,
) -> ZSTD_ErrorCode {
    use structured_zstd::decoding::errors::{
        FrameDecoderError, FrameHeaderError, ReadFrameHeaderError,
    };

    match error {
        FrameDecoderError::ReadFrameHeaderError(ReadFrameHeaderError::BadMagicNumber(_)) => {
            ZSTD_ErrorCode::ZSTD_error_prefix_unknown
        }
        FrameDecoderError::ReadFrameHeaderError(ReadFrameHeaderError::SkipFrame { .. }) => {
            ZSTD_ErrorCode::ZSTD_error_prefix_unknown
        }
        FrameDecoderError::FrameHeaderError(FrameHeaderError::WindowTooBig { .. })
        | FrameDecoderError::WindowSizeTooBig { .. } => {
            ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge
        }
        FrameDecoderError::DictionaryDecodeError(_) => {
            ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted
        }
        FrameDecoderError::DictNotProvided { .. } => ZSTD_ErrorCode::ZSTD_error_dictionary_wrong,
        FrameDecoderError::TargetTooSmall => ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall,
        _ => ZSTD_ErrorCode::ZSTD_error_corruption_detected,
    }
}

fn map_oxiarc_error(error: OxiArcError) -> ZSTD_ErrorCode {
    match error {
        OxiArcError::InvalidMagic { .. } => ZSTD_ErrorCode::ZSTD_error_prefix_unknown,
        OxiArcError::CrcMismatch { .. } => ZSTD_ErrorCode::ZSTD_error_checksum_wrong,
        OxiArcError::UnexpectedEof { .. } => ZSTD_ErrorCode::ZSTD_error_srcSize_wrong,
        OxiArcError::BufferTooSmall { .. } => ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall,
        OxiArcError::CorruptedData { .. }
        | OxiArcError::InvalidHeader { .. }
        | OxiArcError::InvalidHuffmanCode { .. }
        | OxiArcError::InvalidDistance { .. } => ZSTD_ErrorCode::ZSTD_error_corruption_detected,
        _ => ZSTD_ErrorCode::ZSTD_error_GENERIC,
    }
}

fn validate_dictionary_for_frame(
    header: ZSTD_frameHeader,
    dict: DictionaryRef<'_>,
) -> Result<(), ZSTD_ErrorCode> {
    match dict {
        DictionaryRef::None => {
            if header.dictID == 0 {
                Ok(())
            } else {
                Err(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong)
            }
        }
        DictionaryRef::Raw(_) => {
            if header.dictID == 0 {
                Ok(())
            } else {
                Err(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong)
            }
        }
        DictionaryRef::Formatted(bytes) => {
            let dict_id = formatted_dict_id(bytes);
            if header.dictID == 0 || dict_id == header.dictID {
                Ok(())
            } else {
                Err(ZSTD_ErrorCode::ZSTD_error_dictionary_wrong)
            }
        }
    }
}

fn collect_structured_output(
    decoder: &mut FrameDecoder,
    input: &mut &[u8],
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let mut output = Vec::new();

    while !decoder.is_finished() {
        decoder
            .decode_blocks(&mut *input, BlockDecodingStrategy::All)
            .map_err(map_structured_error)?;
        if let Some(chunk) = decoder.collect() {
            output.extend_from_slice(&chunk);
        }
    }
    if let Some(chunk) = decoder.collect() {
        output.extend_from_slice(&chunk);
    }
    if !input.is_empty() {
        return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
    }

    Ok(output)
}

fn frame_checksum(frame: &[u8], header: ZSTD_frameHeader) -> Option<u32> {
    if header.checksumFlag == 0 || frame.len() < 4 {
        return None;
    }
    Some(u32::from_le_bytes(
        frame[frame.len() - 4..]
            .try_into()
            .expect("slice length checked"),
    ))
}

fn decoded_matches_frame(decoded: &[u8], frame: &[u8], header: ZSTD_frameHeader) -> bool {
    if header.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
        && decoded.len() as u64 != header.frameContentSize
    {
        return false;
    }
    match frame_checksum(frame, header) {
        Some(expected) => (crate::ffi::compress::xxh64(decoded) as u32) == expected,
        None => true,
    }
}

fn decode_with_structured_dict(
    frame: &[u8],
    header: ZSTD_frameHeader,
    format: ZSTD_format_e,
    dict: StructuredDictionary,
    validate_decoded_frame: bool,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let input = build_modern_frame_bytes(frame, format);
    let mut remaining = input.as_slice();
    let mut decoder = FrameDecoder::new();
    let dict_id = dict.id;

    decoder.add_dict(dict).map_err(map_structured_error)?;
    decoder.init(&mut remaining).map_err(map_structured_error)?;
    if header.dictID == 0 {
        decoder.force_dict(dict_id).map_err(map_structured_error)?;
    }

    let decoded = collect_structured_output(&mut decoder, &mut remaining)?;
    if !validate_decoded_frame || decoded_matches_frame(&decoded, frame, header) {
        Ok(decoded)
    } else {
        Err(ZSTD_ErrorCode::ZSTD_error_checksum_wrong)
    }
}

fn decode_with_oxiarc_dict(
    frame: &[u8],
    format: ZSTD_format_e,
    dict: &[u8],
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let input = build_modern_frame_bytes(frame, format);
    static OXIARC_PANIC_HOOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let hook_guard = OXIARC_PANIC_HOOK_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("panic hook mutex poisoned");
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let oxiarc_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut decoder = oxiarc_zstd::ZstdDecoder::new();
        decoder.set_dictionary(dict);
        decoder.decode_frame(&input).map_err(map_oxiarc_error)
    }));
    std::panic::set_hook(panic_hook);
    drop(hook_guard);
    match oxiarc_result {
        Ok(Ok(decoded)) => Ok(decoded),
        Ok(Err(code)) => Err(code),
        Err(_) => Err(ZSTD_ErrorCode::ZSTD_error_GENERIC),
    }
}

fn decode_with_formatted_dict(
    frame: &[u8],
    header: ZSTD_frameHeader,
    format: ZSTD_format_e,
    dict: &[u8],
    validate_decoded_frame: bool,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let raw_content = crate::dict_builder::zdict::formatted_dictionary_content(dict)?;
    let dict_id = formatted_dict_id(dict);
    match StructuredDictionary::decode_dict(dict) {
        Ok(dict) => {
            decode_with_structured_dict(frame, header, format, dict, validate_decoded_frame)
                .or_else(|_| {
                    decode_with_raw_dict_id(
                        frame,
                        header,
                        format,
                        raw_content,
                        dict_id,
                        validate_decoded_frame,
                    )
                })
                .or_else(|_| decode_with_oxiarc_dict(frame, format, raw_content))
        }
        Err(_) => {
            let synthetic = crate::dict_builder::zdict::synthesize_formatted_dictionary(
                raw_content,
                dict,
                dict_id,
            )?;
            match StructuredDictionary::decode_dict(&synthetic) {
                Ok(dict) => {
                    decode_with_structured_dict(frame, header, format, dict, validate_decoded_frame)
                        .or_else(|_| {
                            decode_with_raw_dict_id(
                                frame,
                                header,
                                format,
                                raw_content,
                                dict_id,
                                validate_decoded_frame,
                            )
                        })
                        .or_else(|_| decode_with_oxiarc_dict(frame, format, raw_content))
                }
                Err(_) => decode_with_oxiarc_dict(frame, format, raw_content),
            }
        }
    }
}

fn decode_with_raw_dict_id(
    frame: &[u8],
    header: ZSTD_frameHeader,
    format: ZSTD_format_e,
    dict: &[u8],
    dict_id: u32,
    validate_decoded_frame: bool,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let synthetic =
        crate::dict_builder::zdict::synthesize_formatted_dictionary(dict, dict, dict_id)?;
    let dict = StructuredDictionary::decode_dict(&synthetic)
        .map_err(|_| ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted)?;
    decode_with_structured_dict(frame, header, format, dict, validate_decoded_frame)
}

fn decode_with_raw_dict(
    frame: &[u8],
    header: ZSTD_frameHeader,
    format: ZSTD_format_e,
    dict: &[u8],
    validate_decoded_frame: bool,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    decode_with_raw_dict_id(frame, header, format, dict, 1, validate_decoded_frame)
}

fn decode_without_dict(frame: &[u8], format: ZSTD_format_e) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let input = build_modern_frame_bytes(frame, format);
    static OXIARC_PANIC_HOOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let hook_guard = OXIARC_PANIC_HOOK_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("panic hook mutex poisoned");
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let oxiarc_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut decoder = oxiarc_zstd::ZstdDecoder::new();
        decoder.decode_frame(&input).map_err(map_oxiarc_error)
    }));
    std::panic::set_hook(panic_hook);
    drop(hook_guard);
    match oxiarc_result {
        Ok(Ok(decoded)) => return Ok(decoded),
        Ok(Err(_)) | Err(_) => {}
    }

    let mut remaining = input.as_slice();
    let mut decoder = FrameDecoder::new();
    decoder.init(&mut remaining).map_err(map_structured_error)?;
    collect_structured_output(&mut decoder, &mut remaining)
}

fn decode_single_modern_frame_impl(
    frame: &[u8],
    header: ZSTD_frameHeader,
    dict: DictionaryRef<'_>,
    format: ZSTD_format_e,
    max_window_size: usize,
    validate_decoded_frame: bool,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    if header.windowSize as usize > max_window_size {
        return Err(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge);
    }

    validate_dictionary_for_frame(header, dict)?;

    match dict {
        DictionaryRef::Raw(bytes) if !bytes.is_empty() => {
            return decode_with_raw_dict(frame, header, format, bytes, validate_decoded_frame)
        }
        DictionaryRef::Formatted(bytes) if !bytes.is_empty() => {
            return decode_with_formatted_dict(frame, header, format, bytes, validate_decoded_frame)
        }
        _ => {}
    }

    decode_without_dict(frame, format)
}

pub(crate) fn find_frame_size_info(
    src: &[u8],
    format: ZSTD_format_e,
) -> Result<FrameSizeInfo, ZSTD_ErrorCode> {
    if format != ZSTD_format_e::ZSTD_f_zstd1_magicless {
        if let Some(FrameKind::Legacy(_)) = classify_frame(src) {
            let compressed_size = legacy::find_frame_compressed_size(src)?;
            return Ok(FrameSizeInfo {
                compressed_size,
                decompressed_bound: legacy::find_decompressed_bound(src),
                nb_blocks: 0,
            });
        }
        if let Some(FrameKind::Skippable) = classify_frame(src) {
            return Ok(FrameSizeInfo {
                compressed_size: read_skippable_frame_size(src)?,
                decompressed_bound: 0,
                nb_blocks: 0,
            });
        }
    }

    let header = match parse_frame_header(src, format)? {
        HeaderProbe::Need(_) => return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong),
        HeaderProbe::Header(header) => header,
    };
    if header.frameType == ZSTD_frameType_e::ZSTD_skippableFrame {
        return Ok(FrameSizeInfo {
            compressed_size: read_skippable_frame_size(src)?,
            decompressed_bound: 0,
            nb_blocks: 0,
        });
    }

    let mut pos = header.headerSize as usize;
    let mut nb_blocks = 0usize;
    loop {
        if src.len().saturating_sub(pos) < BLOCK_HEADER_SIZE {
            return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
        }
        let BlockHeader {
            last_block,
            block_type,
            content_size,
        } = parse_block_header(&src[pos..pos + BLOCK_HEADER_SIZE])?;
        pos += BLOCK_HEADER_SIZE;
        let compressed_size = if block_type == BlockType::Rle {
            1
        } else {
            content_size
        };
        if src.len().saturating_sub(pos) < compressed_size {
            return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
        }
        pos += compressed_size;
        nb_blocks += 1;
        if last_block {
            break;
        }
    }

    if header.checksumFlag != 0 {
        if src.len().saturating_sub(pos) < 4 {
            return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong);
        }
        pos += 4;
    }

    let decompressed_bound = if header.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
        header.frameContentSize
    } else {
        nb_blocks as u64 * u64::from(header.blockSizeMax)
    };

    Ok(FrameSizeInfo {
        compressed_size: pos,
        decompressed_bound,
        nb_blocks,
    })
}

#[allow(dead_code)]
pub(crate) fn archive_is_complete(
    src: &[u8],
    format: ZSTD_format_e,
) -> Result<bool, ZSTD_ErrorCode> {
    let mut remaining = src;
    while !remaining.is_empty() {
        if remaining.len() < starting_input_length(format) {
            return Ok(false);
        }

        match find_frame_size_info(remaining, format) {
            Ok(info) => {
                if info.compressed_size > remaining.len() {
                    return Ok(false);
                }
                remaining = &remaining[info.compressed_size..];
            }
            Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong) => return Ok(false),
            Err(code) => return Err(code),
        }
    }
    Ok(true)
}

fn decode_all_frames_impl(
    src: &[u8],
    dict: DictionaryRef<'_>,
    format: ZSTD_format_e,
    max_window_size: usize,
    validate_decoded_frame: bool,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    let mut remaining = src;
    let mut output = Vec::new();

    while !remaining.is_empty() {
        if format != ZSTD_format_e::ZSTD_f_zstd1_magicless {
            match classify_frame(remaining) {
                Some(FrameKind::Skippable) => {
                    let skip = read_skippable_frame_size(remaining)?;
                    remaining = &remaining[skip..];
                    continue;
                }
                Some(FrameKind::Legacy(_)) => {
                    let frame_size = legacy::find_frame_compressed_size(remaining)?;
                    let supported = &remaining[..frame_size];
                    let mut buffer = vec![0u8; legacy::find_decompressed_bound(supported) as usize];
                    let decoded = legacy::decompress(
                        buffer.as_mut_slice(),
                        supported,
                        match dict {
                            DictionaryRef::None => None,
                            DictionaryRef::Raw(bytes) => Some(bytes),
                            DictionaryRef::Formatted(bytes) => Some(bytes),
                        },
                    )?;
                    buffer.truncate(decoded);
                    output.extend_from_slice(&buffer);
                    remaining = &remaining[frame_size..];
                    continue;
                }
                Some(FrameKind::Modern) => {}
                None => return Err(ZSTD_ErrorCode::ZSTD_error_prefix_unknown),
            }
        }

        let header = match parse_frame_header(remaining, format)? {
            HeaderProbe::Need(_) => return Err(ZSTD_ErrorCode::ZSTD_error_srcSize_wrong),
            HeaderProbe::Header(header) => header,
        };
        let info = find_frame_size_info(remaining, format)?;
        let decoded = decode_single_modern_frame_impl(
            &remaining[..info.compressed_size],
            header,
            dict,
            format,
            max_window_size,
            validate_decoded_frame,
        )?;
        output.extend_from_slice(&decoded);
        remaining = &remaining[info.compressed_size..];
    }

    Ok(output)
}

pub(crate) fn decode_all_frames(
    src: &[u8],
    dict: DictionaryRef<'_>,
    format: ZSTD_format_e,
    max_window_size: usize,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    decode_all_frames_impl(src, dict, format, max_window_size, true)
}

pub(crate) fn decode_all_frames_relaxed(
    src: &[u8],
    dict: DictionaryRef<'_>,
    format: ZSTD_format_e,
    max_window_size: usize,
) -> Result<Vec<u8>, ZSTD_ErrorCode> {
    decode_all_frames_impl(src, dict, format, max_window_size, false)
}

#[allow(dead_code)]
pub(crate) fn dictionary_ref<'a>(dict: Option<&'a [u8]>) -> DictionaryRef<'a> {
    match dict {
        None => DictionaryRef::None,
        Some(bytes) if bytes.is_empty() => DictionaryRef::None,
        Some(bytes) if crate::decompress::huf::is_formatted_dictionary(bytes) => {
            let _ = formatted_dict_id(bytes);
            DictionaryRef::Formatted(bytes)
        }
        Some(bytes) => DictionaryRef::Raw(bytes),
    }
}

pub(crate) fn get_frame_content_size(src: &[u8]) -> u64 {
    if let Some(FrameKind::Legacy(_)) = classify_frame(src) {
        let size = legacy::get_decompressed_size(src);
        return if size == 0 {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            size
        };
    }

    match parse_frame_header(src, ZSTD_format_e::ZSTD_f_zstd1) {
        Ok(HeaderProbe::Header(header))
            if header.frameType == ZSTD_frameType_e::ZSTD_skippableFrame =>
        {
            0
        }
        Ok(HeaderProbe::Header(header)) => header.frameContentSize,
        _ => ZSTD_CONTENTSIZE_ERROR,
    }
}

pub(crate) fn get_decompressed_size(src: &[u8]) -> u64 {
    let value = get_frame_content_size(src);
    if value >= ZSTD_CONTENTSIZE_ERROR {
        0
    } else {
        value
    }
}

pub(crate) fn find_frame_compressed_size(src: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    find_frame_size_info(src, ZSTD_format_e::ZSTD_f_zstd1).map(|info| info.compressed_size)
}

pub(crate) fn find_decompressed_size(src: &[u8]) -> u64 {
    let mut remaining = src;
    let mut total = 0u64;

    while remaining.len() >= starting_input_length(ZSTD_format_e::ZSTD_f_zstd1) {
        let value = get_frame_content_size(remaining);
        if value >= ZSTD_CONTENTSIZE_ERROR {
            return value;
        }
        total = match total.checked_add(value) {
            Some(value) => value,
            None => return ZSTD_CONTENTSIZE_ERROR,
        };

        match find_frame_compressed_size(remaining) {
            Ok(size) => remaining = &remaining[size..],
            Err(_) => return ZSTD_CONTENTSIZE_ERROR,
        }
    }

    if !remaining.is_empty() {
        ZSTD_CONTENTSIZE_ERROR
    } else {
        total
    }
}

pub(crate) fn decompress_bound(src: &[u8]) -> u64 {
    let mut remaining = src;
    let mut bound = 0u64;

    while !remaining.is_empty() {
        let info = match find_frame_size_info(remaining, ZSTD_format_e::ZSTD_f_zstd1) {
            Ok(info) => info,
            Err(_) => return ZSTD_CONTENTSIZE_ERROR,
        };
        bound = match bound.checked_add(info.decompressed_bound) {
            Some(value) => value,
            None => return ZSTD_CONTENTSIZE_ERROR,
        };
        remaining = &remaining[info.compressed_size..];
    }

    bound
}

pub(crate) fn decompression_margin(src: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    let mut remaining = src;
    let mut margin = 0usize;
    let mut max_block_size = 0usize;

    while !remaining.is_empty() {
        let info = find_frame_size_info(remaining, ZSTD_format_e::ZSTD_f_zstd1)?;
        let header = match parse_frame_header(remaining, ZSTD_format_e::ZSTD_f_zstd1)? {
            HeaderProbe::Need(_) => return Err(ZSTD_ErrorCode::ZSTD_error_corruption_detected),
            HeaderProbe::Header(header) => header,
        };

        if header.frameType == ZSTD_frameType_e::ZSTD_frame {
            margin = margin
                .checked_add(header.headerSize as usize)
                .and_then(|value| value.checked_add(usize::from(header.checksumFlag != 0) * 4))
                .and_then(|value| value.checked_add(info.nb_blocks * BLOCK_HEADER_SIZE))
                .ok_or(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)?;
            max_block_size = max_block_size.max(header.blockSizeMax as usize);
        } else {
            margin = margin
                .checked_add(info.compressed_size)
                .ok_or(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)?;
        }

        remaining = &remaining[info.compressed_size..];
    }

    margin
        .checked_add(max_block_size)
        .ok_or(ZSTD_ErrorCode::ZSTD_error_frameParameter_windowTooLarge)
}

pub(crate) fn is_frame(src: &[u8]) -> bool {
    classify_frame(src).is_some()
}

pub(crate) fn is_skippable_frame(src: &[u8]) -> bool {
    matches!(classify_frame(src), Some(FrameKind::Skippable))
}

pub(crate) fn get_dict_id_from_frame(src: &[u8]) -> u32 {
    match parse_frame_header(src, ZSTD_format_e::ZSTD_f_zstd1) {
        Ok(HeaderProbe::Header(header)) => header.dictID,
        _ => 0,
    }
}

pub(crate) fn copy_decoded_to_ptr(
    decoded: &[u8],
    dst: *mut core::ffi::c_void,
    dst_capacity: usize,
) -> usize {
    if decoded.len() > dst_capacity {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    if decoded.is_empty() {
        return 0;
    }
    if dst.is_null() {
        return error_result(ZSTD_ErrorCode::ZSTD_error_dstBuffer_null);
    }

    // SAFETY: The caller guarantees that `dst` points to `dst_capacity` writable bytes.
    unsafe {
        core::ptr::copy_nonoverlapping(decoded.as_ptr(), dst.cast::<u8>(), decoded.len());
    }
    decoded.len()
}

pub(crate) fn decode_error_result(result: usize) -> Result<usize, ZSTD_ErrorCode> {
    if crate::common::error::is_error_result(result) {
        Err(decode_error(result))
    } else {
        Ok(result)
    }
}
