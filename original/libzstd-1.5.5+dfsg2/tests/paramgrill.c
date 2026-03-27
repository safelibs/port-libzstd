/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

/*-************************************
*  Dependencies
**************************************/
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "zstd.h"


/*-************************************
*  Constants
**************************************/
#define PROGRAM_DESCRIPTION "ZSTD public parameter tester"
#define DEFAULT_INPUT_SIZE (256U << 10)
#define DEFAULT_ITERATIONS 1U
#define MIN_INPUT_SIZE 1024U


typedef struct {
    const char* name;
    int useDictionary;
    int expectKnownContentSize;
} testCase_t;

static const testCase_t g_testCases[] = {
    { "default-level", 0, 1 },
    { "windowed-greedy", 0, 1 },
    { "lazy-target", 0, 1 },
    { "btopt-checksum", 0, 1 },
    { "dictionary-hidden-size", 1, 0 }
};


static void usage(const char* programName)
{
    printf(
        "%s\n"
        "Usage: %s [-i iterations] [-s input-size] [-v]\n"
        "  -i iterations : number of passes over the built-in parameter cases (default: %u)\n"
        "  -s input-size : bytes to compress per pass; accepts K/M/G suffixes (default: %u)\n"
        "  -v            : print one line per passing case\n"
        "  -h            : display this help text\n",
        PROGRAM_DESCRIPTION,
        programName,
        DEFAULT_ITERATIONS,
        DEFAULT_INPUT_SIZE);
}

static int parseSize(const char* text, size_t* value)
{
    char* end = NULL;
    unsigned long long parsed = 0;
    unsigned long long multiplier = 1;

    errno = 0;
    parsed = strtoull(text, &end, 10);
    if (errno != 0 || end == text) {
        return 0;
    }

    if (*end != '\0') {
        if (end[1] != '\0') {
            return 0;
        }
        switch (*end) {
            case 'K':
            case 'k':
                multiplier = 1ULL << 10;
                break;
            case 'M':
            case 'm':
                multiplier = 1ULL << 20;
                break;
            case 'G':
            case 'g':
                multiplier = 1ULL << 30;
                break;
            default:
                return 0;
        }
    }

    if (parsed == 0 || parsed > ULLONG_MAX / multiplier) {
        return 0;
    }
    parsed *= multiplier;
    if (parsed > (unsigned long long)SIZE_MAX) {
        return 0;
    }

    *value = (size_t)parsed;
    return 1;
}

static int parseUnsigned(const char* text, unsigned* value)
{
    char* end = NULL;
    unsigned long parsed = 0;

    errno = 0;
    parsed = strtoul(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || parsed == 0 || parsed > UINT_MAX) {
        return 0;
    }

    *value = (unsigned)parsed;
    return 1;
}

static void fillBuffer(void* dst, size_t size, unsigned seed)
{
    static const char pattern[] =
        "Public zstd parameter coverage should stay on the imported API surface.\n";
    unsigned char* const output = (unsigned char*)dst;
    unsigned state = seed | 1U;
    size_t i;

    for (i = 0; i < size; ++i) {
        state = state * 1103515245U + 12345U;
        if ((i % 97U) < 72U) {
            output[i] = (unsigned char)pattern[(i + (size_t)(state >> 24)) % (sizeof(pattern) - 1U)];
        } else {
            output[i] = (unsigned char)(state >> 16);
        }
    }
}

static int checkZstd(size_t code, const char* action)
{
    if (ZSTD_isError(code)) {
        fprintf(stderr, "paramgrill: %s failed: %s\n", action, ZSTD_getErrorName(code));
        return 1;
    }
    return 0;
}

static int boundedValue(ZSTD_cParameter param, int preferred)
{
    ZSTD_bounds const bounds = ZSTD_cParam_getBounds(param);
    if (ZSTD_isError(bounds.error)) {
        fprintf(stderr, "paramgrill: bounds query failed: %s\n", ZSTD_getErrorName(bounds.error));
        exit(1);
    }
    if (preferred < bounds.lowerBound) {
        return bounds.lowerBound;
    }
    if (preferred > bounds.upperBound) {
        return bounds.upperBound;
    }
    return preferred;
}

