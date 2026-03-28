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
#include <stddef.h>
#include <stdio.h>
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
#define ARRAY_SIZE(a) (sizeof(a) / sizeof((a)[0]))

typedef struct {
    int clevel;
    int windowLog;
    int hashLog;
    int chainLog;
    int searchLog;
    int minMatch;
    int targetLength;
    int strategy;
    int checksumFlag;
    int contentSizeFlag;
    int dictIDFlag;
} config_t;

typedef struct {
    unsigned iterations;
    size_t inputSize;
    int verbose;
    int optimize;
    unsigned optimizeMask;
    unsigned displayMask;
    unsigned explicitMask;
    const char* inputFile;
    config_t config;
} options_t;

typedef struct {
    ZSTD_CCtx* cctx;
    ZSTD_DCtx* dctx;
    void* src;
    void* compressed;
    void* decompressed;
    size_t srcSize;
    size_t compressedCapacity;
} runner_t;

enum {
    MASK_LEVEL         = 1U << 0,
    MASK_WINDOWLOG     = 1U << 1,
    MASK_HASHLOG       = 1U << 2,
    MASK_CHAINLOG      = 1U << 3,
    MASK_SEARCHLOG     = 1U << 4,
    MASK_MINMATCH      = 1U << 5,
    MASK_TARGETLENGTH  = 1U << 6,
    MASK_STRATEGY      = 1U << 7,
    MASK_CHECKSUMFLAG  = 1U << 8,
    MASK_CONTENTSIZE   = 1U << 9,
    MASK_DICTID        = 1U << 10
};

#define MASK_COMPRESSION_PARAMETERS \
    (MASK_LEVEL | MASK_WINDOWLOG | MASK_HASHLOG | MASK_CHAINLOG | MASK_SEARCHLOG | \
     MASK_MINMATCH | MASK_TARGETLENGTH | MASK_STRATEGY)
#define MASK_FRAME_FLAGS (MASK_CHECKSUMFLAG | MASK_CONTENTSIZE | MASK_DICTID)
#define MASK_ALL (MASK_COMPRESSION_PARAMETERS | MASK_FRAME_FLAGS)

typedef struct {
    const char* longName;
    const char* shortName;
    const char* displayName;
    ZSTD_cParameter param;
    size_t offset;
    unsigned mask;
} paramInfo_t;

static const paramInfo_t g_params[] = {
    { "level", "lvl", "level", ZSTD_c_compressionLevel, offsetof(config_t, clevel), MASK_LEVEL },
    { "windowLog", "wlog", "wlog", ZSTD_c_windowLog, offsetof(config_t, windowLog), MASK_WINDOWLOG },
    { "hashLog", "hlog", "hlog", ZSTD_c_hashLog, offsetof(config_t, hashLog), MASK_HASHLOG },
    { "chainLog", "clog", "clog", ZSTD_c_chainLog, offsetof(config_t, chainLog), MASK_CHAINLOG },
    { "searchLog", "slog", "slog", ZSTD_c_searchLog, offsetof(config_t, searchLog), MASK_SEARCHLOG },
    { "minMatch", "mml", "mml", ZSTD_c_minMatch, offsetof(config_t, minMatch), MASK_MINMATCH },
    { "targetLength", "tlen", "tlen", ZSTD_c_targetLength, offsetof(config_t, targetLength), MASK_TARGETLENGTH },
    { "strategy", "strat", "strat", ZSTD_c_strategy, offsetof(config_t, strategy), MASK_STRATEGY },
    { "checksumFlag", "check", "checksumFlag", ZSTD_c_checksumFlag, offsetof(config_t, checksumFlag), MASK_CHECKSUMFLAG },
    { "contentSizeFlag", "content", "contentSizeFlag", ZSTD_c_contentSizeFlag, offsetof(config_t, contentSizeFlag), MASK_CONTENTSIZE },
    { "dictIDFlag", "dictID", "dictIDFlag", ZSTD_c_dictIDFlag, offsetof(config_t, dictIDFlag), MASK_DICTID }
};

