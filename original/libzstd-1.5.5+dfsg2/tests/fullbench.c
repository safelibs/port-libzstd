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

#define PROGRAM_DESCRIPTION "Zstandard public API benchmark"
#define AUTHOR "Meta"
#define DEFAULT_CLEVEL 1
#define DEFAULT_ITERATIONS 6
#define DEFAULT_COMPRESSIBILITY 50
#define DEFAULT_SAMPLE_SIZE 10000000U
#define CHUNK_SIZE (32 * 1024U)

#define DISPLAY(...) fprintf(stderr, __VA_ARGS__)
#define ARRAY_SIZE(a) (sizeof(a) / sizeof((a)[0]))
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
    int clevel;
    unsigned compressibility;
    size_t sampleSize;
    unsigned iterations;
    unsigned benchNb;
    int windowLog;
    int hashLog;
    int chainLog;
    int searchLog;
    int minMatch;
    int targetLength;
    int strategy;
} benchOptions_t;

typedef struct {
    const char* name;
    int id;
} benchInfo_t;

static const benchInfo_t g_benches[] = {
    { "compress2", 1 },
    { "decompress", 2 },
    { "compress2_freshCCtx", 3 },
    { "compressStream2", 41 },
    { "decompressStream", 42 },
    { "compressStream2_freshCCtx", 43 },
};

static unsigned readU32FromChar(const char** stringPtr)
{
    unsigned result = 0;
    while ((**stringPtr >= '0') && (**stringPtr <= '9')) {
        result *= 10;
        result += (unsigned)(**stringPtr - '0');
        (*stringPtr)++;
    }
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

static double nowSeconds(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1000000000.0;
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

static void generateSample(void* buffer, size_t size, unsigned compressibility)
{
    unsigned char* out = (unsigned char*)buffer;
    unsigned seed = 0x9E3779B9U;
    size_t i;
    size_t const history = 8192;

    for (i = 0; i < size; ++i) {
        unsigned const rnd = nextRandom(&seed);
        if (i > 0 && (rnd % 100U) < compressibility) {
            size_t const back = (rnd % MIN(i, history)) + 1;
            out[i] = out[i - back];
        } else {
            out[i] = (unsigned char)rnd;
        }
    }
}

static int applyParams(ZSTD_CCtx* cctx, const benchOptions_t* options)
{
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, options->clevel));
    if (options->windowLog >= 0) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, options->windowLog));
    }
    if (options->hashLog >= 0) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_hashLog, options->hashLog));
    }
    if (options->chainLog >= 0) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_chainLog, options->chainLog));
    }
    if (options->searchLog >= 0) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_searchLog, options->searchLog));
    }
    if (options->minMatch >= 0) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_minMatch, options->minMatch));
    }
    if (options->targetLength >= 0) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_targetLength, options->targetLength));
    }
    if (options->strategy >= 0) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, options->strategy));
    }
    return 0;
}

static int streamCompressOnce(ZSTD_CCtx* cctx,
                              const benchOptions_t* options,
                              const void* src,
                              size_t srcSize,
                              void* dst,
                              size_t dstCapacity,
                              size_t* written)
{
    const unsigned char* input = (const unsigned char*)src;
    unsigned char* output = (unsigned char*)dst;
    size_t srcPos = 0;
    size_t dstPos = 0;

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    if (applyParams(cctx, options)) {
        return 1;
    }

    while (srcPos < srcSize) {
        size_t const chunkSize = MIN(srcSize - srcPos, (size_t)CHUNK_SIZE);
        ZSTD_inBuffer in = { input + srcPos, chunkSize, 0 };
        while (in.pos < in.size) {
            ZSTD_outBuffer out = { output + dstPos, dstCapacity - dstPos, 0 };
            CHECK_Z(ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_continue));
            dstPos += out.pos;
        }
        srcPos += chunkSize;
    }

    for (;;) {
        ZSTD_inBuffer in = { NULL, 0, 0 };
        ZSTD_outBuffer out = { output + dstPos, dstCapacity - dstPos, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_end);
        if (ZSTD_isError(remaining)) {
            DISPLAY("ZSTD_compressStream2(..., ZSTD_e_end): %s\n",
                    ZSTD_getErrorName(remaining));
            return 1;
        }
        dstPos += out.pos;
        if (remaining == 0) {
            break;
        }
    }

    *written = dstPos;
    return 0;
}

