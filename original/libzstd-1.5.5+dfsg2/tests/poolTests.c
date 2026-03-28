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
#define MIN(a, b) ((a) < (b) ? (a) : (b))
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

typedef struct {
    unsigned char* data;
    size_t size;
    size_t capacity;
} buffer_t;

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
    static const char* const fragments[] = {
        "tenant=alpha;kind=session;payload=",
        "tenant=beta;kind=session;payload=",
        "tenant=gamma;kind=metric;payload=",
        "tenant=delta;kind=record;payload="
    };
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    unsigned char* out = (unsigned char*)buffer;
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        size_t const fragment = nextRandom(&state) % ARRAY_SIZE(fragments);
        size_t const fragLen = strlen(fragments[fragment]);
        size_t i;
        for (i = 0; i < fragLen && pos < size; ++i) {
            out[pos++] = (unsigned char)fragments[fragment][i];
        }
        for (i = 0; i < 128U && pos < size; ++i) {
            out[pos++] = (unsigned char)alphabet[nextRandom(&state) % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) {
            out[pos++] = '\n';
        }
    }
}

static void bufferFree(buffer_t* buffer)
{
    free(buffer->data);
    buffer->data = NULL;
    buffer->size = 0;
    buffer->capacity = 0;
}

static int bufferReserve(buffer_t* buffer, size_t neededCapacity)
{
    size_t newCapacity = buffer->capacity == 0 ? 4096U : buffer->capacity;
    unsigned char* grown;

    if (neededCapacity <= buffer->capacity) {
        return 0;
    }
    while (newCapacity < neededCapacity) {
        if (newCapacity > ((size_t)-1) / 2U) {
            newCapacity = neededCapacity;
            break;
        }
        newCapacity *= 2U;
    }
    grown = (unsigned char*)realloc(buffer->data, newCapacity);
    if (grown == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }
    buffer->data = grown;
    buffer->capacity = newCapacity;
    return 0;
}

static int bufferAppend(buffer_t* buffer, const void* src, size_t srcSize)
{
    if (srcSize == 0) {
        return 0;
    }
    if (bufferReserve(buffer, buffer->size + srcSize)) {
        return 1;
    }
    memcpy(buffer->data + buffer->size, src, srcSize);
    buffer->size += srcSize;
    return 0;
}

static void bufferClear(buffer_t* buffer)
{
    buffer->size = 0;
}

static int setMultithreadParameters(ZSTD_CCtx* cctx,
                                    unsigned workers,
                                    int level,
                                    size_t jobSize,
                                    int overlapLog)
{
    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, level));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, (int)workers));
    if (workers != 0U) {
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_jobSize, (int)jobSize));
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_overlapLog, overlapLog));
    }
    return 0;
}

static int compressContinue(ZSTD_CCtx* cctx,
                            buffer_t* compressed,
                            const void* src,
                            size_t srcSize)
{
    ZSTD_inBuffer in;
    in.src = src;
    in.size = srcSize;
    in.pos = 0;

    while (in.pos < in.size) {
        unsigned char outChunk[257];
        ZSTD_outBuffer out;
        size_t remaining;

        out.dst = outChunk;
        out.size = sizeof(outChunk);
        out.pos = 0;
        remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_continue);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(..., continue) failed: %s\n",
              ZSTD_getErrorName(remaining));
        if (bufferAppend(compressed, outChunk, out.pos)) {
            return 1;
        }
        CHECK(out.pos != 0 || in.pos != 0,
              "stream compression made no forward progress\n");
    }

    return 0;
}

static int compressDrain(ZSTD_CCtx* cctx,
                         buffer_t* compressed,
                         ZSTD_EndDirective directive)
{
    ZSTD_inBuffer in;
    in.src = "";
    in.size = 0;
    in.pos = 0;

    while (1) {
        unsigned char outChunk[257];
        ZSTD_outBuffer out;
        size_t remaining;

        out.dst = outChunk;
        out.size = sizeof(outChunk);
        out.pos = 0;
        remaining = ZSTD_compressStream2(cctx, &out, &in, directive);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(..., drain) failed: %s\n",
              ZSTD_getErrorName(remaining));
        if (bufferAppend(compressed, outChunk, out.pos)) {
            return 1;
        }
        if (remaining == 0) {
            return 0;
        }
        CHECK(out.pos != 0, "drain operation stalled with pending output\n");
    }
}