static const char* const g_defaultProfiles[] = {
    "level=1,checksumFlag=1",
    "level=3,strategy=1,windowLog=19,hashLog=18,chainLog=18,searchLog=1,minMatch=4",
    "level=5,strategy=3,windowLog=19,hashLog=18,chainLog=18,searchLog=2,minMatch=4,contentSizeFlag=0",
    "level=6,strategy=5,windowLog=20,hashLog=19,chainLog=19,searchLog=4,minMatch=5,targetLength=8",
    "level=9,strategy=7,windowLog=21,hashLog=20,chainLog=20,searchLog=5,minMatch=5,targetLength=16,checksumFlag=1",
    "level=12,strategy=8,windowLog=22,hashLog=21,chainLog=21,searchLog=6,minMatch=5,targetLength=32,dictIDFlag=0",
    "level=15,strategy=9,windowLog=23,hashLog=22,chainLog=22,searchLog=7,minMatch=6,targetLength=64,checksumFlag=1,dictIDFlag=0"
};

static void configInit(config_t* config)
{
    memset(config, 0xFF, sizeof(*config));
    config->clevel = ZSTD_CLEVEL_DEFAULT;
}

static int* configField(config_t* config, const paramInfo_t* info)
{
    return (int*)((char*)config + info->offset);
}

