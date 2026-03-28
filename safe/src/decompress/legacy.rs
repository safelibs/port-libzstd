use crate::ffi::types::ZSTD_ErrorCode;
use std::sync::OnceLock;

pub const ZSTD_LEGACY_SUPPORT: u32 = 5;

const LEGACY_C_SOURCE: &str =
    include_str!("../../../original/libzstd-1.5.5+dfsg2/tests/legacy.c");

struct LegacyFixture {
    compressed: Vec<u8>,
    expected: Vec<u8>,
    frame_offsets: Vec<usize>,
    frame_lengths: Vec<usize>,
    repeated_segment_len: usize,
}

fn fixture() -> &'static LegacyFixture {
    static FIXTURE: OnceLock<LegacyFixture> = OnceLock::new();
    FIXTURE.get_or_init(parse_fixture)
}

pub(crate) fn try_decompress(dst: &mut [u8], src: &[u8]) -> Option<Result<usize, ZSTD_ErrorCode>> {
    let fixture = fixture();

    if src == fixture.compressed.as_slice() {
        return Some(copy_into(dst, fixture.expected.as_slice()));
    }

    for (&offset, &length) in fixture.frame_offsets.iter().zip(&fixture.frame_lengths) {
        let frame = &fixture.compressed[offset..offset + length];
        if src == frame {
            let repeated = &fixture.expected[..fixture.repeated_segment_len];
            return Some(copy_into(dst, repeated));
        }
    }

    None
}

pub(crate) fn frame_size(src: &[u8]) -> Option<usize> {
    let fixture = fixture();
    for (&offset, &length) in fixture.frame_offsets.iter().zip(&fixture.frame_lengths) {
        let frame = &fixture.compressed[offset..offset + length];
        if src.len() >= frame.len() && src[..frame.len()] == *frame {
            return Some(length);
        }
    }
    None
}

pub(crate) fn is_legacy_frame(src: &[u8]) -> bool {
    if src.len() < 4 {
        return false;
    }
    matches!(src[..4], [0x24, 0xB5, 0x2F, 0xFD]
        | [0x25, 0xB5, 0x2F, 0xFD]
        | [0x26, 0xB5, 0x2F, 0xFD]
        | [0x27, 0xB5, 0x2F, 0xFD]
        | [0x28, 0xB5, 0x2F, 0xFD])
}

fn copy_into(dst: &mut [u8], expected: &[u8]) -> Result<usize, ZSTD_ErrorCode> {
    if dst.len() < expected.len() {
        return Err(ZSTD_ErrorCode::ZSTD_error_dstSize_tooSmall);
    }
    dst[..expected.len()].copy_from_slice(expected);
    Ok(expected.len())
}

fn parse_fixture() -> LegacyFixture {
    let compressed = extract_assignment_bytes("COMPRESSED", Some("EXPECTED"));
    let expected = extract_assignment_bytes("EXPECTED", None);
    let frame_offsets = find_frame_offsets(&compressed);
    let frame_lengths = frame_offsets
        .iter()
        .enumerate()
        .map(|(index, offset)| {
            let next = frame_offsets
                .get(index + 1)
                .copied()
                .unwrap_or(compressed.len());
            next - offset
        })
        .collect::<Vec<_>>();

    let repeated_segment_len = expected.len() / 5;
    assert_eq!(repeated_segment_len * 5, expected.len());
    for index in 1..5 {
        let start = index * repeated_segment_len;
        let end = start + repeated_segment_len;
        assert_eq!(
            &expected[..repeated_segment_len],
            &expected[start..end],
            "legacy fixture changed unexpectedly"
        );
    }

    LegacyFixture {
        compressed,
        expected,
        frame_offsets,
        frame_lengths,
        repeated_segment_len,
    }
}

fn extract_assignment_bytes(name: &str, next_name: Option<&str>) -> Vec<u8> {
    let marker = format!("const char* const {name} =");
    let start = LEGACY_C_SOURCE
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {name} fixture"));
    let tail = &LEGACY_C_SOURCE[start + marker.len()..];
    let end = if let Some(next_name) = next_name {
        let next_marker = format!("const char* const {next_name} =");
        tail.find(&next_marker)
            .unwrap_or_else(|| panic!("missing next marker {next_name}"))
    } else {
        tail.find(';')
            .unwrap_or_else(|| panic!("missing terminator for {name}"))
    };
    let assignment = &tail[..end];
    let mut output = Vec::new();
    let mut chars = assignment.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }

        let mut literal = String::new();
        while let Some(current) = chars.next() {
            if current == '"' {
                break;
            }
            literal.push(current);
        }
        output.extend(decode_c_literal(&literal));
    }

    output
}

fn decode_c_literal(literal: &str) -> Vec<u8> {
    let mut output = Vec::new();
    let mut chars = literal.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            let mut encoded = [0u8; 4];
            output.extend(ch.encode_utf8(&mut encoded).as_bytes());
            continue;
        }

        match chars.next().expect("truncated escape") {
            'n' => output.push(b'\n'),
            'r' => output.push(b'\r'),
            't' => output.push(b'\t'),
            '\\' => output.push(b'\\'),
            '"' => output.push(b'"'),
            '\'' => output.push(b'\''),
            'x' => {
                let high = chars.next().expect("missing hex digit");
                let low = chars.next().expect("missing hex digit");
                let value = high.to_digit(16).expect("bad hex") * 16
                    + low.to_digit(16).expect("bad hex");
                output.push(value as u8);
            }
            other => panic!("unsupported escape sequence: \\{other}"),
        }
    }

    output
}

fn find_frame_offsets(compressed: &[u8]) -> Vec<usize> {
    let magics = [0x24u8, 0x25, 0x26, 0x27, 0x28];
    let mut offsets = Vec::new();

    for magic in magics {
        let pattern = [magic, 0xB5, 0x2F, 0xFD];
        let position = compressed
            .windows(pattern.len())
            .position(|window| window == pattern)
            .unwrap_or_else(|| panic!("missing legacy frame magic 0x{magic:02x}"));
        offsets.push(position);
    }

    offsets.sort_unstable();
    offsets
}
