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

static int check_non_mt_progression_contract(void)
{
    size_t const srcSize = 64U * 1024U;
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    unsigned char* const compressed = (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    size_t dstPos = 0;
    size_t srcPos = 0;
    size_t remaining;
    ZSTD_frameProgression progression;

    CHECK(cctx != NULL && src != NULL && compressed != NULL, "allocation failure\n");

    fill_sample(src, srcSize, 0xA51CE55U);
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, 0));

    remaining = ZSTD_compressStream2_simpleArgs(
        cctx,
        compressed,
        ZSTD_compressBound(srcSize),
        &dstPos,
        src,
        srcSize,
        &srcPos,
        ZSTD_e_continue);
    CHECK(!ZSTD_isError(remaining), "non-MT ZSTD_compressStream2_simpleArgs failed: %s\n",
          ZSTD_getErrorName(remaining));

    progression = ZSTD_getFrameProgression(cctx);
    CHECK(progression.ingested == srcSize, "non-MT ingested mismatch\n");
    CHECK(progression.consumed < progression.ingested,
          "non-MT progression no longer reports buffered input\n");
    CHECK(progression.produced == progression.flushed,
          "non-MT produced/flushed mismatch\n");
    CHECK(progression.produced == dstPos,
          "non-MT produced bytes mismatch\n");
    CHECK(progression.currentJobID == 0U, "non-MT currentJobID must stay zero\n");
    CHECK(progression.nbActiveWorkers == 0U, "non-MT active workers must stay zero\n");
    CHECK(ZSTD_toFlushNow(cctx) == 0U, "non-MT ZSTD_toFlushNow must stay zero\n");

    ZSTD_freeCCtx(cctx);
    free(compressed);
    free(src);
    return 0;
}

static int check_mt_progression_contract(void)
{
    size_t const srcSize = 256U * 1024U;
    ZSTD_threadPool* const pool = ZSTD_createThreadPool(2);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    unsigned char continueDst[16];
    unsigned char flushDst[64];
    size_t dstPos = 0;
    size_t srcPos = 0;
    size_t remaining;
    ZSTD_frameProgression progression;
    size_t flushNow;

    CHECK(pool != NULL && cctx != NULL && src != NULL, "allocation failure\n");

    fill_sample(src, srcSize, 0x51A7E0U);
    CHECK_ZSTD(ZSTD_CCtx_refThreadPool(cctx, pool));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, 2));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_jobSize, 1 << 17));

    remaining = ZSTD_compressStream2_simpleArgs(
        cctx,
        continueDst,
        sizeof(continueDst),
        &dstPos,
        src,
        srcSize,
        &srcPos,
        ZSTD_e_continue);
    CHECK(!ZSTD_isError(remaining), "MT continue failed: %s\n", ZSTD_getErrorName(remaining));
    CHECK(srcPos == srcSize, "MT continue did not ingest all input\n");

    progression = ZSTD_getFrameProgression(cctx);
    CHECK(progression.ingested == srcSize, "MT ingested mismatch\n");
    CHECK(progression.consumed < progression.ingested,
          "MT progression must report buffered or inflight input\n");
    CHECK(progression.currentJobID >= 2U,
          "MT progression must expose started jobs for buffered input\n");
    CHECK(progression.nbActiveWorkers >= 1U && progression.nbActiveWorkers <= 2U,
          "MT active worker accounting mismatch\n");

    dstPos = 0;
    remaining = ZSTD_compressStream2_simpleArgs(
        cctx,
        flushDst,
        sizeof(flushDst),
        &dstPos,
        NULL,
        0,
        NULL,
        ZSTD_e_flush);
    CHECK(!ZSTD_isError(remaining), "MT flush failed: %s\n", ZSTD_getErrorName(remaining));

    progression = ZSTD_getFrameProgression(cctx);
    flushNow = ZSTD_toFlushNow(cctx);
    CHECK(progression.produced > progression.flushed,
          "MT flush should leave produced data pending when dst is tiny\n");
    CHECK(flushNow > 0U, "MT ZSTD_toFlushNow must expose pending bytes\n");

    CHECK_ZSTD(ZSTD_CCtx_refThreadPool(cctx, NULL));
    ZSTD_freeCCtx(cctx);
    ZSTD_freeThreadPool(pool);
    free(src);
    return 0;
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
        CHECK(progression.nbActiveWorkers <= 3U, "unexpected active worker count\n");
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
    ZSTD_threadPool* const invalidPool = ZSTD_createThreadPool(0);
    ZSTD_threadPool* const pool = ZSTD_createThreadPool(3);

    CHECK(sampleA != NULL && sampleB != NULL && pool != NULL, "allocation failure\n");
    CHECK(invalidPool == NULL, "ZSTD_createThreadPool(0) must return NULL\n");

    fill_sample(sampleA, sizeA, 0x1234ABCDU);
    fill_sample(sampleB, sizeB, 0xBEEF1234U);

    if (check_non_mt_progression_contract() ||
        check_mt_progression_contract() ||
        roundtrip_with_pool(pool, sampleA, sizeA, 4, 2) ||
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
