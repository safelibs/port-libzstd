#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"

#define CHECK(cond, ...)                             \
    do {                                             \
        if (!(cond)) {                               \
            fprintf(stderr, __VA_ARGS__);            \
            return 1;                                \
        }                                            \
    } while (0)

#define CHECK_ZSTD(expr)                                             \
    do {                                                             \
        size_t const zstd_ret = (expr);                              \
        CHECK(!ZSTD_isError(zstd_ret), "%s: %s\n", #expr,            \
              ZSTD_getErrorName(zstd_ret));                          \
    } while (0)

static void fill_sample(unsigned char* dst, size_t size, unsigned seed)
{
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        size_t i;
        for (i = 0; i < 96U && pos < size; ++i) {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            dst[pos++] = (unsigned char)alphabet[state % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) {
            dst[pos++] = '\n';
        }
    }
}

static int roundtrip_with_pool(ZSTD_threadPool* pool,
                               const unsigned char* src,
                               size_t srcSize,
                               int level,
                               unsigned workers)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const compressed = (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    unsigned char* const decoded = (unsigned char*)malloc(srcSize == 0 ? 1 : srcSize);
    size_t dstPos = 0;
    size_t srcPos = 0;
    size_t cSize;
    ZSTD_frameProgression progression;

    CHECK(cctx != NULL && compressed != NULL && decoded != NULL, "allocation failure\n");

    CHECK_ZSTD(ZSTD_CCtx_refThreadPool(cctx, pool));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, level));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_jobSize, 1 << 18));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_overlapLog, 5));

    for (;;) {
        size_t const remaining = ZSTD_compressStream2_simpleArgs(
            cctx,
            compressed,
            ZSTD_compressBound(srcSize),
            &dstPos,
            src,
            srcSize,
            &srcPos,
            ZSTD_e_end);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2_simpleArgs failed: %s\n",
              ZSTD_getErrorName(remaining));
        progression = ZSTD_getFrameProgression(cctx);
        CHECK(progression.ingested >= progression.consumed,
              "invalid frame progression counters\n");
        (void)ZSTD_toFlushNow(cctx);
        if (remaining == 0) {
            break;
        }
        CHECK(dstPos < ZSTD_compressBound(srcSize), "compression output overflow\n");
    }

    cSize = dstPos;
    CHECK(srcPos == srcSize, "compression did not consume all input\n");

    {
        size_t const decodedSize = ZSTD_decompress(decoded, srcSize == 0 ? 1 : srcSize,
                                                   compressed, cSize);
        CHECK(!ZSTD_isError(decodedSize), "ZSTD_decompress failed: %s\n",
              ZSTD_getErrorName(decodedSize));
        CHECK(decodedSize == srcSize, "decoded size mismatch\n");
        CHECK(memcmp(decoded, src, srcSize) == 0, "decoded payload mismatch\n");
    }

    CHECK_ZSTD(ZSTD_CCtx_refThreadPool(cctx, NULL));
    ZSTD_freeCCtx(cctx);
    free(decoded);
    free(compressed);
    return 0;
}

int main(void)
{
    size_t const sizeA = 320U * 1024U;
    size_t const sizeB = 196U * 1024U;
    unsigned char* const sampleA = (unsigned char*)malloc(sizeA);
    unsigned char* const sampleB = (unsigned char*)malloc(sizeB);
    ZSTD_threadPool* const pool = ZSTD_createThreadPool(3);

    CHECK(sampleA != NULL && sampleB != NULL && pool != NULL, "allocation failure\n");

    fill_sample(sampleA, sizeA, 0x1234ABCDU);
    fill_sample(sampleB, sizeB, 0xBEEF1234U);

    if (roundtrip_with_pool(pool, sampleA, sizeA, 4, 2) ||
        roundtrip_with_pool(pool, sampleB, sizeB, 5, 2)) {
        ZSTD_freeThreadPool(pool);
        free(sampleB);
        free(sampleA);
        return 1;
    }

    ZSTD_freeThreadPool(pool);
    free(sampleB);
    free(sampleA);
    return 0;
}