static int streamDecompressOnce(const void* src,
                                size_t srcSize,
                                void* dst,
                                size_t dstCapacity,
                                size_t* written)
{
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    const unsigned char* input = (const unsigned char*)src;
    unsigned char* output = (unsigned char*)dst;
    size_t srcPos = 0;
    size_t dstPos = 0;

    if (dctx == NULL) {
        DISPLAY("ZSTD_createDCtx failed\n");
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only));
    while (srcPos < srcSize) {
        size_t const inChunk = MIN(srcSize - srcPos, (size_t)(CHUNK_SIZE / 2));
        ZSTD_inBuffer in = { input + srcPos, inChunk, 0 };
        while (in.pos < in.size) {
            ZSTD_outBuffer out = { output + dstPos, dstCapacity - dstPos, 0 };
            size_t const ret = ZSTD_decompressStream(dctx, &out, &in);
            if (ZSTD_isError(ret)) {
                DISPLAY("ZSTD_decompressStream: %s\n", ZSTD_getErrorName(ret));
                ZSTD_freeDCtx(dctx);
                return 1;
            }
            dstPos += out.pos;
        }
        srcPos += inChunk;
    }

    ZSTD_freeDCtx(dctx);
    *written = dstPos;
    return 0;
}

static int benchCompress(const void* src, size_t srcSize,
                         const benchOptions_t* options, int fresh,
                         double* speedOut)
{
    ZSTD_CCtx* cctx = fresh ? NULL : ZSTD_createCCtx();
    void* dst = malloc(ZSTD_compressBound(srcSize));
    double start;
    double elapsed;
    unsigned loop;

    if (dst == NULL || (!fresh && cctx == NULL)) {
        free(dst);
        ZSTD_freeCCtx(cctx);
        DISPLAY("Allocation failure\n");
        return 1;
    }

    start = nowSeconds();
    for (loop = 0; loop < options->iterations; ++loop) {
        size_t size;
        if (fresh) {
            cctx = ZSTD_createCCtx();
            if (cctx == NULL) {
                free(dst);
                DISPLAY("ZSTD_createCCtx failed\n");
                return 1;
            }
        } else {
            CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        }
        if (applyParams(cctx, options)) {
            free(dst);
            ZSTD_freeCCtx(cctx);
            return 1;
        }
        size = ZSTD_compress2(cctx, dst, ZSTD_compressBound(srcSize), src, srcSize);
        if (ZSTD_isError(size)) {
            DISPLAY("ZSTD_compress2: %s\n", ZSTD_getErrorName(size));
            free(dst);
            ZSTD_freeCCtx(cctx);
            return 1;
        }
        if (fresh) {
            ZSTD_freeCCtx(cctx);
            cctx = NULL;
        }
    }
    elapsed = nowSeconds() - start;
    if (elapsed <= 0.0) {
        elapsed = 1e-9;
    }
    *speedOut = (double)(srcSize * options->iterations) / elapsed / 1000000.0;

    free(dst);
    ZSTD_freeCCtx(cctx);
    return 0;
}

