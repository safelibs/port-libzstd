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
#include <time.h>

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
    unsigned contentSize;
    unsigned useDict;
    unsigned dictSize;
    unsigned maxContentSizeLog;
} advancedOptions_t;

static unsigned readInt(const char** argument)
{
    unsigned result = 0;
    while ((**argument >= '0') && (**argument <= '9')) {
        result *= 10;
        result += (unsigned)(**argument - '0');
        (*argument)++;
    }
    return result;
}

static unsigned readU32FromChar(const char** stringPtr)
{
    unsigned result = readInt(stringPtr);
    if ((**stringPtr == 'K') || (**stringPtr == 'M')) {
        result <<= 10;
        if (**stringPtr == 'M') {
            result <<= 10;
        }
        (*stringPtr)++;
        if (**stringPtr == 'i') {
            (*stringPtr)++;
        }
        if (**stringPtr == 'B') {
            (*stringPtr)++;
        }
    }
    return result;
}

static int longCommandWArg(const char** stringPtr, const char* longCommand)
{
    size_t const len = strlen(longCommand);
    int const match = strncmp(*stringPtr, longCommand, len) == 0;
    if (match) {
        *stringPtr += len;
    }
    return match;
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
        if (i > 0 && (rnd % 100U) < 68U) {
            size_t const back = (rnd % MIN(i, (size_t)8192U)) + 1U;
            out[i] = out[i - back];
        } else {
            out[i] = (unsigned char)rnd;
        }
    }
}

static unsigned makeSeed(void)
{
    return (unsigned)time(NULL) ^ 0xA5A5A5A5U;
}

static int outputBuffer(const void* buffer, size_t size, const char* path)
{
    FILE* file = fopen(path, "wb");
    if (file == NULL) {
        DISPLAY("cannot open %s for writing\n", path);
        return 1;
    }
    if (fwrite(buffer, 1, size, file) != size) {
        DISPLAY("cannot write %s\n", path);
        fclose(file);
        return 1;
    }
    fclose(file);
    return 0;
}

static int joinPath(char* out, size_t outSize, const char* dir, const char* name)
{
    int const written = snprintf(out, outSize, "%s/%s", dir, name);
    if (written < 0 || (size_t)written >= outSize) {
        DISPLAY("path too long\n");
        return 1;
    }
    return 0;
}

static int compressSample(const void* src,
                          size_t srcSize,
                          const void* dict,
                          size_t dictSize,
                          unsigned contentSize,
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
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 3));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, (int)contentSize));
    if (dict != NULL && dictSize != 0) {
        CHECK_Z(ZSTD_CCtx_loadDictionary(cctx, dict, dictSize));
    }

    compressedSize = ZSTD_compress2(cctx, compressed, ZSTD_compressBound(srcSize), src, srcSize);
    if (ZSTD_isError(compressedSize)) {
        DISPLAY("ZSTD_compress2: %s\n", ZSTD_getErrorName(compressedSize));
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    ZSTD_freeCCtx(cctx);
    *compressedOut = compressed;
    *compressedSizeOut = compressedSize;
    return 0;
}

static int verifySample(const void* src,
                        size_t srcSize,
                        const void* compressed,
                        size_t compressedSize,
                        const void* dict,
                        size_t dictSize)
{
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    void* const decoded = malloc(srcSize);
    size_t decodedSize;

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
    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize);
    if (ZSTD_isError(decodedSize) || decodedSize != srcSize || memcmp(decoded, src, srcSize) != 0) {
        DISPLAY("decodecorpus verification failed\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        return 1;
    }

    ZSTD_freeDCtx(dctx);
    free(decoded);
    return 0;
}

static int generateOneFile(unsigned seed,
                           const char* compressedPath,
                           const char* originalPath,
                           const void* dict,
                           size_t dictSize,
                           const advancedOptions_t* advanced)
{
    size_t const maxSize = 1U << advanced->maxContentSizeLog;
    size_t const srcSize = MIN(maxSize, (size_t)(64U * 1024U + (seed % (256U * 1024U))));
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    void* compressed = NULL;
    size_t compressedSize = 0;

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    generateSample(src, srcSize, seed);
    if (compressSample(src, srcSize, dict, dictSize, advanced->contentSize, &compressed, &compressedSize)) {
        free(src);
        return 1;
    }
    if (verifySample(src, srcSize, compressed, compressedSize, dict, dictSize)) {
        free(src);
        free(compressed);
        return 1;
    }
    if (outputBuffer(compressed, compressedSize, compressedPath)) {
        free(src);
        free(compressed);
        return 1;
    }
    if (originalPath != NULL && outputBuffer(src, srcSize, originalPath)) {
        free(src);
        free(compressed);
        return 1;
    }

    free(src);
    free(compressed);
    return 0;
}

