#include <stddef.h>

#define main upstream_zstreamtest_main
#include "../../../original/libzstd-1.5.5+dfsg2/tests/zstreamtest.c"
#undef main

size_t ZDICT_trainFromBuffer(void* dictBuffer, size_t dictBufferCapacity,
                             const void* samplesBuffer,
                             const size_t* samplesSizes, unsigned nbSamples)
{
    (void)dictBuffer;
    (void)dictBufferCapacity;
    (void)samplesBuffer;
    (void)samplesSizes;
    (void)nbSamples;
    return (size_t)-1;
}

unsigned ZDICT_isError(size_t errorCode)
{
    return ZSTD_isError(errorCode);
}

int main(void)
{
    size_t const srcSize = 384U * 1024U;
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    unsigned seed = 0x1234ABCDU;
    void* compressed = NULL;
    size_t compressedSize = 0;
    ZSTD_DCtx* dctx = NULL;
    unsigned char* decoded = NULL;
    size_t decodedSize = 0;

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    generateSample(src, srcSize, seed);

    if (runLegacyRoundTrip(src, srcSize, seed ^ 0x11111111U) ||
        runNewRoundTrip(src, srcSize, 4, 0, NULL, 0, seed ^ 0x22222222U) ||
        testSkippableFrame()) {
        free(src);
        return 1;
    }

    if (compressNewStream(src, srcSize, 5, 0, NULL, 0, seed ^ 0x44444444U,
                          &compressed, &compressedSize)) {
        free(src);
        free(compressed);
        return 1;
    }

    dctx = ZSTD_createDCtx();
    decoded = (unsigned char*)malloc(srcSize);
    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        free(src);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    if (streamDecodeFully(dctx, compressed, compressedSize, decoded, srcSize,
                          compressedSize, srcSize, 0, &seed, &decodedSize) ||
        testByteByByteInput(compressed, compressedSize, src, srcSize) ||
        testNoForwardProgress(compressed, compressedSize, srcSize)) {
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        free(src);
        return 1;
    }

    CHECK(decodedSize == srcSize, "decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "decoded data mismatch\n");

    ZSTD_freeDCtx(dctx);
    free(decoded);
    free(compressed);
    free(src);
    return 0;
}
