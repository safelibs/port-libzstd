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
        if (i > 0 && (rnd % 100U) < 70U) {
            size_t const back = (rnd % MIN(i, (size_t)16384U)) + 1;
            out[i] = out[i - back];
        } else {
            out[i] = (unsigned char)rnd;
        }
    }
}

static int oneShotRoundTrip(const void* src,
                            size_t srcSize,
                            int level,
                            unsigned workers,
                            const void* dict,
                            size_t dictSize)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    void* const compressed = malloc(ZSTD_compressBound(srcSize));
    void* const decoded = malloc(srcSize);
    size_t compressedSize;
    size_t decodedSize;

    if (cctx == NULL || dctx == NULL || compressed == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, level));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));

    if (dict != NULL && dictSize != 0) {
        CHECK_Z(ZSTD_CCtx_loadDictionary(cctx, dict, dictSize));
        CHECK_Z(ZSTD_DCtx_loadDictionary(dctx, dict, dictSize));
    }

    compressedSize = ZSTD_compress2(cctx, compressed, ZSTD_compressBound(srcSize), src, srcSize);
    if (ZSTD_isError(compressedSize)) {
        DISPLAY("ZSTD_compress2: %s\n", ZSTD_getErrorName(compressedSize));
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize);
    if (ZSTD_isError(decodedSize) || decodedSize != srcSize || memcmp(decoded, src, srcSize) != 0) {
        DISPLAY("one-shot round-trip mismatch\n");
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    if (ZSTD_findFrameCompressedSize(compressed, compressedSize) != compressedSize) {
        DISPLAY("ZSTD_findFrameCompressedSize mismatch\n");
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }
    if (ZSTD_getFrameContentSize(compressed, compressedSize) != srcSize) {
        DISPLAY("ZSTD_getFrameContentSize mismatch\n");
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }
    if (compressedSize > 1 && !ZSTD_isError(ZSTD_decompress(decoded, srcSize, compressed, compressedSize - 1))) {
        DISPLAY("truncated frame unexpectedly decompressed\n");
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    free(compressed);
    free(decoded);
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
    unsigned test;

    memset(&options, 0, sizeof(options));
    options.maxWorkers = 1;
    options.seed = 0x1234567U;

    for (test = 1; test < (unsigned)argc; ++test) {
        const char* argument = argv[test];
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
                    return usage(argv[0]);
                }
            }
        }
    }

    if (options.maxWorkers == 0) {
        options.maxWorkers = 1;
    }

    iterations = options.durationSeconds == 0 ? 16U : MIN(options.durationSeconds * 4U, 64U);
    if (iterations == 0) {
        iterations = 16U;
    }

    for (test = 0; test < iterations; ++test) {
        size_t const srcSize = 1024U + (nextRandom(&options.seed) % (1024U * 1024U));
        unsigned char* const src = (unsigned char*)malloc(srcSize);
        size_t const dictSize = srcSize > (32U * 1024U) ? 16U * 1024U : 0U;
        int const level = 1 + (int)(test % 7U);
        unsigned const workers = test % (options.maxWorkers + 1U);

        if (src == NULL) {
            DISPLAY("allocation failure\n");
            return 1;
        }

        generateSample(src, srcSize, options.seed ^ test);

        if (oneShotRoundTrip(src, srcSize, level, workers, NULL, 0)) {
            free(src);
            return 1;
        }
        if (dictSize != 0 &&
            oneShotRoundTrip(src + dictSize, srcSize - dictSize, level,
                             workers, src, dictSize)) {
            free(src);
            return 1;
        }
        if (options.verbose >= 2) {
            DISPLAY("test%3u : public API fuzz case ok\n", test + 1);
        }
        free(src);
    }

    DISPLAY("fuzzer: public API checks passed (%u cases)\n", iterations);
    return 0;
}