static int generateCorpus(unsigned seed,
                          unsigned numFiles,
                          const char* path,
                          const char* origPath,
                          const advancedOptions_t* advanced)
{
    char outPath[4096];
    char origFile[4096];
    unsigned fileNb;
    unsigned char* dict = NULL;

    if (advanced->useDict) {
        dict = (unsigned char*)malloc(advanced->dictSize);
        if (dict == NULL) {
            DISPLAY("allocation failure\n");
            return 1;
        }
        generateSample(dict, advanced->dictSize, seed ^ 0xFACE1234U);
        if (joinPath(outPath, sizeof(outPath), path, "dictionary") ||
            outputBuffer(dict, advanced->dictSize, outPath)) {
            free(dict);
            return 1;
        }
    }

    for (fileNb = 0; fileNb < numFiles; ++fileNb) {
        if (snprintf(outPath, sizeof(outPath), "%s/z%06u.zst", path, fileNb) >= (int)sizeof(outPath)) {
            DISPLAY("path too long\n");
            free(dict);
            return 1;
        }
        if (origPath != NULL) {
            if (snprintf(origFile, sizeof(origFile), "%s/z%06u", origPath, fileNb) >= (int)sizeof(origFile)) {
                DISPLAY("path too long\n");
                free(dict);
                return 1;
            }
        }
        if (generateOneFile(seed + fileNb,
                            outPath,
                            origPath != NULL ? origFile : NULL,
                            dict,
                            advanced->useDict ? advanced->dictSize : 0,
                            advanced)) {
            free(dict);
            return 1;
        }
    }

    free(dict);
    return 0;
}

static int runTestMode(unsigned seed,
                       unsigned numFiles,
                       unsigned testDuration,
                       const advancedOptions_t* advanced)
{
    unsigned iterations = testDuration == 0 ? 12U : MIN(testDuration * 4U, 48U);
    unsigned test;
    (void)numFiles;

    if (iterations == 0) {
        iterations = 12U;
    }

    for (test = 0; test < iterations; ++test) {
        size_t const maxSize = 1U << advanced->maxContentSizeLog;
        size_t const srcSize = MIN(maxSize, (size_t)(1024U + (nextRandom(&seed) % (256U * 1024U))));
        unsigned char* const src = (unsigned char*)malloc(srcSize);
        unsigned char* dict = NULL;
        size_t dictSize = 0;
        void* compressed = NULL;
        size_t compressedSize = 0;

        if (src == NULL) {
            DISPLAY("allocation failure\n");
            return 1;
        }
        generateSample(src, srcSize, seed ^ test);

        if (advanced->useDict && srcSize > 32768U) {
            dictSize = MIN((size_t)advanced->dictSize, srcSize / 4U);
            dict = (unsigned char*)malloc(dictSize);
            if (dict == NULL) {
                free(src);
                DISPLAY("allocation failure\n");
                return 1;
            }
            memcpy(dict, src, dictSize);
        }

        if (compressSample(src + dictSize, srcSize - dictSize,
                           dict, dictSize,
                           advanced->contentSize,
                           &compressed, &compressedSize) ||
            verifySample(src + dictSize, srcSize - dictSize,
                         compressed, compressedSize,
                         dict, dictSize)) {
            free(src);
            free(dict);
            free(compressed);
            return 1;
        }

        free(src);
        free(dict);
        free(compressed);
    }

    DISPLAY("decodecorpus: public API corpus checks passed (%u cases)\n", iterations);
    return 0;
}

