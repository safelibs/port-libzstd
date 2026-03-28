pub(crate) const DICTIONARY_MAGIC: [u8; 4] = [0x37, 0xA4, 0x30, 0xEC];

pub(crate) fn is_formatted_dictionary(bytes: &[u8]) -> bool {
    bytes.len() >= DICTIONARY_MAGIC.len() && bytes[..DICTIONARY_MAGIC.len()] == DICTIONARY_MAGIC
}

pub(crate) fn header_prefix_matches(bytes: &[u8]) -> bool {
    let prefix_len = bytes.len().min(DICTIONARY_MAGIC.len());
    DICTIONARY_MAGIC[..prefix_len] == bytes[..prefix_len]
}
