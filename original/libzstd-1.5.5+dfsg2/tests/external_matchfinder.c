/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "zstd.h"
#include "zstd_errors.h"

#define DISPLAY(...) fprintf(stderr, __VA_ARGS__)
#define ARRAY_SIZE(a) (sizeof(a) / sizeof((a)[0]))
#define MAX(a, b) ((a) > (b) ? (a) : (b))

#define CHECK_Z(value)                                                       \
    do {                                                                     \
        size_t const check_z_result = (value);                               \
        if (ZSTD_isError(check_z_result)) {                                  \
            DISPLAY("%s: %s\n", #value, ZSTD_getErrorName(check_z_result));  \
            return 1;                                                        \
        }                                                                    \
    } while (0)

#define CHECK(cond, ...)            \
    do {                            \
        if (!(cond)) {              \
            DISPLAY(__VA_ARGS__);   \
            return 1;               \
        }                           \
    } while (0)

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
    static const char* const tokens[] = {
        "matchfinder-alpha-", "matchfinder-beta-",
        "matchfinder-gamma-", "matchfinder-delta-"
    };
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    unsigned char* out = (unsigned char*)buffer;
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        size_t const token = nextRandom(&state) % ARRAY_SIZE(tokens);
        size_t const tokenLen = strlen(tokens[token]);
        size_t i;
        for (i = 0; i < tokenLen && pos < size; ++i) {
            out[pos++] = (unsigned char)tokens[token][i];
        }
        for (i = 0; i < 96U && pos < size; ++i) {
            out[pos++] = (unsigned char)alphabet[nextRandom(&state) % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) {
            out[pos++] = '\n';
        }
    }
}

static int roundTrip(const void* src,
                     size_t srcSize,
                     const void* compressed,
                     size_t compressedSize,
                     const void* prefix,
                     size_t prefixSize)
{
    unsigned char* const decoded = (unsigned char*)malloc(MAX(srcSize, (size_t)1));
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    size_t decodedSize;

    if (decoded == NULL || dctx == NULL) {
        DISPLAY("allocation failure\n");
        free(decoded);
        ZSTD_freeDCtx(dctx);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    if (prefix != NULL && prefixSize != 0U) {
        CHECK_Z(ZSTD_DCtx_refPrefix(dctx, prefix, prefixSize));
    }

    decodedSize = ZSTD_decompressDCtx(dctx,
                                      decoded,
                                      MAX(srcSize, (size_t)1),
                                      compressed,
                                      compressedSize);
    CHECK(!ZSTD_isError(decodedSize), "ZSTD_decompressDCtx failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "decoded data mismatch\n");
    CHECK(ZSTD_getFrameContentSize(compressed, compressedSize) == (unsigned long long)srcSize,
          "frame content size mismatch\n");
    CHECK(ZSTD_findFrameCompressedSize(compressed, compressedSize) == compressedSize,
          "frame compressed size mismatch\n");

    free(decoded);
    ZSTD_freeDCtx(dctx);
    return 0;
}

static int compressWithParams(const void* src,
                              size_t srcSize,
                              const void* prefix,
                              size_t prefixSize,
                              int strategy,
                              int enableLdm,
                              void** compressedOut,
                              size_t* compressedSizeOut)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    void* const compressed = malloc(ZSTD_compressBound(srcSize));
    size_t compressedSize;

    if (cctx == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 5));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, strategy));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_enableLongDistanceMatching, enableLdm));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, 27));
    if (prefix != NULL && prefixSize != 0U) {
        CHECK_Z(ZSTD_CCtx_refPrefix(cctx, prefix, prefixSize));
    }

    compressedSize = ZSTD_compress2(cctx, compressed, ZSTD_compressBound(srcSize), src, srcSize);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compress2 failed: %s\n", ZSTD_getErrorName(compressedSize));

    ZSTD_freeCCtx(cctx);
    *compressedOut = compressed;
    *compressedSizeOut = compressedSize;
    return 0;
}

