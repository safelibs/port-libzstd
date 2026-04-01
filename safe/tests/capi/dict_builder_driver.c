#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#define ZDICT_STATIC_LINKING_ONLY
#include "zdict.h"
#include "zstd.h"

#define CHECK(cond, ...)                             \
    do {                                             \
        if (!(cond)) {                               \
            fprintf(stderr, __VA_ARGS__);            \
            goto cleanup;                            \
        }                                            \
    } while (0)

#define CHECK_ZSTD(expr)                                             \
    do {                                                             \
        size_t const zstd_ret = (expr);                              \
        CHECK(!ZSTD_isError(zstd_ret), "%s: %s\n", #expr,            \
              ZSTD_getErrorName(zstd_ret));                          \
    } while (0)

#define CHECK_ZDICT(expr)                                            \
    do {                                                             \
        size_t const zdict_ret = (expr);                             \
        CHECK(!ZDICT_isError(zdict_ret), "%s: %s\n", #expr,          \
              ZDICT_getErrorName(zdict_ret));                        \
    } while (0)

static void fill_sample(unsigned char* dst, size_t size, unsigned seed)
{
    static const char* const fragments[] = {
        "{\"tenant\":\"alpha\",\"region\":\"west\",\"kind\":\"session\",\"payload\":\"",
        "{\"tenant\":\"beta\",\"region\":\"east\",\"kind\":\"metric\",\"payload\":\"",
        "{\"tenant\":\"gamma\",\"region\":\"north\",\"kind\":\"record\",\"payload\":\"",
    };
    static const char* const tokens[] = {
        "route=/api/v1/items;",
        "route=/api/v1/status;",
        "feature=checkout;",
        "feature=inventory;",
        "status=ok;",
        "status=retry;",
        "bucket=a;",
        "bucket=b;",
    };
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        size_t const fragment = state % (sizeof(fragments) / sizeof(fragments[0]));
        size_t i;
        for (i = 0; fragments[fragment][i] != '\0' && pos < size; ++i) {
            dst[pos++] = (unsigned char)fragments[fragment][i];
        }
        for (i = 0; i < 6U && pos < size; ++i) {
            size_t const token = (state + (unsigned)i) % (sizeof(tokens) / sizeof(tokens[0]));
            size_t j;
            for (j = 0; tokens[token][j] != '\0' && pos < size; ++j) {
                dst[pos++] = (unsigned char)tokens[token][j];
            }
            if (pos < size) {
                dst[pos++] = (unsigned char)('0' + ((state + (unsigned)i) % 10U));
            }
        }
        if (pos < size) {
            dst[pos++] = '"';
        }
        if (pos < size) {
            dst[pos++] = '}';
        }
        if (pos < size) {
            dst[pos++] = '\n';
        }
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
    }
}