static int benchDecompress(const void* src, size_t srcSize,
                           const benchOptions_t* options,
                           double* speedOut)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    void* compressed = malloc(ZSTD_compressBound(srcSize));
    void* decoded = malloc(srcSize);
    size_t compressedSize;
    double start;
    double elapsed;
    unsigned loop;

    if (cctx == NULL || dctx == NULL || compressed == NULL || decoded == NULL) {
        DISPLAY("Allocation failure\n");
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    if (applyParams(cctx, options)) {
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        free(compressed);
        free(decoded);
        return 1;
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

    start = nowSeconds();
    for (loop = 0; loop < options->iterations; ++loop) {
        size_t const decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize);
        if (ZSTD_isError(decodedSize) || decodedSize != srcSize || memcmp(decoded, src, srcSize) != 0) {
            DISPLAY("ZSTD_decompressDCtx failed\n");
            ZSTD_freeCCtx(cctx);
            ZSTD_freeDCtx(dctx);
            free(compressed);
            free(decoded);
            return 1;
        }
    }
    elapsed = nowSeconds() - start;
    if (elapsed <= 0.0) {
        elapsed = 1e-9;
    }
    *speedOut = (double)(srcSize * options->iterations) / elapsed / 1000000.0;

    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    free(compressed);
    free(decoded);
    return 0;
}

static int benchStreamCompress(const void* src, size_t srcSize,
                               const benchOptions_t* options, int fresh,
                               double* speedOut)
{
    ZSTD_CCtx* cctx = fresh ? NULL : ZSTD_createCCtx();
    void* dst = malloc(ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize());
    double start;
    double elapsed;
    unsigned loop;

    if (dst == NULL || (!fresh && cctx == NULL)) {
        DISPLAY("Allocation failure\n");
        free(dst);
        ZSTD_freeCCtx(cctx);
        return 1;
    }

    start = nowSeconds();
    for (loop = 0; loop < options->iterations; ++loop) {
        size_t written = 0;
        if (fresh) {
            cctx = ZSTD_createCCtx();
            if (cctx == NULL) {
                DISPLAY("ZSTD_createCCtx failed\n");
                free(dst);
                return 1;
            }
        }
        if (streamCompressOnce(cctx, options, src, srcSize,
                               dst, ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize(),
                               &written)) {
            free(dst);
            ZSTD_freeCCtx(cctx);
            return 1;
        }
        if (fresh) {
            ZSTD_freeCCtx(cctx);
            cctx = NULL;
        }
    }
    elapsed = nowSeconds() - start;
    if (elapsed <= 0.0) {
        elapsed = 1e-9;
    }
    *speedOut = (double)(srcSize * options->iterations) / elapsed / 1000000.0;

    free(dst);
    ZSTD_freeCCtx(cctx);
    return 0;
}