static int setParam(ZSTD_CCtx* cctx, ZSTD_cParameter param, int preferred, const char* name)
{
    int const value = boundedValue(param, preferred);
    size_t const code = ZSTD_CCtx_setParameter(cctx, param, value);
    if (ZSTD_isError(code)) {
        fprintf(stderr, "paramgrill: setting %s=%d failed: %s\n",
                name, value, ZSTD_getErrorName(code));
        return 1;
    }
    return 0;
}

static int configureCase(ZSTD_CCtx* cctx, size_t testCaseIndex)
{
    switch (testCaseIndex) {
        case 0:
            if (setParam(cctx, ZSTD_c_compressionLevel, 1, "compressionLevel")) return 1;
            if (setParam(cctx, ZSTD_c_contentSizeFlag, 1, "contentSizeFlag")) return 1;
            return 0;
        case 1:
            if (setParam(cctx, ZSTD_c_compressionLevel, 4, "compressionLevel")) return 1;
            if (setParam(cctx, ZSTD_c_strategy, ZSTD_greedy, "strategy")) return 1;
            if (setParam(cctx, ZSTD_c_windowLog, 19, "windowLog")) return 1;
            if (setParam(cctx, ZSTD_c_hashLog, 18, "hashLog")) return 1;
            if (setParam(cctx, ZSTD_c_chainLog, 18, "chainLog")) return 1;
            if (setParam(cctx, ZSTD_c_searchLog, 2, "searchLog")) return 1;
            if (setParam(cctx, ZSTD_c_minMatch, 4, "minMatch")) return 1;
            if (setParam(cctx, ZSTD_c_contentSizeFlag, 1, "contentSizeFlag")) return 1;
            return 0;
        case 2:
            if (setParam(cctx, ZSTD_c_compressionLevel, 6, "compressionLevel")) return 1;
            if (setParam(cctx, ZSTD_c_strategy, ZSTD_lazy2, "strategy")) return 1;
            if (setParam(cctx, ZSTD_c_windowLog, 20, "windowLog")) return 1;
            if (setParam(cctx, ZSTD_c_hashLog, 19, "hashLog")) return 1;
            if (setParam(cctx, ZSTD_c_chainLog, 19, "chainLog")) return 1;
            if (setParam(cctx, ZSTD_c_searchLog, 4, "searchLog")) return 1;
            if (setParam(cctx, ZSTD_c_minMatch, 5, "minMatch")) return 1;
            if (setParam(cctx, ZSTD_c_targetLength, 8, "targetLength")) return 1;
            if (setParam(cctx, ZSTD_c_contentSizeFlag, 1, "contentSizeFlag")) return 1;
            return 0;
        case 3:
            if (setParam(cctx, ZSTD_c_compressionLevel, 9, "compressionLevel")) return 1;
            if (setParam(cctx, ZSTD_c_strategy, ZSTD_btopt, "strategy")) return 1;
            if (setParam(cctx, ZSTD_c_windowLog, 21, "windowLog")) return 1;
            if (setParam(cctx, ZSTD_c_hashLog, 20, "hashLog")) return 1;
            if (setParam(cctx, ZSTD_c_chainLog, 20, "chainLog")) return 1;
            if (setParam(cctx, ZSTD_c_searchLog, 5, "searchLog")) return 1;
            if (setParam(cctx, ZSTD_c_minMatch, 5, "minMatch")) return 1;
            if (setParam(cctx, ZSTD_c_targetLength, 16, "targetLength")) return 1;
            if (setParam(cctx, ZSTD_c_checksumFlag, 1, "checksumFlag")) return 1;
            if (setParam(cctx, ZSTD_c_contentSizeFlag, 1, "contentSizeFlag")) return 1;
            return 0;
        case 4:
            if (setParam(cctx, ZSTD_c_compressionLevel, 5, "compressionLevel")) return 1;
            if (setParam(cctx, ZSTD_c_strategy, ZSTD_dfast, "strategy")) return 1;
            if (setParam(cctx, ZSTD_c_windowLog, 18, "windowLog")) return 1;
            if (setParam(cctx, ZSTD_c_hashLog, 17, "hashLog")) return 1;
            if (setParam(cctx, ZSTD_c_minMatch, 4, "minMatch")) return 1;
            if (setParam(cctx, ZSTD_c_checksumFlag, 1, "checksumFlag")) return 1;
            if (setParam(cctx, ZSTD_c_contentSizeFlag, 0, "contentSizeFlag")) return 1;
            return 0;
        default:
            fprintf(stderr, "paramgrill: invalid test case index %zu\n", testCaseIndex);
            return 1;
    }
}

