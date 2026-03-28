#include "zstd_legacy.h"

unsigned libzstd_safe_legacy_support(void) {
    return ZSTD_LEGACY_SUPPORT;
}

unsigned libzstd_safe_is_legacy(const void* src, size_t srcSize) {
    return ZSTD_isLegacy(src, srcSize);
}

unsigned long long libzstd_safe_get_decompressed_size_legacy(const void* src, size_t srcSize) {
    return ZSTD_getDecompressedSize_legacy(src, srcSize);
}

size_t libzstd_safe_decompress_legacy(
    void* dst,
    size_t dstCapacity,
    const void* src,
    size_t compressedSize,
    const void* dict,
    size_t dictSize
) {
    return ZSTD_decompressLegacy(dst, dstCapacity, src, compressedSize, dict, dictSize);
}

size_t libzstd_safe_find_frame_compressed_size_legacy(const void* src, size_t srcSize) {
    return ZSTD_findFrameCompressedSizeLegacy(src, srcSize);
}

unsigned long long libzstd_safe_find_decompressed_bound_legacy(const void* src, size_t srcSize) {
    return ZSTD_findFrameSizeInfoLegacy(src, srcSize).decompressedBound;
}

size_t libzstd_safe_free_legacy_stream(void* legacyContext, unsigned version) {
    return ZSTD_freeLegacyStreamContext(legacyContext, version);
}

size_t libzstd_safe_init_legacy_stream(
    void** legacyContext,
    unsigned prevVersion,
    unsigned newVersion,
    const void* dict,
    size_t dictSize
) {
    return ZSTD_initLegacyStream(legacyContext, prevVersion, newVersion, dict, dictSize);
}

size_t libzstd_safe_decompress_legacy_stream(
    void* legacyContext,
    unsigned version,
    ZSTD_outBuffer* output,
    ZSTD_inBuffer* input
) {
    return ZSTD_decompressLegacyStream(legacyContext, version, output, input);
}
