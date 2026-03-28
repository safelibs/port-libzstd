/// Phase-2 keeps allocator-sensitive decompression behavior delegated to the
/// host libzstd implementation behind the narrow FFI loader.
#[allow(dead_code)]
pub(crate) const PHASE2_ALLOC_MODEL: &str = "system-libzstd";
