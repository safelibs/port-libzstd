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

#include "zdict.h"
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
    static const char* const fragments[] = {
        "{\"tenant\":\"alpha\",\"region\":\"west\",\"kind\":\"session\",\"payload\":\"",
        "{\"tenant\":\"beta\",\"region\":\"east\",\"kind\":\"session\",\"payload\":\"",
        "{\"tenant\":\"alpha\",\"region\":\"west\",\"kind\":\"metric\",\"payload\":\"",
        "{\"tenant\":\"gamma\",\"region\":\"north\",\"kind\":\"record\",\"payload\":\""
    };
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    unsigned char* out = (unsigned char*)buffer;
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        const char* fragment = fragments[nextRandom(&state) % ARRAY_SIZE(fragments)];
        size_t fragLen = strlen(fragment);
        size_t i;
        for (i = 0; i < fragLen && pos < size; ++i) {
            out[pos++] = (unsigned char)fragment[i];
        }
        for (i = 0; i < 96U && pos < size; ++i) {
            out[pos++] = (unsigned char)alphabet[nextRandom(&state) % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) {
            out[pos++] = '"';
        }
        if (pos < size) {
            out[pos++] = '}';
        }
        if (pos < size) {
            out[pos++] = '\n';
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

static int compressLegacyStream(const void* src,
                                size_t srcSize,
                                int level,
                                unsigned seed,
                                void** compressedPtr,
                                size_t* compressedSizePtr)
{
    ZSTD_CStream* const cstream = ZSTD_createCStream();
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize) + (4U * ZSTD_CStreamOutSize()));
    size_t const capacity = ZSTD_compressBound(srcSize) + (4U * ZSTD_CStreamOutSize());
    size_t dstPos = 0;
    ZSTD_inBuffer in;

    if (cstream == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCStream(cstream);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_initCStream(cstream, level));
    (void)seed;
    in.src = src;
    in.size = srcSize;
    in.pos = 0;
    while (in.pos < in.size) {
        ZSTD_outBuffer out = { compressed + dstPos, capacity - dstPos, 0 };
        CHECK_Z(ZSTD_compressStream(cstream, &out, &in));
        dstPos += out.pos;
    }

    for (;;) {
        ZSTD_outBuffer out = { compressed + dstPos, capacity - dstPos, 0 };
        size_t const remaining = ZSTD_endStream(cstream, &out);
        CHECK(!ZSTD_isError(remaining), "ZSTD_endStream failed: %s\n", ZSTD_getErrorName(remaining));
        dstPos += out.pos;
        if (remaining == 0) {
            break;
        }
    }

    ZSTD_freeCStream(cstream);
    *compressedPtr = compressed;
    *compressedSizePtr = dstPos;
    return 0;
}

static int compressNewStream(const void* src,
                             size_t srcSize,
                             int level,
                             unsigned workers,
                             const void* dict,
                             size_t dictSize,
                             unsigned seed,
                             void** compressedPtr,
                             size_t* compressedSizePtr)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(srcSize) + (4U * ZSTD_CStreamOutSize()));
    size_t const capacity = ZSTD_compressBound(srcSize) + (4U * ZSTD_CStreamOutSize());
    size_t dstPos = 0;
    ZSTD_inBuffer in;

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

    (void)seed;
    in.src = src;
    in.size = srcSize;
    in.pos = 0;

    for (;;) {
        ZSTD_outBuffer out = { compressed + dstPos, capacity - dstPos, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_end);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(..., end) failed: %s\n",
              ZSTD_getErrorName(remaining));
        dstPos += out.pos;
        if (remaining == 0) {
            break;
        }
    }

    ZSTD_freeCCtx(cctx);
    *compressedPtr = compressed;
    *compressedSizePtr = dstPos;
    return 0;
}

