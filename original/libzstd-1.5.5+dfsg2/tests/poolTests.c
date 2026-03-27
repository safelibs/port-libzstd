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

#define DISPLAY(...) fprintf(stderr, __VA_ARGS__)
#define MIN(a, b) ((a) < (b) ? (a) : (b))

#define CHECK_Z(value)                                                       \
    do {                                                                     \
        size_t const check_z_result = (value);                               \
        if (ZSTD_isError(check_z_result)) {                                  \
            DISPLAY("%s: %s\n", #value, ZSTD_getErrorName(check_z_result));  \
            return 1;                                                        \
        }                                                                    \
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
    unsigned char* out = (unsigned char*)buffer;
    size_t i;
    for (i = 0; i < size; ++i) {
        unsigned const rnd = nextRandom(&seed);
        if (i > 0 && (rnd % 100U) < 70U) {
            size_t const back = (rnd % MIN(i, (size_t)8192)) + 1;
            out[i] = out[i - back];
        } else {
            out[i] = (unsigned char)rnd;
        }
    }
}

static int roundTrip(size_t workers, int useDict)
{
    size_t const srcSize = 2U * 1024U * 1024U + 257U;
    size_t const dictSize = 64U * 1024U;
    void* const src = malloc(srcSize);
    void* const compressed = malloc(ZSTD_compressBound(srcSize));
    void* const decoded = malloc(srcSize);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    size_t compressedSize;

    if (src == NULL || compressed == NULL || decoded == NULL || cctx == NULL || dctx == NULL) {
        DISPLAY("allocation failure\n");
        free(src);
        free(compressed);
        free(decoded);
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        return 1;
    }

    generateSample(src, srcSize, (unsigned)(workers + 1U));

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 3));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));

    if (useDict) {
        CHECK_Z(ZSTD_CCtx_loadDictionary(cctx, src, dictSize));
        CHECK_Z(ZSTD_DCtx_loadDictionary(dctx, src, dictSize));
        compressedSize = ZSTD_compress2(cctx,
                                        compressed, ZSTD_compressBound(srcSize),
                                        (const unsigned char*)src + dictSize, srcSize - dictSize);
        if (ZSTD_isError(compressedSize)) {
            DISPLAY("ZSTD_compress2: %s\n", ZSTD_getErrorName(compressedSize));
            free(src);
            free(compressed);
            free(decoded);
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            return 1;
        }
        CHECK_Z(ZSTD_decompressDCtx(dctx, decoded, srcSize - dictSize, compressed, compressedSize));
        if (memcmp(decoded, (const unsigned char*)src + dictSize, srcSize - dictSize) != 0) {
            DISPLAY("dictionary round-trip mismatch\n");
            free(src);
            free(compressed);
            free(decoded);
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            return 1;
        }
    } else {
        compressedSize = ZSTD_compress2(cctx, compressed, ZSTD_compressBound(srcSize), src, srcSize);
        if (ZSTD_isError(compressedSize)) {
            DISPLAY("ZSTD_compress2: %s\n", ZSTD_getErrorName(compressedSize));
            free(src);
            free(compressed);
            free(decoded);
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            return 1;
        }
        CHECK_Z(ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize));
        if (memcmp(decoded, src, srcSize) != 0) {
            DISPLAY("round-trip mismatch\n");
            free(src);
            free(compressed);
            free(decoded);
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            return 1;
        }
    }

    free(src);
    free(compressed);
    free(decoded);
    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    return 0;
}

static int reuseContexts(void)
{
    size_t const srcSize = 1024U * 1024U + 13U;
    void* const src = malloc(srcSize);
    void* const compressed = malloc(ZSTD_compressBound(srcSize));
    void* const decoded = malloc(srcSize);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    size_t workers;

    if (src == NULL || compressed == NULL || decoded == NULL || cctx == NULL || dctx == NULL) {
        DISPLAY("allocation failure\n");
        free(src);
        free(compressed);
        free(decoded);
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        return 1;
    }

    generateSample(src, srcSize, 7U);
    for (workers = 0; workers <= 3; ++workers) {
        size_t compressedSize;
        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, (int)(workers + 1)));
        compressedSize = ZSTD_compress2(cctx, compressed, ZSTD_compressBound(srcSize), src, srcSize);
        if (ZSTD_isError(compressedSize)) {
            DISPLAY("context reuse compression failed: %s\n", ZSTD_getErrorName(compressedSize));
            free(src);
            free(compressed);
            free(decoded);
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            return 1;
        }
        CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only));
        CHECK_Z(ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize));
        if (memcmp(decoded, src, srcSize) != 0) {
            DISPLAY("context reuse mismatch\n");
            free(src);
            free(compressed);
            free(decoded);
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            return 1;
        }
    }

    free(src);
    free(compressed);
    free(decoded);
    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    return 0;
}

int main(void)
{
    if (roundTrip(0, 0)) {
        return 1;
    }
    if (roundTrip(1, 0)) {
        return 1;
    }
    if (roundTrip(2, 0)) {
        return 1;
    }
    if (roundTrip(2, 1)) {
        return 1;
    }
    if (reuseContexts()) {
        return 1;
    }

    DISPLAY("poolTests: public multithread smoke tests passed\n");
    return 0;
}
