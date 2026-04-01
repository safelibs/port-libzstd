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
        for (i = 0; i < 80U && pos < size; ++i) {
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

static int check_roundtrip(const void* src, size_t srcSize,
                           const void* compressed, size_t compressedSize)
{
    unsigned char* const decoded = (unsigned char*)malloc(srcSize == 0 ? 1 : srcSize);
    size_t const decodedSize = ZSTD_decompress(decoded, srcSize == 0 ? 1 : srcSize,
                                               compressed, compressedSize);
    CHECK(decoded != NULL, "allocation failure\n");
    CHECK(!ZSTD_isError(decodedSize), "ZSTD_decompress failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "decoded payload mismatch\n");
    free(decoded);
    return 0;
}

static size_t failing_sequence_producer(
    void* sequenceProducerState,
    ZSTD_Sequence* outSeqs, size_t outSeqsCapacity,
    const void* src, size_t srcSize,
    const void* dict, size_t dictSize,
    int compressionLevel,
    size_t windowSize)
{
    (void)sequenceProducerState;
    (void)outSeqs;
    (void)src;
    (void)srcSize;
    (void)dict;
    (void)dictSize;
    (void)compressionLevel;
    (void)windowSize;
    return outSeqsCapacity + 1;
}

int main(void)
{
    size_t const srcSize = 256U * 1024U;
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    size_t const seqCapacity = ZSTD_sequenceBound(srcSize);
    ZSTD_Sequence* const seqs = (ZSTD_Sequence*)malloc(seqCapacity * sizeof(*seqs));
    ZSTD_Sequence* const merged = (ZSTD_Sequence*)malloc(seqCapacity * sizeof(*merged));
    void* const compressedA = malloc(ZSTD_compressBound(srcSize));
    void* const compressedB = malloc(ZSTD_compressBound(srcSize));
    void* const compressedC = malloc(ZSTD_compressBound(srcSize));
    size_t generated;
    size_t mergedCount;
    size_t compressedSize;

    CHECK(src != NULL && cctx != NULL && seqs != NULL && merged != NULL &&
              compressedA != NULL && compressedB != NULL && compressedC != NULL,
          "allocation failure\n");

    fill_sample(src, srcSize, 0x5A17C9E3U);

    CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
    generated = ZSTD_generateSequences(cctx, seqs, seqCapacity, src, srcSize);
    CHECK(generated <= seqCapacity, "ZSTD_generateSequences failed\n");
    CHECK(ZSTD_sequenceBound(srcSize) >= generated, "sequence bound contract violated\n");
    CHECK(generated > 0, "ZSTD_generateSequences unexpectedly produced no sequences\n");
    {
        size_t i;
        int hasMatch = 0;
        for (i = 0; i < generated; ++i) {
            if (seqs[i].offset != 0 && seqs[i].matchLength != 0) {
                hasMatch = 1;
                break;
            }
        }
        CHECK(hasMatch, "ZSTD_generateSequences failed to emit match sequences\n");
    }

    compressedSize = ZSTD_compressSequences(cctx, compressedA, ZSTD_compressBound(srcSize),
                                            seqs, generated, src, srcSize);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compressSequences(generated default) failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    CHECK(check_roundtrip(src, srcSize, compressedA, compressedSize) == 0,
          "generated sequence roundtrip failed\n");

    {
        ZSTD_Sequence literalOnly = { 0, (unsigned int)srcSize, 0, 0 };
        size_t literalOnlySize;
        CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
        literalOnlySize = ZSTD_compressSequences(cctx, compressedB, ZSTD_compressBound(srcSize),
                                                 &literalOnly, 1, src, srcSize);
        CHECK(!ZSTD_isError(literalOnlySize),
              "ZSTD_compressSequences(all literals default) failed: %s\n",
              ZSTD_getErrorName(literalOnlySize));
        CHECK(check_roundtrip(src, srcSize, compressedB, literalOnlySize) == 0,
              "default all-literals roundtrip failed\n");
        CHECK(literalOnlySize != compressedSize ||
                  memcmp(compressedA, compressedB, compressedSize) != 0,
              "generated sequences collapsed to the all-literals stream\n");
    }

    CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_blockDelimiters,
                                      ZSTD_sf_explicitBlockDelimiters));
    compressedSize = ZSTD_compressSequences(cctx, compressedA, ZSTD_compressBound(srcSize),
                                            seqs, generated, src, srcSize);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compressSequences(explicit) failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    CHECK(check_roundtrip(src, srcSize, compressedA, compressedSize) == 0,
          "explicit block-delimiter roundtrip failed\n");

    memcpy(merged, seqs, generated * sizeof(*seqs));
    mergedCount = ZSTD_mergeBlockDelimiters(merged, generated);
    CHECK(mergedCount <= generated, "ZSTD_mergeBlockDelimiters failed\n");

    CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_blockDelimiters,
                                      ZSTD_sf_noBlockDelimiters));
    compressedSize = ZSTD_compressSequences(cctx, compressedB, ZSTD_compressBound(srcSize),
                                            merged, mergedCount, src, srcSize);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compressSequences(merged) failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    CHECK(check_roundtrip(src, srcSize, compressedB, compressedSize) == 0,
          "merged sequence roundtrip failed\n");

    {
        size_t const literalSrcSize = 64U * 1024U;
        unsigned char* const literalSrc = (unsigned char*)malloc(literalSrcSize);
        ZSTD_Sequence literalSeq = {0, 0, 0, 0};
        ZSTD_Sequence invalidSeq = {0, 0, 4, 0};
        size_t referenceSize;

        CHECK(literalSrc != NULL, "allocation failure\n");
        fill_sample(literalSrc, literalSrcSize, 0x1A2B3C4DU);
        literalSeq.litLength = (unsigned int)literalSrcSize;
        invalidSeq.offset = (unsigned int)(literalSrcSize + 1U);

        CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
        CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_blockDelimiters,
                                          ZSTD_sf_explicitBlockDelimiters));
        CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_validateSequences, 1));
        compressedSize = ZSTD_compressSequences(cctx, compressedA, ZSTD_compressBound(literalSrcSize),
                                                &literalSeq, 1, literalSrc, literalSrcSize);
        CHECK(!ZSTD_isError(compressedSize), "ZSTD_compressSequences(all literals) failed: %s\n",
              ZSTD_getErrorName(compressedSize));
        CHECK(check_roundtrip(literalSrc, literalSrcSize, compressedA, compressedSize) == 0,
              "all-literals sequence roundtrip failed\n");

        CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
        referenceSize = ZSTD_compress2(cctx, compressedB, ZSTD_compressBound(literalSrcSize),
                                       literalSrc, literalSrcSize);
        CHECK(!ZSTD_isError(referenceSize), "ZSTD_compress2(reference) failed: %s\n",
              ZSTD_getErrorName(referenceSize));
        CHECK(referenceSize != compressedSize ||
                  memcmp(compressedA, compressedB, compressedSize) != 0,
              "all-literals sequence stream should change compression behavior\n");

        CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 4));
        CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_validateSequences, 1));
        compressedSize = ZSTD_compressSequences(cctx, compressedC, ZSTD_compressBound(literalSrcSize),
                                                &invalidSeq, 1, literalSrc, literalSrcSize);
        CHECK(ZSTD_isError(compressedSize),
              "ZSTD_c_validateSequences failed to reject invalid input\n");

        free(literalSrc);
    }

    CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 3));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_enableSeqProducerFallback, 1));
    CHECK_ZSTD(ZSTD_CCtx_setParameter(cctx, ZSTD_c_enableLongDistanceMatching,
                                      ZSTD_ps_disable));
    ZSTD_registerSequenceProducer(cctx, NULL, failing_sequence_producer);
    compressedSize = ZSTD_compress2(cctx, compressedC, ZSTD_compressBound(srcSize), src, srcSize);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compress2 with fallback producer failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    CHECK(check_roundtrip(src, srcSize, compressedC, compressedSize) == 0,
          "external sequence producer fallback roundtrip failed\n");
    ZSTD_registerSequenceProducer(cctx, NULL, NULL);

    ZSTD_freeCCtx(cctx);
    free(compressedC);
    free(compressedB);
    free(compressedA);
    free(merged);
    free(seqs);
    free(src);
    return 0;
}