static int streamDecodeFully(ZSTD_DCtx* dctx,
                             const void* compressed,
                             size_t compressedSize,
                             void* decoded,
                             size_t decodedCapacity,
                             size_t maxInChunk,
                             size_t maxOutChunk,
                             int randomized,
                             unsigned* seed,
                             size_t* decodedSize)
{
    size_t srcPos = 0;
    size_t dstPos = 0;
    size_t ret = 1;
    size_t loops = 0;

    while (1) {
        size_t inChunk = 0;
        size_t outChunk = 0;
        ZSTD_inBuffer in;
        ZSTD_outBuffer out;
        size_t const remainingInput = compressedSize - srcPos;
        size_t const remainingOutput = decodedCapacity - dstPos;
        if (remainingInput != 0) {
            inChunk = MIN(remainingInput, maxInChunk);
            if (randomized) {
                inChunk = MIN(remainingInput, (size_t)((nextRandom(seed) % maxInChunk) + 1U));
            }
        }
        if (remainingOutput != 0) {
            outChunk = MIN(remainingOutput, maxOutChunk);
            if (randomized) {
                outChunk = MIN(remainingOutput, (size_t)((nextRandom(seed) % maxOutChunk) + 1U));
            }
        }

        CHECK(inChunk != 0 || outChunk != 0, "decoder helper ran out of room\n");

        in.src = compressed;
        in.size = srcPos + inChunk;
        in.pos = srcPos;
        out.dst = decoded;
        out.size = dstPos + outChunk;
        out.pos = dstPos;

        ret = ZSTD_decompressStream(dctx, &out, &in);
        CHECK(!ZSTD_isError(ret), "ZSTD_decompressStream failed: %s\n", ZSTD_getErrorName(ret));

        srcPos = in.pos;
        dstPos = out.pos;

        if (ret == 0 && srcPos == compressedSize) {
            *decodedSize = dstPos;
            return 0;
        }

        CHECK(++loops < compressedSize + decodedCapacity + 4096U,
              "decoder helper exceeded progress limit\n");
    }
}

static int runLegacyRoundTrip(const void* src, size_t srcSize, unsigned seed)
{
    void* compressed = NULL;
    size_t compressedSize = 0;
    ZSTD_DCtx* dctx = NULL;
    unsigned char* decoded = NULL;
    size_t decodedSize = 0;

    if (compressLegacyStream(src, srcSize, 3, seed, &compressed, &compressedSize)) {
        free(compressed);
        return 1;
    }

    dctx = ZSTD_createDCtx();
    decoded = (unsigned char*)malloc(srcSize);
    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        return 1;
    }

    (void)seed;
    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize);
    CHECK(!ZSTD_isError(decodedSize), "legacy ZSTD_decompressDCtx failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "legacy stream decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "legacy stream round-trip mismatch\n");

    ZSTD_freeDCtx(dctx);
    free(decoded);
    free(compressed);
    return 0;
}

static int runNewRoundTrip(const void* src,
                           size_t srcSize,
                           int level,
                           unsigned workers,
                           const void* dict,
                           size_t dictSize,
                           unsigned seed)
{
    void* compressed = NULL;
    size_t compressedSize = 0;
    ZSTD_DCtx* dctx = NULL;
    unsigned char* decoded = NULL;
    size_t decodedSize = 0;

    if (compressNewStream(src, srcSize, level, workers, dict, dictSize, seed,
                          &compressed, &compressedSize)) {
        free(compressed);
        return 1;
    }

    dctx = ZSTD_createDCtx();
    decoded = (unsigned char*)malloc(srcSize);
    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    if (dict != NULL && dictSize != 0) {
        CHECK_Z(ZSTD_DCtx_loadDictionary(dctx, dict, dictSize));
    }
    (void)seed;
    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, compressed, compressedSize);
    CHECK(!ZSTD_isError(decodedSize), "new API ZSTD_decompressDCtx failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "new API stream decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "new API stream round-trip mismatch\n");

    ZSTD_freeDCtx(dctx);
    free(decoded);
    free(compressed);
    return 0;
}