static void usage(const char* programName)
{
    DISPLAY("Usage:\n");
    DISPLAY("      %s [args]\n", programName);
    DISPLAY("Arguments:\n");
    DISPLAY(" -p<path> : output path (directory for -n>0)\n");
    DISPLAY(" -o<path> : original data path (directory for -n>0)\n");
    DISPLAY(" -s#      : seed\n");
    DISPLAY(" -n#      : number of files to generate\n");
    DISPLAY(" -t       : run in in-memory test mode\n");
    DISPLAY(" -T#      : requested test duration hint\n");
    DISPLAY(" -v       : ignored for compatibility\n");
    DISPLAY(" -h/-H    : help\n");
    DISPLAY(" --content-size           : write content sizes\n");
    DISPLAY(" --use-dict=#             : emit and use a raw dictionary\n");
    DISPLAY(" --max-content-size-log=# : cap generated content size\n");
    DISPLAY(" --gen-blocks             : unsupported in public-only mode\n");
    DISPLAY(" --max-block-size-log=#   : unsupported in public-only mode\n");
}

int main(int argc, char** argv)
{
    unsigned seed = 0;
    int seedSet = 0;
    unsigned numFiles = 0;
    unsigned testDuration = 0;
    int testMode = 0;
    const char* path = NULL;
    const char* origPath = NULL;
    advancedOptions_t advanced;
    int argNb;

    memset(&advanced, 0, sizeof(advanced));
    advanced.dictSize = 10U << 10;
    advanced.maxContentSizeLog = 20U;

    for (argNb = 1; argNb < argc; ++argNb) {
        const char* argument = argv[argNb];
        if (argument[0] != '-') {
            usage(argv[0]);
            return 1;
        }
        argument++;
        while (*argument != 0) {
            switch (*argument) {
            case 'h':
            case 'H':
                usage(argv[0]);
                return 0;
            case 'v':
                argument++;
                break;
            case 's':
                argument++;
                seed = readInt(&argument);
                seedSet = 1;
                break;
            case 'n':
                argument++;
                numFiles = readInt(&argument);
                break;
            case 'T':
                argument++;
                testDuration = readU32FromChar(&argument);
                break;
            case 'o':
                argument++;
                origPath = argument;
                argument += strlen(argument);
                break;
            case 'p':
                argument++;
                path = argument;
                argument += strlen(argument);
                break;
            case 't':
                argument++;
                testMode = 1;
                break;
            case '-':
                argument++;
                if (strcmp(argument, "content-size") == 0) {
                    advanced.contentSize = 1;
                } else if (longCommandWArg(&argument, "use-dict=")) {
                    advanced.useDict = 1;
                    advanced.dictSize = readU32FromChar(&argument);
                } else if (strcmp(argument, "gen-blocks") == 0) {
                    DISPLAY("decodecorpus: --gen-blocks is unsupported in public-only mode\n");
                    return 1;
                } else if (longCommandWArg(&argument, "max-block-size-log=")) {
                    DISPLAY("decodecorpus: --max-block-size-log is unsupported in public-only mode\n");
                    return 1;
                } else if (longCommandWArg(&argument, "max-content-size-log=")) {
                    advanced.maxContentSizeLog = readU32FromChar(&argument);
                    if (advanced.maxContentSizeLog > 20U) {
                        advanced.maxContentSizeLog = 20U;
                    }
                } else {
                    usage(argv[0]);
                    return 1;
                }
                argument += strlen(argument);
                break;
            default:
                usage(argv[0]);
                return 1;
            }
        }
    }

    if (!seedSet) {
        seed = makeSeed();
    }

    if (testMode) {
        return runTestMode(seed, numFiles, testDuration, &advanced);
    }

    if (path == NULL) {
        DISPLAY("Error: path is required in file generation mode\n");
        usage(argv[0]);
        return 1;
    }

    if (numFiles == 0) {
        unsigned char* dict = NULL;
        if (advanced.useDict) {
            char dictPath[4096];
            dict = (unsigned char*)malloc(advanced.dictSize);
            if (dict == NULL) {
                DISPLAY("allocation failure\n");
                return 1;
            }
            generateSample(dict, advanced.dictSize, seed ^ 0xBADCAFEU);
            if (joinPath(dictPath, sizeof(dictPath), path, "dictionary") ||
                outputBuffer(dict, advanced.dictSize, dictPath)) {
                free(dict);
                return 1;
            }
        }
        if (generateOneFile(seed, path, origPath,
                            dict,
                            advanced.useDict ? advanced.dictSize : 0,
                            &advanced)) {
            free(dict);
            return 1;
        }
        free(dict);
        return 0;
    }

    return generateCorpus(seed, numFiles, path, origPath, &advanced);
}