static const int* configFieldConst(const config_t* config, const paramInfo_t* info)
{
    return (const int*)((const char*)config + info->offset);
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

static void usage(const char* programName)
{
    printf(
        "%s\n"
        "Usage: %s [options] [file]\n"
        "  file            : optional reference file; otherwise generate a compressible sample\n"
        "  -i# or -i #     : number of passes over the built-in profile suite (default: %u)\n"
        "  -s# or -s #     : input size in bytes; accepts K/M/G suffixes (default: %u)\n"
        "  -v              : print one line per successful profile or optimizer candidate\n"
        "  --zstd=...      : run a single stable-parameter configuration\n"
        "  --optimize=...  : search over the selected public parameters\n"
        "  --display=...   : choose which parameters are printed in the final --zstd= line\n"
        "  -h              : display this help text\n",
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
    if (parsed > (unsigned long long)(size_t)-1) {
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

static int parseIntStrict(const char* text, int* value)
{
    char* end = NULL;
    long parsed = 0;

    errno = 0;
    parsed = strtol(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || parsed < INT_MIN || parsed > INT_MAX) {
        return 0;
    }

    *value = (int)parsed;
    return 1;
}

static const paramInfo_t* findParamByName(const char* name)
{
    size_t i;
    for (i = 0; i < ARRAY_SIZE(g_params); ++i) {
        if (!strcmp(name, g_params[i].longName) || !strcmp(name, g_params[i].shortName)) {
            return &g_params[i];
        }
    }
    if (!strcmp(name, "checksum")) {
        return &g_params[8];
    }
    if (!strcmp(name, "dictid")) {
        return &g_params[10];
    }
    return NULL;
}

static int parseList(const char* text,
                     config_t* config,
                     unsigned* maskOut,
                     unsigned* explicitMaskOut,
                     int allowAssignments,
                     int allowBareNames)
{
    const char* cursor = text;

    while (*cursor != '\0') {
        const char* tokenStart = cursor;
        const char* equals = NULL;
        size_t tokenLen;
        char token[64];

        while (*cursor != '\0' && *cursor != ',') {
            if (*cursor == '=') {
                equals = cursor;
            }
            cursor++;
        }
        tokenLen = (size_t)(cursor - tokenStart);
        if (tokenLen == 0 || tokenLen >= sizeof(token)) {
            return 0;
        }
        memcpy(token, tokenStart, tokenLen);
        token[tokenLen] = '\0';

        if (equals != NULL) {
            char* valueText = NULL;
            const paramInfo_t* info;
            int value = 0;

            if (!allowAssignments) {
                return 0;
            }
            valueText = strchr(token, '=');
            if (valueText == NULL || valueText == token || valueText[1] == '\0') {
                return 0;
            }
            *valueText++ = '\0';
            info = findParamByName(token);
            if (info == NULL || !parseIntStrict(valueText, &value)) {
                return 0;
            }
            *configField(config, info) = value;
            if (explicitMaskOut != NULL) {
                *explicitMaskOut |= info->mask;
            }
        } else {
            const paramInfo_t* info;
            if (!allowBareNames) {
                return 0;
            }
            if (!strcmp(token, "all")) {
                *maskOut |= MASK_ALL;
            } else if (!strcmp(token, "compressionParameters") || !strcmp(token, "cParams")) {
                *maskOut |= MASK_COMPRESSION_PARAMETERS;
            } else if (!strcmp(token, "frameFlags")) {
                *maskOut |= MASK_FRAME_FLAGS;
            } else {
                info = findParamByName(token);
                if (info == NULL) {
                    return 0;
                }
                *maskOut |= info->mask;
            }
        }

        if (*cursor == ',') {
            cursor++;
        }
    }

    return 1;
}

static int loadFile(const char* path, void** buffer, size_t* size)
{
    FILE* file = fopen(path, "rb");
    long length;
    void* data;

    if (file == NULL) {
        fprintf(stderr, "paramgrill: cannot open %s\n", path);
        return 1;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        fprintf(stderr, "paramgrill: cannot seek %s\n", path);
        return 1;
    }
    length = ftell(file);
    if (length < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        fprintf(stderr, "paramgrill: cannot size %s\n", path);
        return 1;
    }
    data = malloc((size_t)length);
    if (data == NULL) {
        fclose(file);
        fprintf(stderr, "paramgrill: not enough memory for %s\n", path);
        return 1;
    }
    if (fread(data, 1, (size_t)length, file) != (size_t)length) {
        fclose(file);
        free(data);
        fprintf(stderr, "paramgrill: cannot read %s\n", path);
        return 1;
    }
    fclose(file);
    *buffer = data;
    *size = (size_t)length;
    return 0;
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

static int initRunner(runner_t* runner, size_t inputSize)
{
    memset(runner, 0, sizeof(*runner));
    runner->compressedCapacity = ZSTD_compressBound(inputSize);
    if (checkZstd(runner->compressedCapacity, "computing compression bound")) {
        return 1;
    }

    runner->cctx = ZSTD_createCCtx();
    runner->dctx = ZSTD_createDCtx();
    runner->src = malloc(inputSize);
    runner->compressed = malloc(runner->compressedCapacity);
    runner->decompressed = malloc(inputSize);
    runner->srcSize = inputSize;

    if (runner->cctx == NULL || runner->dctx == NULL ||
        runner->src == NULL || runner->compressed == NULL ||
        runner->decompressed == NULL) {
        fprintf(stderr, "paramgrill: allocation failure\n");
        return 1;
    }

    return 0;
}

static void freeRunner(runner_t* runner)
{
    ZSTD_freeDCtx(runner->dctx);
    ZSTD_freeCCtx(runner->cctx);
    free(runner->decompressed);
    free(runner->compressed);
    free(runner->src);
}

static int applyConfig(ZSTD_CCtx* cctx, const config_t* config)
{
    size_t i;

    if (checkZstd(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, config->clevel),
                  "setting compressionLevel")) {
        return 1;
    }

    for (i = 1; i < ARRAY_SIZE(g_params); ++i) {
        int const value = *configFieldConst(config, &g_params[i]);
        if (value >= 0) {
            if (checkZstd(ZSTD_CCtx_setParameter(cctx, g_params[i].param, value),
                          g_params[i].longName)) {
                return 1;
            }
        }
    }

    return 0;
}

static int isReasonableConfig(const config_t* config)
{
    int const windowLog = config->windowLog;
    int const hashLog = config->hashLog;
    int const chainLog = config->chainLog;
    int const searchLog = config->searchLog;

    if (windowLog >= 0 && hashLog >= 0 && hashLog > windowLog + 1) {
        return 0;
    }
    if (windowLog >= 0 && chainLog >= 0 && chainLog > windowLog + 1) {
        return 0;
    }
    if (chainLog >= 0 && searchLog >= 0 && searchLog > chainLog) {
        return 0;
    }
    return 1;
}

static int verifyFrameMetadata(const config_t* config,
                               const void* compressed,
                               size_t compressedSize,
                               size_t srcSize)
{
    int const expectKnownContentSize = config->contentSizeFlag != 0;
    size_t const frameSize = ZSTD_findFrameCompressedSize(compressed, compressedSize);
    unsigned long long const contentSize = ZSTD_getFrameContentSize(compressed, compressedSize);

    if (checkZstd(frameSize, "finding frame size")) {
        return 1;
    }
    if (frameSize != compressedSize) {
        fprintf(stderr, "paramgrill: incomplete frame accounting (%zu != %zu)\n",
                frameSize, compressedSize);
        return 1;
    }

    if (contentSize == ZSTD_CONTENTSIZE_ERROR) {
        fprintf(stderr, "paramgrill: invalid content size in generated frame\n");
        return 1;
    }

    if (expectKnownContentSize) {
        if (contentSize != (unsigned long long)srcSize) {
            fprintf(stderr, "paramgrill: unexpected known content size (%llu != %zu)\n",
                    contentSize, srcSize);
            return 1;
        }
    } else {
        if (contentSize != ZSTD_CONTENTSIZE_UNKNOWN) {
            fprintf(stderr, "paramgrill: expected hidden content size, got %llu\n", contentSize);
            return 1;
        }
    }

    return 0;
}

static int evaluateConfig(runner_t* runner,
                          const config_t* config,
                          size_t* compressedSizeOut,
                          int strict)
{
    size_t compressedSize;
    size_t regeneratedSize;

    if (!isReasonableConfig(config)) {
        return strict ? 1 : 2;
    }

    if (checkZstd(ZSTD_CCtx_reset(runner->cctx, ZSTD_reset_session_and_parameters),
                  "resetting CCtx")) {
        return strict ? 1 : 2;
    }
    if (applyConfig(runner->cctx, config)) {
        return strict ? 1 : 2;
    }

    compressedSize = ZSTD_compress2(runner->cctx, runner->compressed,
                                    runner->compressedCapacity,
                                    runner->src, runner->srcSize);
    if (ZSTD_isError(compressedSize)) {
        if (strict) {
            fprintf(stderr, "paramgrill: compressing failed: %s\n", ZSTD_getErrorName(compressedSize));
            return 1;
        }
        return 2;
    }
    if (verifyFrameMetadata(config, runner->compressed, compressedSize, runner->srcSize)) {
        return strict ? 1 : 2;
    }

    if (checkZstd(ZSTD_DCtx_reset(runner->dctx, ZSTD_reset_session_and_parameters),
                  "resetting DCtx")) {
        return strict ? 1 : 2;
    }

    regeneratedSize = ZSTD_decompressDCtx(runner->dctx, runner->decompressed, runner->srcSize,
                                          runner->compressed, compressedSize);
    if (ZSTD_isError(regeneratedSize)) {
        if (strict) {
            fprintf(stderr, "paramgrill: decompressing failed: %s\n", ZSTD_getErrorName(regeneratedSize));
            return 1;
        }
        return 2;
    }
    if (regeneratedSize != runner->srcSize ||
        memcmp(runner->decompressed, runner->src, runner->srcSize) != 0) {
        fprintf(stderr, "paramgrill: round-trip mismatch\n");
        return strict ? 1 : 2;
    }

    *compressedSizeOut = compressedSize;
    return 0;
}

static int appendCandidate(int* candidates, unsigned* count, int value)
{
    unsigned i;
    for (i = 0; i < *count; ++i) {
        if (candidates[i] == value) {
            return 0;
        }
    }
    candidates[*count] = value;
    (*count)++;
    return 0;
}

static unsigned buildCandidates(const paramInfo_t* info, int currentValue, int* candidates)
{
    ZSTD_bounds const bounds = ZSTD_cParam_getBounds(info->param);
    unsigned count = 0;
    unsigned i;
    static const int levelValues[] = { 1, 3, 5, 7, 9, 12, 15, 18 };
    static const int logValues[] = { 16, 17, 18, 19, 20, 21, 22, 23 };
    static const int searchValues[] = { 1, 2, 3, 4, 5, 6, 7 };
    static const int matchValues[] = { 3, 4, 5, 6, 7 };
    static const int targetValues[] = { 4, 8, 16, 32, 64, 96, 128 };

    if (ZSTD_isError(bounds.error)) {
        return 0;
    }

    appendCandidate(candidates, &count, bounds.lowerBound);
    appendCandidate(candidates, &count, bounds.upperBound);
    if (currentValue >= 0) {
        appendCandidate(candidates, &count, currentValue);
    }

    if (info->mask == MASK_LEVEL) {
        for (i = 0; i < ARRAY_SIZE(levelValues); ++i) {
            if (levelValues[i] >= bounds.lowerBound && levelValues[i] <= bounds.upperBound) {
                appendCandidate(candidates, &count, levelValues[i]);
            }
        }
        appendCandidate(candidates, &count, ZSTD_defaultCLevel());
    } else if (info->mask == MASK_WINDOWLOG || info->mask == MASK_HASHLOG || info->mask == MASK_CHAINLOG) {
        for (i = 0; i < ARRAY_SIZE(logValues); ++i) {
            if (logValues[i] >= bounds.lowerBound && logValues[i] <= bounds.upperBound) {
                appendCandidate(candidates, &count, logValues[i]);
            }
        }
    } else if (info->mask == MASK_SEARCHLOG) {
        for (i = 0; i < ARRAY_SIZE(searchValues); ++i) {
            if (searchValues[i] >= bounds.lowerBound && searchValues[i] <= bounds.upperBound) {
                appendCandidate(candidates, &count, searchValues[i]);
            }
        }
    } else if (info->mask == MASK_MINMATCH) {
        for (i = 0; i < ARRAY_SIZE(matchValues); ++i) {
            if (matchValues[i] >= bounds.lowerBound && matchValues[i] <= bounds.upperBound) {
                appendCandidate(candidates, &count, matchValues[i]);
            }
        }
    } else if (info->mask == MASK_TARGETLENGTH) {
        for (i = 0; i < ARRAY_SIZE(targetValues); ++i) {
            if (targetValues[i] >= bounds.lowerBound && targetValues[i] <= bounds.upperBound) {
                appendCandidate(candidates, &count, targetValues[i]);
            }
        }
    } else if (info->mask == MASK_STRATEGY) {
        for (i = (unsigned)bounds.lowerBound; i <= (unsigned)bounds.upperBound; ++i) {
            appendCandidate(candidates, &count, (int)i);
        }
    } else {
        appendCandidate(candidates, &count, 0);
        appendCandidate(candidates, &count, 1);
    }

    return count;
}

static void printConfig(FILE* stream, const config_t* config, unsigned mask)
{
    size_t i;
    int printed = 0;

    fprintf(stream, "--zstd=");
    for (i = 0; i < ARRAY_SIZE(g_params); ++i) {
        int const value = *configFieldConst(config, &g_params[i]);
        if ((mask & g_params[i].mask) == 0U) {
            continue;
        }
        if (i != 0 && value < 0) {
            continue;
        }
        if (printed) {
            fprintf(stream, ",");
        }
        fprintf(stream, "%s=%d", g_params[i].displayName,
                i == 0 ? config->clevel : value);
        printed = 1;
    }
    if (!printed) {
        fprintf(stream, "level=%d", config->clevel);
    }
}

static int optimizeConfig(runner_t* runner,
                          const options_t* options,
                          config_t* bestConfig,
                          size_t* bestSizeOut)
{
    size_t bestSize = 0;
    size_t i;

    if (evaluateConfig(runner, bestConfig, &bestSize, 1)) {
        return 1;
    }

    for (i = 0; i < ARRAY_SIZE(g_params); ++i) {
        const paramInfo_t* const info = &g_params[i];
        int candidates[32];
        unsigned count;
        unsigned c;
        config_t localBest = *bestConfig;
        size_t localBestSize = bestSize;

        if ((options->optimizeMask & info->mask) == 0U) {
            continue;
        }

        count = buildCandidates(info, *configFieldConst(bestConfig, info), candidates);
        for (c = 0; c < count; ++c) {
            config_t trial = *bestConfig;
            size_t trialSize = 0;
            int* const field = configField(&trial, info);

            *field = candidates[c];
            if (evaluateConfig(runner, &trial, &trialSize, 0) != 0) {
                continue;
            }
            if (options->verbose) {
                printf("paramgrill: optimizer candidate ");
                printConfig(stdout, &trial, options->displayMask ? options->displayMask : options->optimizeMask);
                printf(" -> %zu bytes\n", trialSize);
            }
            if (trialSize < localBestSize) {
                localBest = trial;
                localBestSize = trialSize;
            }
        }

        *bestConfig = localBest;
        bestSize = localBestSize;
    }

    *bestSizeOut = bestSize;
    return 0;
}

static int parseArgs(int argc, const char** argv, options_t* options)
{
    int argNb;

    memset(options, 0, sizeof(*options));
    options->iterations = DEFAULT_ITERATIONS;
    options->inputSize = DEFAULT_INPUT_SIZE;
    configInit(&options->config);

    for (argNb = 1; argNb < argc; ++argNb) {
        const char* argument = argv[argNb];

        if (!strcmp(argument, "-h") || !strcmp(argument, "--help")) {
            usage(argv[0]);
            return 0;
        }
        if (!strcmp(argument, "-v")) {
            options->verbose = 1;
            continue;
        }
        if (!strcmp(argument, "-i")) {
            if (argNb + 1 >= argc || !parseUnsigned(argv[++argNb], &options->iterations)) {
                fprintf(stderr, "paramgrill: invalid iteration count\n");
                return 1;
            }
            continue;
        }
        if (!strncmp(argument, "-i", 2)) {
            if (!parseUnsigned(argument + 2, &options->iterations)) {
                fprintf(stderr, "paramgrill: invalid iteration count\n");
                return 1;
            }
            continue;
        }
        if (!strcmp(argument, "-s")) {
            if (argNb + 1 >= argc || !parseSize(argv[++argNb], &options->inputSize)) {
                fprintf(stderr, "paramgrill: invalid input size\n");
                return 1;
            }
            continue;
        }
        if (!strncmp(argument, "-s", 2)) {
            if (!parseSize(argument + 2, &options->inputSize)) {
                fprintf(stderr, "paramgrill: invalid input size\n");
                return 1;
            }
            continue;
        }
        if (longCommandWArg(&argument, "--zstd=")) {
            if (!parseList(argument, &options->config, &options->optimizeMask,
                           &options->explicitMask, 1, 0)) {
                fprintf(stderr, "paramgrill: invalid --zstd= format\n");
                return 1;
            }
            continue;
        }
        if (longCommandWArg(&argument, "--optimize=")) {
            options->optimize = 1;
            if (!parseList(argument, &options->config, &options->optimizeMask,
                           &options->explicitMask, 1, 1)) {
                fprintf(stderr, "paramgrill: invalid --optimize= format\n");
                return 1;
            }
            continue;
        }
        if (longCommandWArg(&argument, "--display=")) {
            if (!parseList(argument, &options->config, &options->displayMask,
                           NULL, 0, 1)) {
                fprintf(stderr, "paramgrill: invalid --display= format\n");
                return 1;
            }
            continue;
        }
        if (argument[0] == '-') {
            fprintf(stderr, "paramgrill: unsupported argument: %s\n", argv[argNb]);
            return 1;
        }
        if (options->inputFile != NULL) {
            fprintf(stderr, "paramgrill: only one input file is supported\n");
            return 1;
        }
        options->inputFile = argv[argNb];
    }

    if (options->optimize && options->optimizeMask == 0U) {
        options->optimizeMask = MASK_ALL;
    }

    return 2;
}

int main(int argc, const char** argv)
{
    options_t options;
    runner_t runner;
    int parseResult;
    unsigned iteration;
    int result = 1;

    parseResult = parseArgs(argc, argv, &options);
    if (parseResult != 2) {
        return parseResult;
    }

    if (options.inputSize < MIN_INPUT_SIZE) {
        fprintf(stderr, "paramgrill: input size must be at least %u bytes\n", MIN_INPUT_SIZE);
        return 1;
    }

    if (initRunner(&runner, options.inputSize)) {
        freeRunner(&runner);
        return 1;
    }

    if (options.inputFile != NULL) {
        free(runner.src);
        runner.src = NULL;
        if (loadFile(options.inputFile, &runner.src, &runner.srcSize)) {
            freeRunner(&runner);
            return 1;
        }
        runner.compressedCapacity = ZSTD_compressBound(runner.srcSize);
        if (checkZstd(runner.compressedCapacity, "computing compression bound")) {
            freeRunner(&runner);
            return 1;
        }
        free(runner.compressed);
        free(runner.decompressed);
        runner.compressed = malloc(runner.compressedCapacity);
        runner.decompressed = malloc(runner.srcSize);
        if (runner.compressed == NULL || runner.decompressed == NULL) {
            fprintf(stderr, "paramgrill: allocation failure\n");
            freeRunner(&runner);
            return 1;
        }
    } else {
        fillBuffer(runner.src, runner.srcSize, 0xC0FFEE00U);
    }

    if (options.optimize) {
        config_t bestConfig = options.config;
        size_t bestSize = 0;
        if (optimizeConfig(&runner, &options, &bestConfig, &bestSize)) {
            freeRunner(&runner);
            return 1;
        }
        printf("paramgrill: best ");
        printConfig(stdout, &bestConfig,
                    options.displayMask ? options.displayMask :
                    (options.optimizeMask ? options.optimizeMask : MASK_ALL));
        printf(" -> %zu bytes\n", bestSize);
        result = 0;
        freeRunner(&runner);
        return result;
    }

    if (options.explicitMask != 0U) {
        size_t compressedSize = 0;
        for (iteration = 0; iteration < options.iterations; ++iteration) {
            if (options.inputFile == NULL) {
                fillBuffer(runner.src, runner.srcSize, 0xC0FFEE00U + iteration);
            }
            if (evaluateConfig(&runner, &options.config, &compressedSize, 1)) {
                freeRunner(&runner);
                return 1;
            }
        }
        printf("paramgrill: ");
        printConfig(stdout, &options.config,
                    options.displayMask ? options.displayMask :
                    (options.explicitMask ? options.explicitMask : MASK_ALL));
        printf(" -> %zu bytes across %u iteration(s)\n", compressedSize, options.iterations);
        result = 0;
        freeRunner(&runner);
        return result;
    }

    for (iteration = 0; iteration < options.iterations; ++iteration) {
        size_t profileIndex;
        if (options.inputFile == NULL) {
            fillBuffer(runner.src, runner.srcSize, 0xC0FFEE00U + iteration);
        }
        for (profileIndex = 0; profileIndex < ARRAY_SIZE(g_defaultProfiles); ++profileIndex) {
            config_t config;
            unsigned profileMask = 0;
            unsigned explicitMask = 0;
            size_t compressedSize = 0;

            configInit(&config);
            if (!parseList(g_defaultProfiles[profileIndex], &config, &profileMask, &explicitMask, 1, 0)) {
                fprintf(stderr, "paramgrill: internal profile parse failure\n");
                freeRunner(&runner);
                return 1;
            }
            if (evaluateConfig(&runner, &config, &compressedSize, 1)) {
                freeRunner(&runner);
                return 1;
            }
            if (options.verbose) {
                printf("paramgrill: ");
                printConfig(stdout, &config, explicitMask);
                printf(" passed (%zu bytes -> %zu bytes)\n", runner.srcSize, compressedSize);
            }
        }
    }

    printf("paramgrill: %zu public parameter profiles passed across %u iteration(s)\n",
           ARRAY_SIZE(g_defaultProfiles), options.iterations);
    result = 0;

    freeRunner(&runner);
    return result;
}
