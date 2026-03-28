/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "zstd.h"
#include "zstd_errors.h"

#define DISPLAY(...) fprintf(stderr, __VA_ARGS__)
#define ARRAY_SIZE(a) (sizeof(a) / sizeof((a)[0]))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define SKIPPABLE_FRAME_HEADER_SIZE 8U

#define CHECK_Z(value)                                                       \
    do {                                                                     \
        size_t const check_z_result = (value);                               \
        if (ZSTD_isError(check_z_result)) {                                  \
            DISPLAY("%s: %s\n", #value, ZSTD_getErrorName(check_z_result));  \
            return 1;                                                        \
        }                                                                    \
    } while (0)

#define CHECK(cond, ...)                 \
    do {                                 \
        if (!(cond)) {                   \
            DISPLAY(__VA_ARGS__);        \
            return 1;                    \
        }                                \
    } while (0)

typedef struct {
    unsigned verbose;
    unsigned durationSeconds;
    unsigned maxWorkers;
    unsigned seed;
} options_t;

static unsigned readU32FromChar(const char** stringPtr)
{
    unsigned result = 0;
    while ((**stringPtr >= '0') && (**stringPtr <= '9')) {
        result *= 10;
        result += (unsigned)(**stringPtr - '0');
        (*stringPtr)++;
    }
    if ((**stringPtr == 'm') && ((*stringPtr)[1] == 'n')) {
        result *= 60;
        *stringPtr += 2;
    } else if ((**stringPtr == 's') || (**stringPtr == 'S')) {
        (*stringPtr)++;
    }
    return result;
}

static unsigned nextRandom(unsigned* state)
{
    unsigned value = *state;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    *state = value ? value : 1U;
    return *state;
}

static void generateSample(void* buffer, size_t size, unsigned seed)
{
    unsigned char* out = (unsigned char*)buffer;
    size_t i;
    for (i = 0; i < size; ++i) {
        unsigned const rnd = nextRandom(&seed);
        if (i > 0 && (rnd % 100U) < 72U) {
            size_t const back = (rnd % MIN(i, (size_t)32768U)) + 1;
            out[i] = out[i - back];
        } else {
            out[i] = (unsigned char)rnd;
        }
    }
}

static int checkErrorCode(size_t code, ZSTD_ErrorCode expected, const char* action)
{
    CHECK(ZSTD_isError(code), "%s unexpectedly succeeded\n", action);
    CHECK(ZSTD_getErrorCode(code) == expected,
          "%s returned %s instead of %s\n",
          action,
          ZSTD_getErrorName(code),
          ZSTD_getErrorString(expected));
    return 0;
}

static void writeLE32(unsigned char* dst, unsigned value)
{
    dst[0] = (unsigned char)(value & 0xFFU);
    dst[1] = (unsigned char)((value >> 8) & 0xFFU);
    dst[2] = (unsigned char)((value >> 16) & 0xFFU);
    dst[3] = (unsigned char)((value >> 24) & 0xFFU);
}

static int compressOneShot(const void* src,
                           size_t srcSize,
                           int level,
                           unsigned workers,
                           int checksumFlag,
                           int contentSizeFlag,
                           const void* dict,
                           size_t dictSize,
                           void* compressed,
                           size_t compressedCapacity,
                           size_t* compressedSize)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    if (cctx == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, level));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, checksumFlag));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, contentSizeFlag));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_dictIDFlag, 1));
    if (dict != NULL && dictSize != 0) {
        CHECK_Z(ZSTD_CCtx_loadDictionary(cctx, dict, dictSize));
    }

    *compressedSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
    if (ZSTD_isError(*compressedSize)) {
        DISPLAY("ZSTD_compress2: %s\n", ZSTD_getErrorName(*compressedSize));
        ZSTD_freeCCtx(cctx);
        return 1;
    }

    ZSTD_freeCCtx(cctx);
    return 0;
}