static int benchStreamDecompress(const void* src, size_t srcSize,
                                 const benchOptions_t* options,
                                 double* speedOut)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    void* compressed = malloc(ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize());
    void* decoded = malloc(srcSize);
    size_t compressedSize = 0;
    double start;
    double elapsed;
    unsigned loop;

    if (cctx == NULL || compressed == NULL || decoded == NULL) {
        DISPLAY("Allocation failure\n");
        ZSTD_freeCCtx(cctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    if (streamCompressOnce(cctx, options, src, srcSize,
                           compressed, ZSTD_compressBound(srcSize) + ZSTD_CStreamOutSize(),
                           &compressedSize)) {
        ZSTD_freeCCtx(cctx);
        free(compressed);
        free(decoded);
        return 1;
    }

    start = nowSeconds();
    for (loop = 0; loop < options->iterations; ++loop) {
        size_t written = 0;
        if (streamDecompressOnce(compressed, compressedSize, decoded, srcSize, &written)) {
            ZSTD_freeCCtx(cctx);
            free(compressed);
            free(decoded);
            return 1;
        }
        if (written != srcSize || memcmp(decoded, src, srcSize) != 0) {
            DISPLAY("stream decompression produced invalid output\n");
            ZSTD_freeCCtx(cctx);
            free(compressed);
            free(decoded);
            return 1;
        }
    }
    elapsed = nowSeconds() - start;
    if (elapsed <= 0.0) {
        elapsed = 1e-9;
    }
    *speedOut = (double)(srcSize * options->iterations) / elapsed / 1000000.0;

    ZSTD_freeCCtx(cctx);
    free(compressed);
    free(decoded);
    return 0;
}

static int runBench(unsigned benchNb,
                    const void* src,
                    size_t srcSize,
                    const benchOptions_t* options)
{
    double speed = 0.0;
    const char* benchName = NULL;
    int result = 1;
    size_t index;

    for (index = 0; index < ARRAY_SIZE(g_benches); ++index) {
        if ((unsigned)g_benches[index].id == benchNb) {
            benchName = g_benches[index].name;
            break;
        }
    }

    if (benchName == NULL) {
        DISPLAY("bench %u is not available in the public-only fullbench harness\n", benchNb);
        return 1;
    }

    switch (benchNb) {
    case 1:
        result = benchCompress(src, srcSize, options, 0, &speed);
        break;
    case 2:
        result = benchDecompress(src, srcSize, options, &speed);
        break;
    case 3:
        result = benchCompress(src, srcSize, options, 1, &speed);
        break;
    case 41:
        result = benchStreamCompress(src, srcSize, options, 0, &speed);
        break;
    case 42:
        result = benchStreamDecompress(src, srcSize, options, &speed);
        break;
    case 43:
        result = benchStreamCompress(src, srcSize, options, 1, &speed);
        break;
    default:
        break;
    }

    if (result == 0) {
        DISPLAY("%2u#%-24s : %8.1f MB/s\n", benchNb, benchName, speed);
    }
    return result;
}

static int usage(const char* exename)
{
    DISPLAY("Usage:\n");
    DISPLAY("      %s [arg] [file1 ... fileN]\n", exename);
    DISPLAY("Arguments:\n");
    DISPLAY(" -H/-h  : help\n");
    DISPLAY(" -b#    : benchmark only public benchmark #\n");
    DISPLAY(" -l#    : compression level (default: %d)\n", DEFAULT_CLEVEL);
    DISPLAY(" -P#    : sample compressibility percent (default: %u)\n", DEFAULT_COMPRESSIBILITY);
    DISPLAY(" -B#    : sample size (default: %u)\n", DEFAULT_SAMPLE_SIZE);
    DISPLAY(" -i#    : iteration loops (default: %u)\n", DEFAULT_ITERATIONS);
    DISPLAY(" --zstd=: public parameter overrides (wlog,hlog,clog,slog,mml,tlen,strat,level)\n");
    return 0;
}

static int parseZstdParams(benchOptions_t* options, const char* argument)
{
    const char* cursor = argument;
    while (*cursor != 0) {
        int* target = NULL;
        if (longCommandWArg(&cursor, "windowLog=") || longCommandWArg(&cursor, "wlog=")) {
            target = &options->windowLog;
        } else if (longCommandWArg(&cursor, "hashLog=") || longCommandWArg(&cursor, "hlog=")) {
            target = &options->hashLog;
        } else if (longCommandWArg(&cursor, "chainLog=") || longCommandWArg(&cursor, "clog=")) {
            target = &options->chainLog;
        } else if (longCommandWArg(&cursor, "searchLog=") || longCommandWArg(&cursor, "slog=")) {
            target = &options->searchLog;
        } else if (longCommandWArg(&cursor, "minMatch=") || longCommandWArg(&cursor, "mml=")) {
            target = &options->minMatch;
        } else if (longCommandWArg(&cursor, "targetLength=") || longCommandWArg(&cursor, "tlen=")) {
            target = &options->targetLength;
        } else if (longCommandWArg(&cursor, "strategy=") || longCommandWArg(&cursor, "strat=")) {
            target = &options->strategy;
        } else if (longCommandWArg(&cursor, "level=") || longCommandWArg(&cursor, "lvl=")) {
            options->clevel = (int)readU32FromChar(&cursor);
        } else {
            DISPLAY("invalid --zstd= parameter\n");
            return 1;
        }

        if (target != NULL) {
            *target = (int)readU32FromChar(&cursor);
        }

        if (*cursor == ',') {
            cursor++;
            continue;
        }
        if (*cursor != 0) {
            DISPLAY("invalid --zstd= format\n");
            return 1;
        }
    }
    return 0;
}

static int loadFile(const char* path, void** buffer, size_t* size)
{
    FILE* file = fopen(path, "rb");
    long length;
    void* data;

    if (file == NULL) {
        DISPLAY("cannot open %s\n", path);
        return 1;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        DISPLAY("cannot seek %s\n", path);
        return 1;
    }
    length = ftell(file);
    if (length < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        DISPLAY("cannot size %s\n", path);
        return 1;
    }
    data = malloc((size_t)length);
    if (data == NULL) {
        fclose(file);
        DISPLAY("not enough memory for %s\n", path);
        return 1;
    }
    if (fread(data, 1, (size_t)length, file) != (size_t)length) {
        fclose(file);
        free(data);
        DISPLAY("cannot read %s\n", path);
        return 1;
    }
    fclose(file);
    *buffer = data;
    *size = (size_t)length;
    return 0;
}

int main(int argc, const char** argv)
{
    benchOptions_t options;
    int argNb;
    int filenamesStart = 0;
    int result = 0;

    memset(&options, 0, sizeof(options));
    options.clevel = DEFAULT_CLEVEL;
    options.compressibility = DEFAULT_COMPRESSIBILITY;
    options.sampleSize = DEFAULT_SAMPLE_SIZE;
    options.iterations = DEFAULT_ITERATIONS;
    options.windowLog = -1;
    options.hashLog = -1;
    options.chainLog = -1;
    options.searchLog = -1;
    options.minMatch = -1;
    options.targetLength = -1;
    options.strategy = -1;

    DISPLAY("*** %s %s, by %s ***\n",
            PROGRAM_DESCRIPTION, ZSTD_versionString(), AUTHOR);

    for (argNb = 1; argNb < argc; ++argNb) {
        const char* argument = argv[argNb];
        if (longCommandWArg(&argument, "--zstd=")) {
            if (parseZstdParams(&options, argument)) {
                return 1;
            }
            continue;
        }
        if (argument[0] == '-') {
            argument++;
            while (*argument != 0) {
                switch (*argument) {
                case 'h':
                case 'H':
                    return usage(argv[0]);
                case 'b':
                    argument++;
                    options.benchNb = readU32FromChar(&argument);
                    break;
                case 'l':
                    argument++;
                    options.clevel = (int)readU32FromChar(&argument);
                    break;
                case 'P':
                    argument++;
                    options.compressibility = readU32FromChar(&argument);
                    break;
                case 'B':
                    argument++;
                    options.sampleSize = (size_t)readU32FromChar(&argument);
                    break;
                case 'i':
                    argument++;
                    options.iterations = readU32FromChar(&argument);
                    break;
                default:
                    DISPLAY("Wrong parameters\n");
                    return usage(argv[0]);
                }
            }
            continue;
        }
        filenamesStart = argNb;
        break;
    }

    if (filenamesStart == 0) {
        void* sample = malloc(options.sampleSize);
        size_t index;
        if (sample == NULL) {
            DISPLAY("not enough memory\n");
            return 1;
        }
        generateSample(sample, options.sampleSize, options.compressibility);
        if (options.benchNb != 0) {
            result = runBench(options.benchNb, sample, options.sampleSize, &options);
        } else {
            for (index = 0; index < ARRAY_SIZE(g_benches); ++index) {
                result |= runBench((unsigned)g_benches[index].id, sample, options.sampleSize, &options);
            }
        }
        free(sample);
    } else {
        for (; filenamesStart < argc; ++filenamesStart) {
            void* buffer = NULL;
            size_t size = 0;
            size_t index;
            if (loadFile(argv[filenamesStart], &buffer, &size)) {
                return 1;
            }
            DISPLAY("File %s (%u bytes)\n", argv[filenamesStart], (unsigned)size);
            if (options.benchNb != 0) {
                result |= runBench(options.benchNb, buffer, size, &options);
            } else {
                for (index = 0; index < ARRAY_SIZE(g_benches); ++index) {
                    result |= runBench((unsigned)g_benches[index].id, buffer, size, &options);
                }
            }
            free(buffer);
        }
    }

    return result;
}