static int testSkippableFrame(void)
{
    unsigned char payload[64];
    unsigned char frame[SKIPPABLE_FRAME_HEADER_SIZE + sizeof(payload)];
    unsigned char decoded[sizeof(payload)];
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    ZSTD_inBuffer in;
    ZSTD_outBuffer out;
    size_t frameSize;
    size_t ret;
    size_t i;

    if (dctx == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    for (i = 0; i < sizeof(payload); ++i) {
        payload[i] = (unsigned char)(i ^ 0x5AU);
    }

    writeLE32(frame, ZSTD_MAGIC_SKIPPABLE_START + 11U);
    writeLE32(frame + 4U, (unsigned)sizeof(payload));
    memcpy(frame + SKIPPABLE_FRAME_HEADER_SIZE, payload, sizeof(payload));
    frameSize = SKIPPABLE_FRAME_HEADER_SIZE + sizeof(payload);
    CHECK(ZSTD_findFrameCompressedSize(frame, frameSize) == frameSize,
          "skippable frame compressed size mismatch\n");
    CHECK(ZSTD_decompress(decoded, 0, frame, frameSize) == 0,
          "single-pass skippable frame decode should produce zero bytes\n");

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    in.src = frame;
    in.size = frameSize;
    in.pos = 0;
    out.dst = decoded;
    out.size = sizeof(decoded);
    out.pos = 0;
    ret = ZSTD_decompressStream(dctx, &out, &in);
    CHECK(!ZSTD_isError(ret), "ZSTD_decompressStream(skippable) failed: %s\n", ZSTD_getErrorName(ret));
    CHECK(ret == 0, "skippable frame did not end the frame cleanly\n");
    CHECK(in.pos == frameSize, "skippable frame was not fully consumed\n");
    CHECK(out.pos == 0, "skippable frame should not produce output\n");

    ZSTD_freeDCtx(dctx);
    return 0;
}

static int testEarlyHeaderError(void)
{
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    const unsigned char bogus[3] = { 0, 0, 0 };
    unsigned char dst[32];
    ZSTD_inBuffer in;
    ZSTD_outBuffer out;
    size_t ret;

    if (dctx == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    in.src = bogus;
    in.size = sizeof(bogus);
    in.pos = 0;
    out.dst = dst;
    out.size = sizeof(dst);
    out.pos = 0;
    ret = ZSTD_decompressStream(dctx, &out, &in);
    CHECK(ZSTD_isError(ret), "short invalid header unexpectedly succeeded\n");

    ZSTD_freeDCtx(dctx);
    return 0;
}

static int testNoForwardProgress(const void* compressed, size_t compressedSize, size_t decodedSize)
{
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    unsigned char* const decoded = (unsigned char*)malloc(decodedSize);
    ZSTD_inBuffer in;
    ZSTD_outBuffer out;
    size_t ret = 0;
    unsigned attempt;

    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    in.src = compressed;
    in.size = compressedSize;
    in.pos = 0;
    out.dst = decoded;
    out.size = decodedSize - 1U;
    out.pos = 0;

    for (attempt = 0; attempt < 256U; ++attempt) {
        ret = ZSTD_decompressStream(dctx, &out, &in);
        if (ZSTD_isError(ret)) {
            break;
        }
    }
    if (checkErrorCode(ret, ZSTD_error_noForwardProgress_destFull, "no-forward-progress")) {
        ZSTD_freeDCtx(dctx);
        free(decoded);
        return 1;
    }

    ZSTD_freeDCtx(dctx);
    free(decoded);
    return 0;
}

static int trainDictionary(const void* src,
                           size_t srcSize,
                           void** dictBufferPtr,
                           size_t* dictSizePtr)
{
    size_t* sampleSizes = NULL;
    unsigned char* dictBuffer = NULL;
    size_t sampleSize = 1024U;
    unsigned nbSamples;
    size_t dictSize;
    unsigned i;

    if (srcSize < 64U * sampleSize) {
        sampleSize = 512U;
    }
    nbSamples = (unsigned)(srcSize / sampleSize);
    nbSamples = MIN(nbSamples, 64U);
    CHECK(nbSamples >= 16U, "not enough sample data to train a dictionary\n");

    sampleSizes = (size_t*)malloc((size_t)nbSamples * sizeof(*sampleSizes));
    dictBuffer = (unsigned char*)malloc(4096U);
    if (sampleSizes == NULL || dictBuffer == NULL) {
        DISPLAY("allocation failure\n");
        free(sampleSizes);
        free(dictBuffer);
        return 1;
    }

    for (i = 0; i < nbSamples; ++i) {
        sampleSizes[i] = sampleSize;
    }

    dictSize = ZDICT_trainFromBuffer(dictBuffer, 4096U, src, sampleSizes, nbSamples);
    CHECK(!ZDICT_isError(dictSize), "ZDICT_trainFromBuffer failed\n");
    CHECK(dictSize != 0, "dictionary trainer returned an empty dictionary\n");

    free(sampleSizes);
    *dictBufferPtr = dictBuffer;
    *dictSizePtr = dictSize;
    return 0;
}

static int testDictionaryTraining(const void* src, size_t srcSize, unsigned workers, unsigned seed)
{
    void* dictBuffer = NULL;
    size_t dictSize = 0;
    void* plainCompressed = NULL;
    void* dictCompressed = NULL;
    size_t plainSize = 0;
    size_t dictCompressedSize = 0;
    ZSTD_DCtx* dctx = NULL;
    unsigned char* decoded = NULL;
    size_t decodedSize = 0;

    if (trainDictionary(src, srcSize, &dictBuffer, &dictSize)) {
        free(dictBuffer);
        return 1;
    }

    if (compressNewStream(src, srcSize, 3, workers, NULL, 0, seed,
                          &plainCompressed, &plainSize)) {
        free(dictBuffer);
        free(plainCompressed);
        return 1;
    }
    if (compressNewStream(src, srcSize, 3, workers, dictBuffer, dictSize, seed ^ 0xA5A5A5A5U,
                          &dictCompressed, &dictCompressedSize)) {
        free(dictBuffer);
        free(plainCompressed);
        free(dictCompressed);
        return 1;
    }
    CHECK(dictCompressedSize < plainSize, "trained dictionary did not improve compression size\n");

    dctx = ZSTD_createDCtx();
    decoded = (unsigned char*)malloc(srcSize);
    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(dictBuffer);
        free(plainCompressed);
        free(dictCompressed);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, dictCompressed, dictCompressedSize);
    CHECK(ZSTD_isError(decodedSize), "dictionary decode without dictionary unexpectedly succeeded\n");

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_DCtx_loadDictionary(dctx, dictBuffer, dictSize));
    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, dictCompressed, dictCompressedSize);
    CHECK(!ZSTD_isError(decodedSize), "trained dictionary ZSTD_decompressDCtx failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "trained dictionary decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "trained dictionary round-trip mismatch\n");

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only));
    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, dictCompressed, dictCompressedSize);
    CHECK(!ZSTD_isError(decodedSize), "dictionary persistence ZSTD_decompressDCtx failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "dictionary persistence decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "dictionary persistence mismatch\n");

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    decodedSize = ZSTD_decompressDCtx(dctx, decoded, srcSize, dictCompressed, dictCompressedSize);
    CHECK(ZSTD_isError(decodedSize), "dictionary should be cleared after resetting parameters\n");

    ZSTD_freeDCtx(dctx);
    free(decoded);
    free(dictBuffer);
    free(plainCompressed);
    free(dictCompressed);
    return 0;
}