int main(void)
{
    enum {
        kSampleSize = 1024,
        kNbSamples = 96,
        kDictCapacity = 8192,
        kRawContentSize = 4096,
        kSrcSize = 192 * 1024
    };

    int rc = 1;
    unsigned char* samples = NULL;
    unsigned char* altSamples = NULL;
    unsigned char* src = NULL;
    unsigned char* decoded = NULL;
    unsigned char* compressed = NULL;
    size_t* sampleSizes = NULL;
    unsigned char dictBuffer[kDictCapacity];
    unsigned char finalDict[kDictCapacity];
    unsigned char coverDict[kDictCapacity];
    unsigned char fastCoverDict[kDictCapacity];
    unsigned char legacyDict[kDictCapacity];
    unsigned char entropyDict[kDictCapacity];
    unsigned char finalDictAlt[kDictCapacity];
    unsigned char entropyDictAlt[kDictCapacity];
    size_t dictSize;
    size_t finalSize;
    size_t coverSize;
    size_t fastCoverSize;
    size_t legacySize;
    size_t entropySize;
    unsigned dictID;
    ZDICT_cover_params_t coverParams = { 64, 8, 4, 1, 1.0, 0, 0, { 3, 0, 0 } };
    ZDICT_cover_params_t coverOpt = { 64, 8, 4, 1, 1.0, 0, 0, { 3, 0, 0 } };
    ZDICT_fastCover_params_t fastParams = { 64, 8, 18, 4, 1, 1.0, 1, 0, 0, { 3, 0, 0 } };
    ZDICT_fastCover_params_t fastOpt = { 64, 8, 18, 4, 1, 1.0, 1, 0, 0, { 3, 0, 0 } };
    ZDICT_legacy_params_t legacyParams = { 0, { 3, 0, 0 } };
    ZDICT_params_t finalizeParams = { 3, 0, 0 };
    ZSTD_customMem customMem = { 0 };
    ZSTD_CCtx_params* cctxParams = NULL;
    ZSTD_CCtx* cctx = NULL;
    ZSTD_DCtx* dctx = NULL;
    ZSTD_CStream* cstream = NULL;
    ZSTD_DStream* dstream = NULL;
    ZSTD_CDict* cdictCopy = NULL;
    ZSTD_CDict* cdictRef = NULL;
    ZSTD_CDict* cdictAdv2 = NULL;
    ZSTD_DDict* ddictCopy = NULL;
    ZSTD_DDict* ddictRef = NULL;
    void* staticCCtxWorkspace = NULL;
    void* staticCStreamWorkspace = NULL;
    void* staticDCtxWorkspace = NULL;
    void* staticDStreamWorkspace = NULL;
    void* staticCDictWorkspace = NULL;
    void* staticDDictWorkspace = NULL;
    ZSTD_CCtx* staticCCtx = NULL;
    ZSTD_CStream* staticCStream = NULL;
    ZSTD_DCtx* staticDCtx = NULL;
    ZSTD_DStream* staticDStream = NULL;
    const ZSTD_CDict* staticCDict = NULL;
    const ZSTD_DDict* staticDDict = NULL;
    ZSTD_compressionParameters cParams;
    ZSTD_frameParameters fParams;
    ZSTD_parameters fullParams;
    size_t cctxEstimate;
    size_t cstreamEstimate;
    size_t dctxEstimate;
    size_t dstreamEstimate;
    size_t cdictEstimate;
    size_t ddictEstimate;
    size_t compressedSize;
    int paramValue = 0;
    union {
        uint64_t align;
        unsigned char bytes[8];
    } tinyWorkspace = { 0 };

    sampleSizes = (size_t*)malloc(kNbSamples * sizeof(*sampleSizes));
    samples = (unsigned char*)malloc(kNbSamples * kSampleSize);
    altSamples = (unsigned char*)malloc(kNbSamples * kSampleSize);
    src = (unsigned char*)malloc(kSrcSize);
    decoded = (unsigned char*)malloc(kSrcSize);
    compressed = (unsigned char*)malloc(ZSTD_compressBound(kSrcSize));
    CHECK(sampleSizes != NULL && samples != NULL && altSamples != NULL && src != NULL &&
              decoded != NULL && compressed != NULL,
          "allocation failure\n");

    for (size_t i = 0; i < kNbSamples; ++i) {
        sampleSizes[i] = kSampleSize;
        fill_sample(samples + (i * kSampleSize), kSampleSize, 0x12340000U + (unsigned)i);
        fill_sample(altSamples + (i * kSampleSize), kSampleSize, 0xDEAD0000U + (unsigned)i);
    }
    fill_sample(src, kSrcSize, 0xBEEF4321U);

    dictSize = ZDICT_trainFromBuffer(dictBuffer, sizeof(dictBuffer), samples, sampleSizes, kNbSamples);
    CHECK(!ZDICT_isError(dictSize) && dictSize != 0, "ZDICT_trainFromBuffer failed: %s\n",
          ZDICT_getErrorName(dictSize));
    CHECK(ZDICT_isError(0) == 0, "ZDICT_isError(0) should be false\n");
    CHECK(ZDICT_getErrorName(0) != NULL, "ZDICT_getErrorName(0) returned NULL\n");
    CHECK(ZDICT_getDictHeaderSize(dictBuffer, dictSize) > 0, "trained dictionary has no header\n");
    dictID = ZDICT_getDictID(dictBuffer, dictSize);
    CHECK(dictID != 0, "trained dictionary has no dict id\n");

    CHECK_ZDICT(ZDICT_finalizeDictionary(finalDict, sizeof(finalDict),
                                         samples + (kNbSamples * kSampleSize) - kRawContentSize,
                                         kRawContentSize,
                                         samples, sampleSizes, kNbSamples,
                                         finalizeParams));
    finalSize = ZDICT_finalizeDictionary(finalDict, sizeof(finalDict),
                                         samples + (kNbSamples * kSampleSize) - kRawContentSize,
                                         kRawContentSize,
                                         samples, sampleSizes, kNbSamples,
                                         finalizeParams);
    CHECK(!ZDICT_isError(finalSize) && finalSize != 0, "finalized dictionary is empty\n");
    {
        size_t const altFinalSize = ZDICT_finalizeDictionary(
            finalDictAlt, sizeof(finalDictAlt),
            samples + (kNbSamples * kSampleSize) - kRawContentSize,
            kRawContentSize,
            altSamples, sampleSizes, kNbSamples,
            finalizeParams);
        CHECK(!ZDICT_isError(altFinalSize) && altFinalSize != 0,
              "alternate finalized dictionary is empty\n");
        CHECK(altFinalSize != finalSize ||
                  memcmp(finalDict, finalDictAlt, finalSize) != 0,
              "ZDICT_finalizeDictionary ignored the sample corpus\n");
    }

    coverSize = ZDICT_trainFromBuffer_cover(coverDict, sizeof(coverDict),
                                            samples, sampleSizes, kNbSamples, coverParams);
    CHECK(!ZDICT_isError(coverSize) && coverSize != 0, "ZDICT_trainFromBuffer_cover failed: %s\n",
          ZDICT_getErrorName(coverSize));
    CHECK_ZDICT(ZDICT_optimizeTrainFromBuffer_cover(coverDict, sizeof(coverDict),
                                                    samples, sampleSizes, kNbSamples, &coverOpt));
    CHECK(coverOpt.k >= coverOpt.d && coverOpt.k != 0 && coverOpt.d != 0,
          "cover optimizer returned invalid parameters\n");

    fastCoverSize = ZDICT_trainFromBuffer_fastCover(fastCoverDict, sizeof(fastCoverDict),
                                                    samples, sampleSizes, kNbSamples, fastParams);
    CHECK(!ZDICT_isError(fastCoverSize) && fastCoverSize != 0,
          "ZDICT_trainFromBuffer_fastCover failed: %s\n",
          ZDICT_getErrorName(fastCoverSize));
    CHECK_ZDICT(ZDICT_optimizeTrainFromBuffer_fastCover(fastCoverDict, sizeof(fastCoverDict),
                                                        samples, sampleSizes, kNbSamples, &fastOpt));
    CHECK(fastOpt.k >= fastOpt.d && fastOpt.k != 0 && fastOpt.d != 0 && fastOpt.f != 0,
          "fastCover optimizer returned invalid parameters\n");

    legacySize = ZDICT_trainFromBuffer_legacy(legacyDict, sizeof(legacyDict),
                                              samples, sampleSizes, kNbSamples, legacyParams);
    CHECK(!ZDICT_isError(legacySize) && legacySize != 0, "ZDICT_trainFromBuffer_legacy failed: %s\n",
          ZDICT_getErrorName(legacySize));

    memcpy(entropyDict,
           samples + (kNbSamples * kSampleSize) - kRawContentSize,
           kRawContentSize);
    entropySize = ZDICT_addEntropyTablesFromBuffer(entropyDict, kRawContentSize,
                                                   sizeof(entropyDict),
                                                   samples, sampleSizes, kNbSamples);
    CHECK(!ZDICT_isError(entropySize) && entropySize >= kRawContentSize,
          "ZDICT_addEntropyTablesFromBuffer failed: %s\n",
          ZDICT_getErrorName(entropySize));
    memcpy(entropyDictAlt,
           samples + (kNbSamples * kSampleSize) - kRawContentSize,
           kRawContentSize);
    {
        size_t const altEntropySize = ZDICT_addEntropyTablesFromBuffer(
            entropyDictAlt, kRawContentSize, sizeof(entropyDictAlt),
            altSamples, sampleSizes, kNbSamples);
        CHECK(!ZDICT_isError(altEntropySize) && altEntropySize >= kRawContentSize,
              "alternate ZDICT_addEntropyTablesFromBuffer failed: %s\n",
              ZDICT_getErrorName(altEntropySize));
        CHECK(altEntropySize != entropySize ||
                  memcmp(entropyDict, entropyDictAlt, entropySize) != 0,
              "ZDICT_addEntropyTablesFromBuffer ignored the sample corpus\n");
    }

    cctxParams = ZSTD_createCCtxParams();
    cctx = ZSTD_createCCtx_advanced(customMem);
    dctx = ZSTD_createDCtx_advanced(customMem);
    cstream = ZSTD_createCStream_advanced(customMem);
    dstream = ZSTD_createDStream_advanced(customMem);
    CHECK(cctxParams != NULL && cctx != NULL && dctx != NULL &&
              cstream != NULL && dstream != NULL,
          "advanced context creation failed\n");

    CHECK_ZSTD(ZSTD_CCtxParams_init(cctxParams, 4));
    CHECK_ZSTD(ZSTD_CCtxParams_setParameter(cctxParams, ZSTD_c_windowLog, 20));
    CHECK_ZSTD(ZSTD_CCtxParams_getParameter(cctxParams, ZSTD_c_windowLog, &paramValue));
    CHECK(paramValue == 20, "unexpected windowLog parameter value\n");

    cParams = ZSTD_getCParams(4, kSrcSize, dictSize);
    fParams.contentSizeFlag = 1;
    fParams.checksumFlag = 1;
    fParams.noDictIDFlag = 0;
    fullParams.cParams = cParams;
    fullParams.fParams = fParams;
    CHECK_ZSTD(ZSTD_CCtxParams_init_advanced(cctxParams, fullParams));
    CHECK_ZSTD(ZSTD_CCtxParams_reset(cctxParams));
    CHECK_ZSTD(ZSTD_CCtxParams_init(cctxParams, 4));
    CHECK_ZSTD(ZSTD_CCtxParams_setParameter(cctxParams, ZSTD_c_windowLog, 20));

    cctxEstimate = ZSTD_estimateCCtxSize(4);
    CHECK(!ZSTD_isError(cctxEstimate) && cctxEstimate > 0, "ZSTD_estimateCCtxSize failed\n");
    CHECK(!ZSTD_isError(ZSTD_estimateCCtxSize_usingCParams(cParams)),
          "ZSTD_estimateCCtxSize_usingCParams failed\n");
    CHECK(!ZSTD_isError(ZSTD_estimateCCtxSize_usingCCtxParams(cctxParams)),
          "ZSTD_estimateCCtxSize_usingCCtxParams failed\n");
    cstreamEstimate = ZSTD_estimateCStreamSize_usingCCtxParams(cctxParams);
    CHECK(!ZSTD_isError(cstreamEstimate) && cstreamEstimate > 0,
          "ZSTD_estimateCStreamSize_usingCCtxParams failed\n");
    CHECK(!ZSTD_isError(ZSTD_estimateCStreamSize(4)),
          "ZSTD_estimateCStreamSize failed\n");
    CHECK(!ZSTD_isError(ZSTD_estimateCStreamSize_usingCParams(cParams)),
          "ZSTD_estimateCStreamSize_usingCParams failed\n");
    dctxEstimate = ZSTD_estimateDCtxSize();
    CHECK(!ZSTD_isError(dctxEstimate) && dctxEstimate > 0, "ZSTD_estimateDCtxSize failed\n");
    dstreamEstimate = ZSTD_estimateDStreamSize((size_t)1U << 20);
    CHECK(!ZSTD_isError(dstreamEstimate) && dstreamEstimate > 0,
          "ZSTD_estimateDStreamSize failed\n");
    cdictEstimate = ZSTD_estimateCDictSize_advanced(dictSize, cParams, ZSTD_dlm_byCopy);
    CHECK(!ZSTD_isError(cdictEstimate) && cdictEstimate > 0,
          "ZSTD_estimateCDictSize_advanced failed\n");
    CHECK(!ZSTD_isError(ZSTD_estimateCDictSize(dictSize, 4)),
          "ZSTD_estimateCDictSize failed\n");
    ddictEstimate = ZSTD_estimateDDictSize(dictSize, ZSTD_dlm_byCopy);
    CHECK(!ZSTD_isError(ddictEstimate) && ddictEstimate > 0,
          "ZSTD_estimateDDictSize failed\n");
    CHECK(ZSTD_estimateDDictSize(dictSize, ZSTD_dlm_byRef) <= ddictEstimate,
          "ZSTD_estimateDDictSize by-reference should not exceed by-copy estimate\n");

    staticCCtxWorkspace = malloc(cctxEstimate);
    staticCStreamWorkspace = malloc(cstreamEstimate);
    staticDCtxWorkspace = malloc(dctxEstimate);
    staticDStreamWorkspace = malloc(dstreamEstimate);
    staticCDictWorkspace = malloc(cdictEstimate);
    staticDDictWorkspace = malloc(ddictEstimate);
    CHECK(staticCCtxWorkspace != NULL && staticCStreamWorkspace != NULL &&
              staticDCtxWorkspace != NULL && staticDStreamWorkspace != NULL &&
              staticCDictWorkspace != NULL && staticDDictWorkspace != NULL,
          "static workspace allocation failure\n");

    staticCCtx = ZSTD_initStaticCCtx(staticCCtxWorkspace, cctxEstimate);
    staticCStream = ZSTD_initStaticCStream(staticCStreamWorkspace, cstreamEstimate);
    staticDCtx = ZSTD_initStaticDCtx(staticDCtxWorkspace, dctxEstimate);
    staticDStream = ZSTD_initStaticDStream(staticDStreamWorkspace, dstreamEstimate);
    staticCDict = ZSTD_initStaticCDict(staticCDictWorkspace, cdictEstimate,
                                       dictBuffer, dictSize,
                                       ZSTD_dlm_byCopy, ZSTD_dct_fullDict, cParams);
    staticDDict = ZSTD_initStaticDDict(staticDDictWorkspace, ddictEstimate,
                                       dictBuffer, dictSize,
                                       ZSTD_dlm_byCopy, ZSTD_dct_fullDict);
    CHECK(ZSTD_initStaticCCtx(tinyWorkspace.bytes, sizeof(tinyWorkspace.bytes)) == NULL,
          "ZSTD_initStaticCCtx accepted an undersized workspace\n");
    CHECK(ZSTD_initStaticCStream(tinyWorkspace.bytes, sizeof(tinyWorkspace.bytes)) == NULL,
          "ZSTD_initStaticCStream accepted an undersized workspace\n");
    CHECK(ZSTD_initStaticDCtx(tinyWorkspace.bytes, sizeof(tinyWorkspace.bytes)) == NULL,
          "ZSTD_initStaticDCtx accepted an undersized workspace\n");
    CHECK(ZSTD_initStaticDStream(tinyWorkspace.bytes, sizeof(tinyWorkspace.bytes)) == NULL,
          "ZSTD_initStaticDStream accepted an undersized workspace\n");
    CHECK(ZSTD_initStaticCDict(tinyWorkspace.bytes, sizeof(tinyWorkspace.bytes),
                               dictBuffer, dictSize,
                               ZSTD_dlm_byCopy, ZSTD_dct_fullDict, cParams) == NULL,
          "ZSTD_initStaticCDict accepted an undersized workspace\n");
    CHECK(ZSTD_initStaticDDict(tinyWorkspace.bytes, sizeof(tinyWorkspace.bytes),
                               dictBuffer, dictSize,
                               ZSTD_dlm_byCopy, ZSTD_dct_fullDict) == NULL,
          "ZSTD_initStaticDDict accepted an undersized workspace\n");
    CHECK(staticCCtx != NULL || staticCStream != NULL || staticDCtx != NULL ||
              staticDStream != NULL || staticCDict != NULL || staticDDict != NULL,
          "all static context initialization calls failed\n");
    if (staticCCtx != NULL) {
        CHECK(ZSTD_isError(ZSTD_freeCCtx(staticCCtx)),
              "ZSTD_freeCCtx unexpectedly accepted a static context\n");
    }
    if (staticDCtx != NULL) {
        CHECK(ZSTD_isError(ZSTD_freeDCtx(staticDCtx)),
              "ZSTD_freeDCtx unexpectedly accepted a static context\n");
    }

    cdictCopy = ZSTD_createCDict_advanced(dictBuffer, dictSize,
                                          ZSTD_dlm_byCopy, ZSTD_dct_fullDict,
                                          cParams, customMem);
    cdictRef = ZSTD_createCDict_byReference(dictBuffer, dictSize, 4);
    cdictAdv2 = ZSTD_createCDict_advanced2(dictBuffer, dictSize,
                                           ZSTD_dlm_byRef, ZSTD_dct_fullDict,
                                           cctxParams, customMem);
    ddictCopy = ZSTD_createDDict_advanced(dictBuffer, dictSize,
                                          ZSTD_dlm_byCopy, ZSTD_dct_fullDict,
                                          customMem);
    ddictRef = ZSTD_createDDict_byReference(dictBuffer, dictSize);
    CHECK(cdictCopy != NULL && cdictRef != NULL && cdictAdv2 != NULL &&
              ddictCopy != NULL && ddictRef != NULL,
          "advanced dictionary creation failed\n");
    CHECK(ZSTD_getDictID_fromCDict(cdictCopy) == dictID,
          "advanced cdict lost the trained dictionary id\n");

    CHECK_ZSTD(ZSTD_CCtx_setParametersUsingCCtxParams(cctx, cctxParams));
    CHECK_ZSTD(ZSTD_CCtx_setCParams(cctx, cParams));
    CHECK_ZSTD(ZSTD_CCtx_setFParams(cctx, fParams));
    CHECK_ZSTD(ZSTD_CCtx_setParams(cctx, fullParams));
    CHECK_ZSTD(ZSTD_CCtx_loadDictionary_byReference(cctx, finalDict, finalSize));
    CHECK_ZSTD(ZSTD_CCtx_loadDictionary_advanced(cctx, finalDict, finalSize,
                                                 ZSTD_dlm_byCopy, ZSTD_dct_fullDict));
    CHECK_ZSTD(ZSTD_CCtx_refPrefix_advanced(cctx, samples, kRawContentSize,
                                            ZSTD_dct_rawContent));

    CHECK_ZSTD(ZSTD_DCtx_loadDictionary_byReference(dctx, finalDict, finalSize));
    CHECK_ZSTD(ZSTD_DCtx_loadDictionary_advanced(dctx, finalDict, finalSize,
                                                 ZSTD_dlm_byCopy, ZSTD_dct_fullDict));
    CHECK_ZSTD(ZSTD_DCtx_refPrefix_advanced(dctx, samples, kRawContentSize,
                                            ZSTD_dct_rawContent));

    CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_ZSTD(ZSTD_CCtx_setParams(cctx, fullParams));
    compressedSize = ZSTD_compress_advanced(cctx, compressed, ZSTD_compressBound(kSrcSize),
                                            src, kSrcSize, NULL, 0, fullParams);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compress_advanced failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    CHECK(!ZSTD_isError(ZSTD_estimateDStreamSize_fromFrame(compressed, compressedSize)),
          "ZSTD_estimateDStreamSize_fromFrame failed\n");
    CHECK_ZSTD(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    CHECK_ZSTD(ZSTD_DCtx_loadDictionary_advanced(dctx, NULL, 0,
                                                 ZSTD_dlm_byCopy, ZSTD_dct_auto));
    {
        size_t const decodedSize = ZSTD_decompressDCtx(dctx, decoded, kSrcSize,
                                                       compressed, compressedSize);
        CHECK(!ZSTD_isError(decodedSize), "advanced decompression failed: %s\n",
              ZSTD_getErrorName(decodedSize));
        CHECK(decodedSize == kSrcSize, "advanced decoded size mismatch\n");
        CHECK(memcmp(decoded, src, kSrcSize) == 0, "advanced decoded payload mismatch\n");
    }

    CHECK_ZSTD(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    compressedSize = ZSTD_compress_usingCDict_advanced(cctx, compressed,
                                                       ZSTD_compressBound(kSrcSize),
                                                       src, kSrcSize,
                                                       cdictCopy, fParams);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compress_usingCDict_advanced failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    CHECK_ZSTD(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    CHECK_ZSTD(ZSTD_DCtx_refDDict(dctx, ddictCopy));
    {
        size_t const decodedSize = ZSTD_decompressDCtx(dctx, decoded, kSrcSize,
                                                       compressed, compressedSize);
        CHECK(!ZSTD_isError(decodedSize), "cdict advanced decompression failed: %s\n",
              ZSTD_getErrorName(decodedSize));
        CHECK(decodedSize == kSrcSize, "cdict advanced decoded size mismatch\n");
        CHECK(memcmp(decoded, src, kSrcSize) == 0, "cdict advanced decoded payload mismatch\n");
    }

    rc = 0;

cleanup:
    ZSTD_freeDDict(ddictRef);
    ZSTD_freeDDict(ddictCopy);
    ZSTD_freeCDict(cdictAdv2);
    ZSTD_freeCDict(cdictRef);
    ZSTD_freeCDict(cdictCopy);
    ZSTD_freeDStream(dstream);
    ZSTD_freeCStream(cstream);
    ZSTD_freeDCtx(dctx);
    ZSTD_freeCCtx(cctx);
    ZSTD_freeCCtxParams(cctxParams);
    free(staticDDictWorkspace);
    free(staticCDictWorkspace);
    free(staticDStreamWorkspace);
    free(staticDCtxWorkspace);
    free(staticCStreamWorkspace);
    free(staticCCtxWorkspace);
    free(compressed);
    free(decoded);
    free(src);
    free(altSamples);
    free(samples);
    free(sampleSizes);
    return rc;
}