static int testStaticHelpers(void)
{
    static const size_t testSizes[] = {
        0U, 1U, 15U, 1024U, 65536U, 131072U, 524288U, 1048576U
    };
    ZSTD_bounds const levelBounds = ZSTD_cParam_getBounds(ZSTD_c_compressionLevel);
    size_t i;

    CHECK(ZSTD_getErrorName(0) != NULL, "ZSTD_getErrorName(0) returned NULL\n");
    CHECK(ZSTD_getErrorName((size_t)499) != NULL, "ZSTD_getErrorName(499) returned NULL\n");
    CHECK(ZSTD_versionNumber() != 0, "ZSTD_versionNumber() returned 0\n");
    CHECK(ZSTD_defaultCLevel() == ZSTD_CLEVEL_DEFAULT,
          "default compression level mismatch: %d != %d\n",
          ZSTD_defaultCLevel(), ZSTD_CLEVEL_DEFAULT);
    CHECK(!ZSTD_isError(levelBounds.error),
          "compression level bounds query failed\n");
    CHECK(levelBounds.lowerBound < levelBounds.upperBound,
          "compression level bounds are inconsistent: %d >= %d\n",
          levelBounds.lowerBound, levelBounds.upperBound);
    CHECK(ZSTD_defaultCLevel() >= levelBounds.lowerBound &&
          ZSTD_defaultCLevel() <= levelBounds.upperBound,
          "default compression level is outside reported bounds\n");

    for (i = 0; i < ARRAY_SIZE(testSizes); ++i) {
        size_t const size = testSizes[i];
        CHECK(ZSTD_compressBound(size) == ZSTD_COMPRESSBOUND(size),
              "compressBound mismatch for %zu\n", size);
    }
    {
        size_t const tooLarge = (size_t)ZSTD_MAX_INPUT_SIZE + 1U;
        CHECK(ZSTD_isError(ZSTD_compressBound(tooLarge)),
              "compressBound unexpectedly accepted oversized input\n");
    }

    return 0;
}

static int testParameterBounds(void)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    ZSTD_bounds const strategyBounds = ZSTD_cParam_getBounds(ZSTD_c_strategy);
    size_t code;

    if (cctx == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    CHECK(!ZSTD_isError(strategyBounds.error), "strategy bounds query failed\n");
    code = ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, strategyBounds.upperBound + 1);
    if (checkErrorCode(code, ZSTD_error_parameter_outOfBound, "strategy upper bound")) {
        ZSTD_freeCCtx(cctx);
        return 1;
    }

    ZSTD_freeCCtx(cctx);
    return 0;
}