static int testByteByByteInput(const void* compressed, size_t compressedSize,
                               const void* expected, size_t expectedSize)
{
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    unsigned char* const decoded = (unsigned char*)malloc(expectedSize);
    size_t srcPos = 0;
    size_t dstPos = 0;
    size_t loops = 0;

    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    while (1) {
        ZSTD_inBuffer in = {
            compressed,
            MIN(compressedSize, srcPos + 1U),
            srcPos
        };
        ZSTD_outBuffer out = {
            decoded,
            expectedSize,
            dstPos
        };
        size_t const ret = ZSTD_decompressStream(dctx, &out, &in);
        CHECK(!ZSTD_isError(ret), "byte-by-byte ZSTD_decompressStream failed: %s\n",
              ZSTD_getErrorName(ret));
        srcPos = in.pos;
        dstPos = out.pos;
        if (ret == 0 && srcPos == compressedSize) {
            break;
        }
        CHECK(++loops < compressedSize + expectedSize + 4096U,
              "byte-by-byte decoder exceeded progress limit\n");
    }

    CHECK(dstPos == expectedSize, "byte-by-byte decoded size mismatch\n");
    CHECK(memcmp(decoded, expected, expectedSize) == 0, "byte-by-byte round-trip mismatch\n");

    ZSTD_freeDCtx(dctx);
    free(decoded);
    return 0;
}

