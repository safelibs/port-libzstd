use crate::ffi::types::ZSTD_ErrorCode;

pub(crate) fn formatted_dict_id(bytes: &[u8]) -> u32 {
    if bytes.len() < 8 || !crate::decompress::huf::is_formatted_dictionary(bytes) {
        return 0;
    }
    u32::from_le_bytes(bytes[4..8].try_into().expect("slice length checked"))
}

pub(crate) fn validate_dictionary_kind(bytes: &[u8]) -> Result<(), ZSTD_ErrorCode> {
    if bytes.is_empty()
        || crate::decompress::huf::is_formatted_dictionary(bytes)
        || bytes.len() < 4
        || !crate::decompress::huf::header_prefix_matches(bytes)
    {
        return Ok(());
    }

    Err(ZSTD_ErrorCode::ZSTD_error_dictionary_corrupted)
}