static int checkOutOfBoundsBehavior(ZSTD_CCtx* cctx, ZSTD_cParameter param, int value)
{
    size_t const code = ZSTD_CCtx_setParameter(cctx, param, value);
    if (ZSTD_isError(code)) {
        CHECK(ZSTD_getErrorCode(code) == ZSTD_error_parameter_outOfBound,
              "parameter %d returned %s instead of parameter_outOfBound\n",
              (int)param, ZSTD_getErrorName(code));
    }
    return 0;
}

static int testMatchParameterBounds(void)
{
    static const ZSTD_cParameter params[] = {
        ZSTD_c_hashLog,
        ZSTD_c_chainLog,
        ZSTD_c_searchLog,
        ZSTD_c_minMatch,
        ZSTD_c_targetLength,
        ZSTD_c_strategy
    };
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    size_t i;

    if (cctx == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    for (i = 0; i < ARRAY_SIZE(params); ++i) {
        ZSTD_bounds const bounds = ZSTD_cParam_getBounds(params[i]);
        CHECK(!ZSTD_isError(bounds.error), "could not query bounds for parameter %d\n", (int)params[i]);
        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, params[i], bounds.lowerBound));
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, params[i], bounds.upperBound));
        if (bounds.upperBound < INT_MAX) {
            if (checkOutOfBoundsBehavior(cctx, params[i], bounds.upperBound + 1)) {
                ZSTD_freeCCtx(cctx);
                return 1;
            }
        } else if (bounds.lowerBound > INT_MIN) {
            if (checkOutOfBoundsBehavior(cctx, params[i], bounds.lowerBound - 1)) {
                ZSTD_freeCCtx(cctx);
                return 1;
            }
        }
    }

    ZSTD_freeCCtx(cctx);
    return 0;
}

static int testStrategyMatrix(void)
{
    static const int strategies[] = {
        ZSTD_fast,
        ZSTD_dfast,
        ZSTD_greedy,
        ZSTD_lazy2,
        ZSTD_btopt,
        ZSTD_btultra2
    };
    size_t const srcSize = 1024U * 1024U + 777U;
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    size_t fastSize = 0;
    size_t ultraSize = 0;
    size_t i;

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }
    generateSample(src, srcSize, 123U);

    for (i = 0; i < ARRAY_SIZE(strategies); ++i) {
        void* compressed = NULL;
        size_t compressedSize = 0;
        if (compressWithParams(src, srcSize, NULL, 0, strategies[i], 0, &compressed, &compressedSize) ||
            roundTrip(src, srcSize, compressed, compressedSize, NULL, 0)) {
            free(compressed);
            free(src);
            return 1;
        }
        if (strategies[i] == ZSTD_fast) {
            fastSize = compressedSize;
        }
        if (strategies[i] == ZSTD_btultra2) {
            ultraSize = compressedSize;
        }
        free(compressed);
    }

    CHECK(ultraSize <= fastSize,
          "btultra2 unexpectedly compressed worse than fast (%u > %u)\n",
          (unsigned)ultraSize, (unsigned)fastSize);
    free(src);
    return 0;
}

static int testPrefixReferenceBenefit(void)
{
    size_t const dictSize = 64U * 1024U;
    size_t const srcSize = dictSize * 4U;
    unsigned char* const dict = (unsigned char*)malloc(dictSize);
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    void* plainCompressed = NULL;
    void* prefixedCompressed = NULL;
    size_t plainSize = 0;
    size_t prefixedSize = 0;
    size_t chunk;

    if (dict == NULL || src == NULL) {
        DISPLAY("allocation failure\n");
        free(dict);
        free(src);
        return 1;
    }

    generateSample(dict, dictSize, 321U);
    for (chunk = 0; chunk < 4U; ++chunk) {
        memcpy(src + chunk * dictSize, dict, dictSize);
    }

    if (compressWithParams(src, srcSize, NULL, 0, ZSTD_greedy, 0, &plainCompressed, &plainSize) ||
        compressWithParams(src, srcSize, dict, dictSize, ZSTD_greedy, 0, &prefixedCompressed, &prefixedSize) ||
        roundTrip(src, srcSize, plainCompressed, plainSize, NULL, 0) ||
        roundTrip(src, srcSize, prefixedCompressed, prefixedSize, dict, dictSize)) {
        free(plainCompressed);
        free(prefixedCompressed);
        free(dict);
        free(src);
        return 1;
    }

    CHECK(prefixedSize < plainSize,
          "prefix reference did not improve compression (%u >= %u)\n",
          (unsigned)prefixedSize, (unsigned)plainSize);

    free(plainCompressed);
    free(prefixedCompressed);
    free(dict);
    free(src);
    return 0;
}

