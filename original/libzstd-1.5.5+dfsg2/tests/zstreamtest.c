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
    int newapi;
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
        if (i > 0 && (rnd % 100U) < 65U) {
            size_t const back = (rnd % MIN(i, (size_t)16384)) + 1;
            out[i] = out[i - back];
        } else {
            out[i] = (unsigned char)rnd;
        }
    }
}

static int decompressStreamRoundTrip(const void* compressed,
                                     size_t compressedSize,
                                     const void* expected,
                                     size_t expectedSize,
                                     const void* dict,
                                     size_t dictSize,
                                     unsigned seed)
{
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    unsigned char* const decoded = (unsigned char*)malloc(expectedSize);
    ZSTD_inBuffer in;
    ZSTD_outBuffer out;
    size_t ret;

    (void)seed;

    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    if (dict != NULL && dictSize != 0) {
        CHECK_Z(ZSTD_DCtx_loadDictionary(dctx, dict, dictSize));
    }

    in.src = compressed;
    in.size = compressedSize;
    in.pos = 0;
    out.dst = decoded;
    out.size = expectedSize;
    out.pos = 0;
    do {
        ret = ZSTD_decompressStream(dctx, &out, &in);
        if (ZSTD_isError(ret)) {
            DISPLAY("ZSTD_decompressStream: %s\n", ZSTD_getErrorName(ret));
            ZSTD_freeDCtx(dctx);
            free(decoded);
            return 1;
        }
    } while (ret != 0);

    if (out.pos != expectedSize || memcmp(decoded, expected, expectedSize) != 0) {
        DISPLAY("stream round-trip mismatch\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        return 1;
    }

    ZSTD_freeDCtx(dctx);
    free(decoded);
    return 0;
}

static int runLegacyRoundTrip(const void* src,
                              size_t srcSize,
                              int level,
                              unsigned seed)
{
    ZSTD_CStream* const cstream = ZSTD_createCStream();
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize());
    ZSTD_inBuffer in;
    ZSTD_outBuffer out;
    size_t dstPos = 0;

    (void)seed;

    if (cstream == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCStream(cstream);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_initCStream(cstream, level));
    in.src = src;
    in.size = srcSize;
    in.pos = 0;
    out.dst = compressed;
    out.size = ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize();
    out.pos = 0;
    while (in.pos < in.size) {
        CHECK_Z(ZSTD_compressStream(cstream, &out, &in));
    }
    dstPos = out.pos;

    for (;;) {
        ZSTD_outBuffer endOut = {
            compressed + dstPos,
            ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize() - dstPos,
            0
        };
        size_t const remaining = ZSTD_endStream(cstream, &endOut);
        if (ZSTD_isError(remaining)) {
            DISPLAY("ZSTD_endStream: %s\n", ZSTD_getErrorName(remaining));
            ZSTD_freeCStream(cstream);
            free(compressed);
            return 1;
        }
        dstPos += endOut.pos;
        if (remaining == 0) {
            break;
        }
    }

    if (decompressStreamRoundTrip(compressed, dstPos, src, srcSize, NULL, 0, seed ^ 0xA5A5A5A5U)) {
        ZSTD_freeCStream(cstream);
        free(compressed);
        return 1;
    }

    ZSTD_freeCStream(cstream);
    free(compressed);
    return 0;
}

static int runNewApiRoundTrip(const void* src,
                              size_t srcSize,
                              int level,
                              unsigned workers,
                              const void* dict,
                              size_t dictSize,
                              unsigned seed)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize());
    ZSTD_inBuffer in;
    ZSTD_outBuffer out;
    size_t dstPos = 0;

    (void)seed;

    if (cctx == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, level));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    if (dict != NULL && dictSize != 0) {
        CHECK_Z(ZSTD_CCtx_loadDictionary(cctx, dict, dictSize));
    }

    in.src = src;
    in.size = srcSize;
    in.pos = 0;
    out.dst = compressed;
    out.size = ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize();
    out.pos = 0;
    for (;;) {
        size_t const remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_end);
        if (ZSTD_isError(remaining)) {
            DISPLAY("ZSTD_compressStream2(..., end): %s\n", ZSTD_getErrorName(remaining));
            ZSTD_freeCCtx(cctx);
            free(compressed);
            return 1;
        }
        if (remaining == 0) {
            break;
        }
    }
    dstPos = out.pos;

    if (decompressStreamRoundTrip(compressed, dstPos, src, srcSize, dict, dictSize, seed ^ 0xDEADBEEFU)) {
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    ZSTD_freeCCtx(cctx);
    free(compressed);
    return 0;
}

static int usage(const char* programName)
{
    DISPLAY("Usage:\n");
    DISPLAY("      %s [-v] [-T#] [--newapi] [-t#]\n", programName);
    DISPLAY(" -v       : increase verbosity\n");
    DISPLAY(" -T#      : requested fuzz duration hint\n");
    DISPLAY(" --newapi : exercise compressStream2 instead of compressStream\n");
    DISPLAY(" -t#      : maximum worker count for new API mode\n");
    return 0;
}

int main(int argc, char** argv)
{
    options_t options;
    unsigned iterations;
    unsigned test;
    unsigned seed = 0xC0FFEEU;

    memset(&options, 0, sizeof(options));

    for (test = 1; test < (unsigned)argc; ++test) {
        const char* argument = argv[test];
        if (strcmp(argument, "--newapi") == 0) {
            options.newapi = 1;
            continue;
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

    iterations = options.durationSeconds == 0 ? 12U : MIN(options.durationSeconds * 4U, 64U);
    if (iterations == 0) {
        iterations = 12U;
    }

    for (test = 0; test < iterations; ++test) {
        size_t const srcSize = 4096U + (nextRandom(&seed) % (768U * 1024U));
        unsigned char* const src = (unsigned char*)malloc(srcSize);
        size_t const dictSize = srcSize > (32U * 1024U) ? 16U * 1024U : 0;
        unsigned const workers = options.newapi ? 1U + (test % options.maxWorkers) : 0U;
        int const level = 1 + (int)(test % 5U);

        if (src == NULL) {
            DISPLAY("allocation failure\n");
            return 1;
        }

        generateSample(src, srcSize, seed);
        if (options.newapi) {
            const void* dict = dictSize == 0 ? NULL : src;
            if (runNewApiRoundTrip(src + dictSize, srcSize - dictSize,
                                   level, workers,
                                   dict, dictSize,
                                   seed ^ test)) {
                free(src);
                return 1;
            }
        } else {
            if (runLegacyRoundTrip(src, srcSize, level, seed ^ test)) {
                free(src);
                return 1;
            }
        }

        if (options.verbose >= 2) {
            DISPLAY("test%3u : stream round-trip ok\n", test + 1);
        }
        free(src);
    }

    DISPLAY("zstreamtest: public streaming checks passed (%u cases%s)\n",
            iterations,
            options.newapi ? ", newapi" : "");
    return 0;
}