static int verifyDecompressedPrefix(const buffer_t* compressed,
                                    const void* expected,
                                    size_t expectedSize,
                                    const void* dict,
                                    size_t dictSize,
                                    int requireFrameEnd)
{
    size_t const decodedCapacity = MAX(expectedSize + 64U, (size_t)64U);
    unsigned char* const decoded = (unsigned char*)malloc(decodedCapacity);
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    ZSTD_inBuffer in;
    size_t decodedSize = 0;
    size_t ret = 1;

    if (decoded == NULL || dctx == NULL) {
        DISPLAY("allocation failure\n");
        free(decoded);
        ZSTD_freeDCtx(dctx);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    if (dict != NULL && dictSize != 0U) {
        CHECK_Z(ZSTD_DCtx_loadDictionary(dctx, dict, dictSize));
    }

    in.src = compressed->data;
    in.size = compressed->size;
    in.pos = 0;
    while (in.pos < in.size) {
        ZSTD_outBuffer out;
        out.dst = decoded + decodedSize;
        out.size = decodedCapacity - decodedSize;
        out.pos = 0;
        ret = ZSTD_decompressStream(dctx, &out, &in);
        CHECK(!ZSTD_isError(ret), "ZSTD_decompressStream failed: %s\n", ZSTD_getErrorName(ret));
        decodedSize += out.pos;
        CHECK(decodedSize <= expectedSize,
              "stream decode produced too much data (%u > %u)\n",
              (unsigned)decodedSize, (unsigned)expectedSize);
        CHECK(out.pos != 0 || in.pos == in.size,
              "stream decode stalled before consuming all input\n");
    }

    CHECK(decodedSize == expectedSize, "decoded prefix size mismatch: %u != %u\n",
          (unsigned)decodedSize, (unsigned)expectedSize);
    CHECK(memcmp(decoded, expected, expectedSize) == 0, "decoded prefix mismatch\n");
    if (requireFrameEnd) {
        CHECK(ret == 0, "frame did not end cleanly\n");
        CHECK(ZSTD_getFrameContentSize(compressed->data, compressed->size)
                  == (unsigned long long)expectedSize,
              "frame content size mismatch\n");
        CHECK(ZSTD_findFrameCompressedSize(compressed->data, compressed->size) == compressed->size,
              "frame compressed size mismatch\n");
    }

    free(decoded);
    ZSTD_freeDCtx(dctx);
    return 0;
}

static int testFlushOrdering(unsigned workers, int useDict)
{
    size_t const dictSize = useDict ? (64U * 1024U) : 0U;
    size_t const segmentSize = 192U * 1024U + 37U;
    size_t const segmentCount = 10U;
    size_t const srcSize = dictSize + segmentCount * segmentSize;
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    const unsigned char* payload;
    size_t payloadSize;
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    buffer_t compressed;
    size_t segment;

    memset(&compressed, 0, sizeof(compressed));

    if (src == NULL || cctx == NULL) {
        DISPLAY("allocation failure\n");
        free(src);
        ZSTD_freeCCtx(cctx);
        return 1;
    }

    generateSample(src, srcSize, 17U + workers + (useDict ? 101U : 0U));
    payload = src + dictSize;
    payloadSize = srcSize - dictSize;

    if (setMultithreadParameters(cctx, workers, 4, 128U * 1024U, 4)) {
        free(src);
        ZSTD_freeCCtx(cctx);
        return 1;
    }
    if (useDict) {
        CHECK_Z(ZSTD_CCtx_loadDictionary(cctx, src, dictSize));
    }
    CHECK_Z(ZSTD_CCtx_setPledgedSrcSize(cctx, payloadSize));

    for (segment = 0; segment < segmentCount; ++segment) {
        size_t const prefixSize = (segment + 1U) * segmentSize;
        if (compressContinue(cctx, &compressed, payload + segment * segmentSize, segmentSize) ||
            compressDrain(cctx, &compressed, ZSTD_e_flush) ||
            verifyDecompressedPrefix(&compressed,
                                     payload,
                                     prefixSize,
                                     useDict ? src : NULL,
                                     dictSize,
                                     0)) {
            bufferFree(&compressed);
            free(src);
            ZSTD_freeCCtx(cctx);
            return 1;
        }
    }

    if (compressDrain(cctx, &compressed, ZSTD_e_end) ||
        verifyDecompressedPrefix(&compressed,
                                 payload,
                                 payloadSize,
                                 useDict ? src : NULL,
                                 dictSize,
                                 1)) {
        bufferFree(&compressed);
        free(src);
        ZSTD_freeCCtx(cctx);
        return 1;
    }

    bufferFree(&compressed);
    free(src);
    ZSTD_freeCCtx(cctx);
    return 0;
}

static int compressFullFrame(ZSTD_CCtx* cctx,
                             buffer_t* compressed,
                             const void* src,
                             size_t srcSize)
{
    size_t const firstChunk = srcSize / 3U;
    size_t const secondChunk = srcSize / 4U;
    size_t const thirdChunk = srcSize - firstChunk - secondChunk;

    bufferClear(compressed);
    CHECK_Z(ZSTD_CCtx_setPledgedSrcSize(cctx, srcSize));
    if (compressContinue(cctx, compressed, src, firstChunk) ||
        compressContinue(cctx, compressed, (const unsigned char*)src + firstChunk, secondChunk) ||
        compressContinue(cctx, compressed, (const unsigned char*)src + firstChunk + secondChunk, thirdChunk) ||
        compressDrain(cctx, compressed, ZSTD_e_end)) {
        return 1;
    }
    return 0;
}

static int testWorkerReconfiguration(void)
{
    static const struct {
        unsigned workers;
        int level;
        size_t jobSize;
        int overlapLog;
    } configs[] = {
        { 4U, 5, 128U * 1024U, 4 },
        { 2U, 3, 192U * 1024U, 3 },
        { 0U, 2, 0U, 0 }
    };
    size_t const srcSize = 3U * 1024U * 1024U + 123U;
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    buffer_t compressed;
    size_t i;

    memset(&compressed, 0, sizeof(compressed));

    if (src == NULL || cctx == NULL) {
        DISPLAY("allocation failure\n");
        free(src);
        ZSTD_freeCCtx(cctx);
        return 1;
    }

    generateSample(src, srcSize, 99U);
    for (i = 0; i < ARRAY_SIZE(configs); ++i) {
        if (setMultithreadParameters(cctx,
                                     configs[i].workers,
                                     configs[i].level,
                                     configs[i].jobSize,
                                     configs[i].overlapLog) ||
            compressFullFrame(cctx, &compressed, src, srcSize) ||
            verifyDecompressedPrefix(&compressed, src, srcSize, NULL, 0, 1)) {
            bufferFree(&compressed);
            free(src);
            ZSTD_freeCCtx(cctx);
            return 1;
        }
    }

    bufferFree(&compressed);
    free(src);
    ZSTD_freeCCtx(cctx);
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

static int testMultithreadParameterSurface(void)
{
    static const ZSTD_cParameter params[] = {
        ZSTD_c_nbWorkers,
        ZSTD_c_jobSize,
        ZSTD_c_overlapLog
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

int main(void)
{
    if (testMultithreadParameterSurface() ||
        testFlushOrdering(1U, 0) ||
        testFlushOrdering(4U, 0) ||
        testFlushOrdering(3U, 1) ||
        testWorkerReconfiguration()) {
        return 1;
    }

    DISPLAY("poolTests: public multithread correctness tests passed\n");
    return 0;
}