static int streamCompressLdm(const void* src,
                             size_t srcSize,
                             void** compressedOut,
                             size_t* compressedSizeOut)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize) + 4096U);
    size_t const capacity = ZSTD_compressBound(srcSize) + 4096U;
    size_t dstPos = 0;
    size_t srcPos = 0;

    if (cctx == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 5));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, ZSTD_btopt));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_enableLongDistanceMatching, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, 27));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, 1));
    CHECK_Z(ZSTD_CCtx_setPledgedSrcSize(cctx, srcSize));

    while (srcPos < srcSize) {
        ZSTD_inBuffer in;
        ZSTD_outBuffer out;
        size_t const chunkSize = (srcSize - srcPos) > (64U * 1024U) ? (64U * 1024U) : (srcSize - srcPos);
        size_t remaining;

        in.src = (const unsigned char*)src + srcPos;
        in.size = chunkSize;
        in.pos = 0;
        do {
            out.dst = compressed + dstPos;
            out.size = capacity - dstPos;
            out.pos = 0;
            remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_continue);
            CHECK(!ZSTD_isError(remaining), "stream LDM compression failed: %s\n", ZSTD_getErrorName(remaining));
            dstPos += out.pos;
            CHECK(out.pos != 0 || in.pos == in.size, "stream LDM compression stalled\n");
        } while (in.pos < in.size);
        srcPos += chunkSize;
    }

    while (1) {
        ZSTD_inBuffer in;
        ZSTD_outBuffer out;
        size_t remaining;

        in.src = "";
        in.size = 0;
        in.pos = 0;
        out.dst = compressed + dstPos;
        out.size = capacity - dstPos;
        out.pos = 0;
        remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_end);
        CHECK(!ZSTD_isError(remaining), "stream LDM finalization failed: %s\n", ZSTD_getErrorName(remaining));
        dstPos += out.pos;
        if (remaining == 0) {
            break;
        }
        CHECK(out.pos != 0, "stream LDM finalization stalled\n");
    }

    ZSTD_freeCCtx(cctx);
    *compressedOut = compressed;
    *compressedSizeOut = dstPos;
    return 0;
}

static int testLongDistanceStreaming(void)
{
    size_t const patternSize = 256U * 1024U;
    size_t const gapSize = 1024U * 1024U;
    size_t const srcSize = patternSize + gapSize + patternSize + gapSize + patternSize;
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    void* compressed = NULL;
    size_t compressedSize = 0;

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    generateSample(src, patternSize, 777U);
    generateSample(src + patternSize, gapSize, 111U);
    memcpy(src + patternSize + gapSize, src, patternSize);
    generateSample(src + patternSize + gapSize + patternSize, gapSize, 222U);
    memcpy(src + patternSize + gapSize + patternSize + gapSize, src, patternSize);

    if (streamCompressLdm(src, srcSize, &compressed, &compressedSize) ||
        roundTrip(src, srcSize, compressed, compressedSize, NULL, 0)) {
        free(compressed);
        free(src);
        return 1;
    }

    free(compressed);
    free(src);
    return 0;
}

int main(void)
{
    if (testMatchParameterBounds() ||
        testStrategyMatrix() ||
        testPrefixReferenceBenefit() ||
        testLongDistanceStreaming()) {
        return 1;
    }

    DISPLAY("external_matchfinder: public match parameter tests passed\n");
    return 0;
}