static int testInvalidEndDirective(const void* src, size_t srcSize)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const compressed = (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    ZSTD_inBuffer in = { src, srcSize, 0 };
    ZSTD_outBuffer out;
    size_t code;

    if (cctx == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    out.dst = compressed;
    out.size = ZSTD_compressBound(srcSize);
    out.pos = 0;
    code = ZSTD_compressStream2(cctx, &out, &in, (ZSTD_EndDirective)3);
    CHECK(ZSTD_isError(code), "invalid end directive unexpectedly succeeded\n");
    if (ZSTD_getErrorCode(code) != ZSTD_error_parameter_unsupported &&
        ZSTD_getErrorCode(code) != ZSTD_error_parameter_outOfBound) {
        DISPLAY("invalid end directive returned %s\n", ZSTD_getErrorName(code));
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    ZSTD_freeCCtx(cctx);
    free(compressed);
    return 0;
}

static int testFrameHelpers(const void* src,
                            size_t srcSize,
                            int level,
                            unsigned workers)
{
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize) + 256U);
    unsigned char* const decoded =
        (unsigned char*)malloc(srcSize + 64U);
    unsigned char skipPayload[37];
    unsigned char* series = NULL;
    size_t compressedSize = 0;
    size_t decodedSize;
    size_t skipFrameSize;
    size_t seriesSize;
    size_t i;

    if (dctx == NULL || compressed == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    if (compressOneShot(src, srcSize, level, workers, 1, 1,
                        NULL, 0, compressed, ZSTD_compressBound(srcSize) + 256U,
                        &compressedSize)) {
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    CHECK(ZSTD_getFrameContentSize(compressed, compressedSize) == (unsigned long long)srcSize,
          "ZSTD_getFrameContentSize mismatch\n");
    CHECK(ZSTD_getDictID_fromFrame(compressed, compressedSize) == 0,
          "unexpected dictionary ID in complete frame\n");
    CHECK(ZSTD_findFrameCompressedSize(compressed, compressedSize) == compressedSize,
          "ZSTD_findFrameCompressedSize mismatch\n");

    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize);
    CHECK(!ZSTD_isError(decodedSize), "ZSTD_decompressDCtx failed: %s\n", ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "decompressed size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "one-shot round-trip mismatch\n");

    for (i = 0; i < sizeof(skipPayload); ++i) {
        skipPayload[i] = (unsigned char)(0x30U + i);
    }
    writeLE32(compressed + compressedSize, ZSTD_MAGIC_SKIPPABLE_START + 3U);
    writeLE32(compressed + compressedSize + 4U, (unsigned)sizeof(skipPayload));
    memcpy(compressed + compressedSize + SKIPPABLE_FRAME_HEADER_SIZE, skipPayload, sizeof(skipPayload));
    skipFrameSize = SKIPPABLE_FRAME_HEADER_SIZE + sizeof(skipPayload);
    CHECK(ZSTD_findFrameCompressedSize(compressed + compressedSize, skipFrameSize) == skipFrameSize,
          "skippable frame compressed size mismatch\n");
    decodedSize = ZSTD_decompress(decoded, 0, compressed + compressedSize, skipFrameSize);
    CHECK(!ZSTD_isError(decodedSize), "skippable ZSTD_decompress failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == 0, "skippable frame should decode to zero bytes\n");

    seriesSize = compressedSize + skipFrameSize;
    series = (unsigned char*)malloc(seriesSize);
    if (series == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }
    memcpy(series, compressed, compressedSize);
    memcpy(series + compressedSize, compressed + compressedSize, skipFrameSize);

    decodedSize = ZSTD_decompress(decoded, srcSize, series, seriesSize);
    CHECK(!ZSTD_isError(decodedSize), "multi-frame ZSTD_decompress failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "multi-frame decompressed size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "multi-frame round-trip mismatch\n");

    free(series);
    ZSTD_freeDCtx(dctx);
    free(compressed);
    free(decoded);
    return 0;
}

static int testErrorPaths(const void* src, size_t srcSize, int level)
{
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize) + 1U);
    unsigned char* const decoded = (unsigned char*)malloc(srcSize);
    size_t const compressedCapacity = ZSTD_compressBound(srcSize) + 1U;
    size_t compressedSize;

    if (compressed == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        free(compressed);
        free(decoded);
        return 1;
    }

    compressedSize = ZSTD_compress(compressed, compressedCapacity, src, srcSize, level);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compress failed: %s\n", ZSTD_getErrorName(compressedSize));
    compressed[compressedSize] = 0;

    if (checkErrorCode(ZSTD_decompress(decoded, srcSize, compressed, 3),
                       ZSTD_error_srcSize_wrong,
                       "short input")) {
        free(compressed);
        free(decoded);
        return 1;
    }
    if (checkErrorCode(ZSTD_decompress(decoded, srcSize, compressed, compressedSize - 1U),
                       ZSTD_error_srcSize_wrong,
                       "truncated frame")) {
        free(compressed);
        free(decoded);
        return 1;
    }
    if (checkErrorCode(ZSTD_decompress(decoded, srcSize, compressed, compressedSize + 1U),
                       ZSTD_error_srcSize_wrong,
                       "extra input byte")) {
        free(compressed);
        free(decoded);
        return 1;
    }
    if (checkErrorCode(ZSTD_decompress(decoded, srcSize - 1U, compressed, compressedSize),
                       ZSTD_error_dstSize_tooSmall,
                       "too small destination")) {
        free(compressed);
        free(decoded);
        return 1;
    }
    if (checkErrorCode(ZSTD_decompress(NULL, 0, compressed, compressedSize),
                       ZSTD_error_dstSize_tooSmall,
                       "NULL destination")) {
        free(compressed);
        free(decoded);
        return 1;
    }
    CHECK(ZSTD_isError(ZSTD_findFrameCompressedSize(compressed, compressedSize - 1U)),
          "truncated ZSTD_findFrameCompressedSize unexpectedly succeeded\n");
    CHECK(ZSTD_getFrameContentSize(compressed, 1U) == ZSTD_CONTENTSIZE_ERROR,
          "short ZSTD_getFrameContentSize unexpectedly succeeded\n");

    free(compressed);
    free(decoded);
    return 0;
}

static int testChecksumMismatch(const void* src, size_t srcSize, int level)
{
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    unsigned char* const decoded = (unsigned char*)malloc(srcSize);
    size_t compressedSize = 0;

    if (compressed == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        free(compressed);
        free(decoded);
        return 1;
    }

    if (compressOneShot(src, srcSize, level, 0, 1, 1,
                        NULL, 0, compressed, ZSTD_compressBound(srcSize),
                        &compressedSize)) {
        free(compressed);
        free(decoded);
        return 1;
    }

    compressed[compressedSize - 1U] ^= 1U;
    if (checkErrorCode(ZSTD_decompress(decoded, srcSize, compressed, compressedSize),
                       ZSTD_error_checksum_wrong,
                       "checksum mismatch")) {
        free(compressed);
        free(decoded);
        return 1;
    }

    free(compressed);
    free(decoded);
    return 0;
}

static int testPrefixDictionary(int level, unsigned workers)
{
    size_t const dictSize = 16384U;
    size_t const srcSize = dictSize * 2U;
    unsigned char* const dict = (unsigned char*)malloc(dictSize);
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    unsigned char* const compressed = (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    unsigned char* const decoded = (unsigned char*)malloc(srcSize);
    ZSTD_CCtx* cctx = NULL;
    ZSTD_DCtx* dctx = NULL;
    ZSTD_DCtx* prefixDctx = NULL;
    size_t compressedSize;
    size_t plainSize;

    if (dict == NULL || src == NULL || compressed == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        free(dict);
        free(src);
        free(compressed);
        free(decoded);
        return 1;
    }

    generateSample(dict, dictSize, 0xA51CE11U);
    memcpy(src, dict, dictSize);
    memcpy(src + dictSize, dict, dictSize);
    src[dictSize / 3U] ^= 0x1BU;
    src[dictSize + dictSize / 5U] ^= 0x2DU;

    cctx = ZSTD_createCCtx();
    dctx = ZSTD_createDCtx();
    prefixDctx = ZSTD_createDCtx();
    if (cctx == NULL || dctx == NULL || prefixDctx == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        ZSTD_freeDCtx(prefixDctx);
        free(dict);
        free(src);
        free(compressed);
        free(decoded);
        return 1;
    }

    plainSize = ZSTD_compress(compressed, ZSTD_compressBound(srcSize), src, srcSize, level);
    CHECK(!ZSTD_isError(plainSize), "plain ZSTD_compress failed: %s\n", ZSTD_getErrorName(plainSize));

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, level));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_refPrefix(cctx, dict, dictSize));
    compressedSize = ZSTD_compress2(cctx, compressed, ZSTD_compressBound(srcSize), src, srcSize);
    CHECK(!ZSTD_isError(compressedSize), "prefix ZSTD_compress2 failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    CHECK(compressedSize < plainSize, "prefix compression did not improve size\n");

    {
        size_t const prefixless = ZSTD_decompress_usingDict(dctx, decoded, srcSize,
                                                            compressed, compressedSize,
                                                            NULL, 0);
        CHECK(ZSTD_isError(prefixless), "prefix decode without prefix unexpectedly succeeded\n");
        if (ZSTD_getErrorCode(prefixless) != ZSTD_error_dictionary_wrong &&
            ZSTD_getErrorCode(prefixless) != ZSTD_error_corruption_detected) {
            DISPLAY("prefix decode without prefix returned %s\n", ZSTD_getErrorName(prefixless));
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            ZSTD_freeDCtx(prefixDctx);
            free(dict);
            free(src);
            free(compressed);
            free(decoded);
            return 1;
        }
    }

    CHECK_Z(ZSTD_DCtx_refPrefix(prefixDctx, dict, dictSize));
    CHECK_Z(ZSTD_DCtx_reset(prefixDctx, ZSTD_reset_session_only));
    CHECK_Z(ZSTD_DCtx_refPrefix(prefixDctx, dict, dictSize));
    plainSize = ZSTD_decompressDCtx(prefixDctx, decoded, srcSize, compressed, compressedSize);
    CHECK(!ZSTD_isError(plainSize), "prefix ZSTD_decompressDCtx failed: %s\n",
          ZSTD_getErrorName(plainSize));
    CHECK(plainSize == srcSize, "prefix decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "prefix round-trip mismatch\n");

    {
        size_t const reused = ZSTD_decompressDCtx(prefixDctx, decoded, srcSize, compressed, compressedSize);
        CHECK(ZSTD_isError(reused), "prefix reuse without reload unexpectedly succeeded\n");
        if (ZSTD_getErrorCode(reused) != ZSTD_error_dictionary_wrong &&
            ZSTD_getErrorCode(reused) != ZSTD_error_corruption_detected) {
            DISPLAY("prefix reuse without reload returned %s\n", ZSTD_getErrorName(reused));
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            ZSTD_freeDCtx(prefixDctx);
            free(dict);
            free(src);
            free(compressed);
            free(decoded);
            return 1;
        }
    }

    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    ZSTD_freeDCtx(prefixDctx);
    free(dict);
    free(src);
    free(compressed);
    free(decoded);
    return 0;
}

static int testMissingContentSize(const void* src, size_t srcSize, int level)
{
    unsigned char* const compressed = (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    unsigned char* const decoded = (unsigned char*)malloc(srcSize);
    size_t compressedSize = 0;
    size_t decodedSize;
    unsigned long long const unknown = ZSTD_CONTENTSIZE_UNKNOWN;

    if (compressed == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        free(compressed);
        free(decoded);
        return 1;
    }

    if (compressOneShot(src, srcSize, level, 0, 0, 0,
                        NULL, 0, compressed, ZSTD_compressBound(srcSize),
                        &compressedSize)) {
        free(compressed);
        free(decoded);
        return 1;
    }

    CHECK(ZSTD_getFrameContentSize(compressed, compressedSize) == unknown,
          "frame content size should be unknown\n");
    CHECK(ZSTD_getDictID_fromFrame(compressed, compressedSize) == 0,
          "unexpected dictionary ID in hidden-size frame\n");
    decodedSize = ZSTD_decompress(decoded, srcSize, compressed, compressedSize);
    CHECK(!ZSTD_isError(decodedSize), "decompress failed for size-hidden frame: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "size-hidden decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "size-hidden round-trip mismatch\n");

    free(compressed);
    free(decoded);
    return 0;
}

static int runIteration(unsigned iteration, const options_t* options)
{
    unsigned seed = options->seed ^ (iteration * 0x9E3779B9U);
    size_t const srcSize = 32768U + (nextRandom(&seed) % (768U * 1024U));
    int const level = 1 + (int)(iteration % 7U);
    unsigned const workers = options->maxWorkers == 0 ? 0U : iteration % (options->maxWorkers + 1U);
    unsigned char* const src = (unsigned char*)malloc(srcSize);

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    generateSample(src, srcSize, seed ^ 0xC001D00DU);

    if (testFrameHelpers(src, srcSize, level, workers)) {
        free(src);
        return 1;
    }
    if (testErrorPaths(src, srcSize, level)) {
        free(src);
        return 1;
    }
    if (testChecksumMismatch(src, srcSize, level)) {
        free(src);
        return 1;
    }
    if (testMissingContentSize(src, srcSize, level)) {
        free(src);
        return 1;
    }
    if (testPrefixDictionary(MAX(level, 3), workers)) {
        free(src);
        return 1;
    }
    if (options->verbose >= 2) {
        DISPLAY("test%3u : public API helper and round-trip checks passed\n", iteration + 1U);
    }

    free(src);
    return 0;
}

static int usage(const char* programName)
{
    DISPLAY("Usage:\n");
    DISPLAY("      %s [-v] [-T#] [-t#] [-s#]\n", programName);
    DISPLAY(" -v       : increase verbosity\n");
    DISPLAY(" -T#      : requested fuzz duration hint\n");
    DISPLAY(" -t#      : maximum worker count to test (default: 1)\n");
    DISPLAY(" -s#      : seed\n");
    return 0;
}

int main(int argc, char** argv)
{
    options_t options;
    unsigned iterations;
    unsigned argNb;

    memset(&options, 0, sizeof(options));
    options.maxWorkers = 1;
    options.seed = 0x1234567U;

    for (argNb = 1; argNb < (unsigned)argc; ++argNb) {
        const char* argument = argv[argNb];
        if (!strcmp(argument, "--no-big-tests")) {
            continue;
        }
        if (!strcmp(argument, "--help")) {
            return usage(argv[0]);
        }
        if (argument[0] == '-' && argument[1] == '-') {
            DISPLAY("unknown option: %s\n", argument);
            usage(argv[0]);
            return 1;
        }
        if (argument[0] == '-') {
            argument++;
            while (*argument != 0) {
                switch (*argument) {
                case 'h':
                case 'H':
                    return usage(argv[0]);
                case 'v':
                    options.verbose++;
                    argument++;
                    break;
                case 'T':
                    argument++;
                    options.durationSeconds = readU32FromChar(&argument);
                    break;
                case 't':
                    argument++;
                    options.maxWorkers = readU32FromChar(&argument);
                    break;
                case 's':
                    argument++;
                    options.seed = readU32FromChar(&argument);
                    break;
                default:
                    DISPLAY("unknown option: -%c\n", *argument);
                    usage(argv[0]);
                    return 1;
                }
            }
        }
    }

    if (options.maxWorkers == 0) {
        options.maxWorkers = 1;
    }

    if (testStaticHelpers()) {
        return 1;
    }
    if (testParameterBounds()) {
        return 1;
    }
    if (testInvalidEndDirective("public-api-fuzzer", strlen("public-api-fuzzer"))) {
        return 1;
    }

    iterations = options.durationSeconds == 0 ? 12U : MIN(options.durationSeconds / 4U + 12U, 48U);
    if (iterations == 0) {
        iterations = 12U;
    }

    for (argNb = 0; argNb < iterations; ++argNb) {
        if (runIteration(argNb, &options)) {
            return 1;
        }
    }

    DISPLAY("fuzzer: public API checks passed (%u cases)\n", iterations);
    return 0;
}