static int runStreamingCoverage(const options_t* options)
{
    size_t const srcSize = 384U * 1024U;
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    void* compressed = NULL;
    size_t compressedSize = 0;
    unsigned seed = 0x1234ABCDU;
    ZSTD_DCtx* dctx = NULL;
    unsigned char* decoded = NULL;
    size_t decodedSize = 0;

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }
    generateSample(src, srcSize, seed);

    if (!options->newapi && runLegacyRoundTrip(src, srcSize, seed ^ 0x11111111U)) {
        free(src);
        return 1;
    }
    if (runNewRoundTrip(src, srcSize, 4, 0, NULL, 0, seed ^ 0x22222222U)) {
        free(src);
        return 1;
    }
    if (testSkippableFrame()) {
        free(src);
        return 1;
    }
    if (testEarlyHeaderError()) {
        free(src);
        return 1;
    }
    if (testDictionaryTraining(src, srcSize, 0, seed ^ 0x33333333U)) {
        free(src);
        return 1;
    }

    if (compressNewStream(src, srcSize, 5, 0, NULL, 0, seed ^ 0x44444444U,
                          &compressed, &compressedSize)) {
        free(src);
        free(compressed);
        return 1;
    }

    dctx = ZSTD_createDCtx();
    decoded = (unsigned char*)malloc(srcSize);
    if (dctx == NULL || decoded == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        free(src);
        return 1;
    }

    CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
    if (streamDecodeFully(dctx, compressed, compressedSize, decoded, srcSize,
                          compressedSize, srcSize, 0, &seed, &decodedSize)) {
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        free(src);
        return 1;
    }
    CHECK(decodedSize == srcSize, "byte-by-byte decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "byte-by-byte round-trip mismatch\n");
    if (testByteByByteInput(compressed, compressedSize, src, srcSize)) {
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        free(src);
        return 1;
    }

    if (testNoForwardProgress(compressed, compressedSize, srcSize)) {
        ZSTD_freeDCtx(dctx);
        free(decoded);
        free(compressed);
        free(src);
        return 1;
    }
    ZSTD_freeDCtx(dctx);
    free(decoded);
    free(compressed);
    free(src);
    return 0;
}

static int runIteration(unsigned iteration, const options_t* options)
{
    unsigned seed = 0xC001D00DU ^ (iteration * 0x9E3779B9U);
    size_t const srcSize = 65536U + (nextRandom(&seed) % (512U * 1024U));
    unsigned char* const src = (unsigned char*)malloc(srcSize);
    unsigned workers = 0;
    int const level = 1 + (int)(iteration % 6U);

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }
    generateSample(src, srcSize, seed);

    if (!options->newapi && runLegacyRoundTrip(src, srcSize, seed ^ 0x55AA55AAU)) {
        free(src);
        return 1;
    }
    if (runNewRoundTrip(src, srcSize, level, workers, NULL, 0, seed ^ 0xAA55AA55U)) {
        free(src);
        return 1;
    }
    if (options->verbose >= 2) {
        DISPLAY("test%3u : streaming round-trip cases passed\n", iteration + 1U);
    }

    free(src);
    return 0;
}

static int usage(const char* programName)
{
    DISPLAY("Usage:\n");
    DISPLAY("      %s [-v] [--newapi] [-T#] [-t#]\n", programName);
    DISPLAY(" -v       : increase verbosity\n");
    DISPLAY(" --newapi : focus on the public ZSTD_compressStream2() path\n");
    DISPLAY(" -T#      : requested fuzz duration hint\n");
    DISPLAY(" -t#      : maximum worker count to test (default: 1)\n");
    return 0;
}

int main(int argc, char** argv)
{
    options_t options;
    unsigned iterations;
    unsigned argNb;

    memset(&options, 0, sizeof(options));
    options.maxWorkers = 1;

    for (argNb = 1; argNb < (unsigned)argc; ++argNb) {
        const char* argument = argv[argNb];
        if (!strcmp(argument, "--newapi")) {
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

    if (runStreamingCoverage(&options)) {
        return 1;
    }

    iterations = options.durationSeconds == 0 ? 8U : MIN(options.durationSeconds / 6U + 8U, 24U);
    if (iterations == 0) {
        iterations = 8U;
    }
    for (argNb = 0; argNb < iterations; ++argNb) {
        if (runIteration(argNb, &options)) {
            return 1;
        }
    }

    DISPLAY("zstreamtest: public streaming API checks passed (%u cases)\n", iterations);
    return 0;
}