static int verifyFrameMetadata(const testCase_t* testCase,
                               const void* compressed,
                               size_t compressedSize,
                               size_t srcSize)
{
    size_t const frameSize = ZSTD_findFrameCompressedSize(compressed, compressedSize);
    unsigned long long const contentSize = ZSTD_getFrameContentSize(compressed, compressedSize);

    if (checkZstd(frameSize, "finding frame size")) {
        return 1;
    }
    if (frameSize != compressedSize) {
        fprintf(stderr, "paramgrill: incomplete frame accounting for %s (%zu != %zu)\n",
                testCase->name, frameSize, compressedSize);
        return 1;
    }

    if (contentSize == ZSTD_CONTENTSIZE_ERROR) {
        fprintf(stderr, "paramgrill: invalid content size for %s\n", testCase->name);
        return 1;
    }

    if (testCase->expectKnownContentSize) {
        if (contentSize != (unsigned long long)srcSize) {
            fprintf(stderr, "paramgrill: unexpected content size for %s (%llu != %zu)\n",
                    testCase->name, contentSize, srcSize);
            return 1;
        }
    } else if (contentSize != ZSTD_CONTENTSIZE_UNKNOWN) {
        fprintf(stderr, "paramgrill: expected hidden content size for %s, got %llu\n",
                testCase->name, contentSize);
        return 1;
    }

    return 0;
}

static int runCase(const testCase_t* testCase,
                   size_t testCaseIndex,
                   ZSTD_CCtx* cctx,
                   ZSTD_DCtx* dctx,
                   const void* src,
                   size_t srcSize,
                   const void* dict,
                   size_t dictSize,
                   void* compressed,
                   size_t compressedCapacity,
                   void* decompressed,
                   int verbose)
{
    size_t compressedSize = 0;
    size_t decompressedSize = 0;

    if (checkZstd(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters), "resetting CCtx")) {
        return 1;
    }
    if (configureCase(cctx, testCaseIndex)) {
        return 1;
    }
    if (testCase->useDictionary &&
        checkZstd(ZSTD_CCtx_loadDictionary(cctx, dict, dictSize), "loading compression dictionary")) {
        return 1;
    }

    compressedSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
    if (checkZstd(compressedSize, "compressing")) {
        return 1;
    }
    if (verifyFrameMetadata(testCase, compressed, compressedSize, srcSize)) {
        return 1;
    }

    if (checkZstd(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters), "resetting DCtx")) {
        return 1;
    }
    if (testCase->useDictionary) {
        decompressedSize = ZSTD_decompressDCtx(dctx, decompressed, srcSize, compressed, compressedSize);
        if (!ZSTD_isError(decompressedSize)) {
            fprintf(stderr, "paramgrill: expected missing-dictionary failure for %s\n", testCase->name);
            return 1;
        }
        if (checkZstd(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters), "resetting DCtx")) {
            return 1;
        }
        if (checkZstd(ZSTD_DCtx_loadDictionary(dctx, dict, dictSize), "loading decompression dictionary")) {
            return 1;
        }
    }

    decompressedSize = ZSTD_decompressDCtx(dctx, decompressed, srcSize, compressed, compressedSize);
    if (checkZstd(decompressedSize, "decompressing")) {
        return 1;
    }
    if (decompressedSize != srcSize || memcmp(decompressed, src, srcSize) != 0) {
        fprintf(stderr, "paramgrill: round-trip mismatch for %s\n", testCase->name);
        return 1;
    }

    if (verbose) {
        printf("paramgrill: %s passed (%zu bytes -> %zu bytes)\n",
               testCase->name, srcSize, compressedSize);
    }

    return 0;
}

int main(int argc, const char** argv)
{
    ZSTD_CCtx* cctx = NULL;
    ZSTD_DCtx* dctx = NULL;
    void* src = NULL;
    void* compressed = NULL;
    void* decompressed = NULL;
    size_t inputSize = DEFAULT_INPUT_SIZE;
    size_t compressedCapacity = 0;
    size_t dictSize = 0;
    unsigned iterations = DEFAULT_ITERATIONS;
    int verbose = 0;
    int result = 1;
    int argNb;
    unsigned iteration;

    for (argNb = 1; argNb < argc; ++argNb) {
        const char* const argument = argv[argNb];
        if (!strcmp(argument, "-h") || !strcmp(argument, "--help")) {
            usage(argv[0]);
            return 0;
        }
        if (!strcmp(argument, "-v")) {
            verbose = 1;
            continue;
        }
        if (!strcmp(argument, "-i")) {
            if (argNb + 1 >= argc || !parseUnsigned(argv[++argNb], &iterations)) {
                fprintf(stderr, "paramgrill: invalid iteration count\n");
                return 1;
            }
            continue;
        }
        if (!strncmp(argument, "-i", 2)) {
            if (!parseUnsigned(argument + 2, &iterations)) {
                fprintf(stderr, "paramgrill: invalid iteration count\n");
                return 1;
            }
            continue;
        }
        if (!strcmp(argument, "-s")) {
            if (argNb + 1 >= argc || !parseSize(argv[++argNb], &inputSize)) {
                fprintf(stderr, "paramgrill: invalid input size\n");
                return 1;
            }
            continue;
        }
        if (!strncmp(argument, "-s", 2)) {
            if (!parseSize(argument + 2, &inputSize)) {
                fprintf(stderr, "paramgrill: invalid input size\n");
                return 1;
            }
            continue;
        }

        fprintf(stderr, "paramgrill: unsupported argument: %s\n", argument);
        usage(argv[0]);
        return 1;
    }

    if (inputSize < MIN_INPUT_SIZE) {
        fprintf(stderr, "paramgrill: input size must be at least %u bytes\n", MIN_INPUT_SIZE);
        return 1;
    }

    compressedCapacity = ZSTD_compressBound(inputSize);
    if (checkZstd(compressedCapacity, "computing compression bound")) {
        return 1;
    }

    src = malloc(inputSize);
    compressed = malloc(compressedCapacity);
    decompressed = malloc(inputSize);
    if (src == NULL || compressed == NULL || decompressed == NULL) {
        fprintf(stderr, "paramgrill: allocation failure\n");
        goto cleanup;
    }

    cctx = ZSTD_createCCtx();
    dctx = ZSTD_createDCtx();
    if (cctx == NULL || dctx == NULL) {
        fprintf(stderr, "paramgrill: context allocation failure\n");
        goto cleanup;
    }

    dictSize = inputSize / 8U;
    if (dictSize < 256U) {
        dictSize = 256U;
    }
    if (dictSize > 4096U) {
        dictSize = 4096U;
    }
    if (dictSize >= inputSize) {
        dictSize = inputSize / 2U;
    }

    for (iteration = 0; iteration < iterations; ++iteration) {
        size_t testCaseIndex;
        fillBuffer(src, inputSize, 0xC0FFEE00U + iteration);
        memset(decompressed, 0xA5, inputSize);

        for (testCaseIndex = 0; testCaseIndex < sizeof(g_testCases) / sizeof(g_testCases[0]); ++testCaseIndex) {
            if (runCase(&g_testCases[testCaseIndex], testCaseIndex, cctx, dctx,
                        src, inputSize, src, dictSize,
                        compressed, compressedCapacity, decompressed, verbose)) {
                goto cleanup;
            }
        }
    }

    printf("paramgrill: %zu public parameter cases passed across %u iteration(s)\n",
           sizeof(g_testCases) / sizeof(g_testCases[0]), iterations);
    result = 0;

cleanup:
    ZSTD_freeDCtx(dctx);
    ZSTD_freeCCtx(cctx);
    free(decompressed);
    free(compressed);
    free(src);
    return result;
}
