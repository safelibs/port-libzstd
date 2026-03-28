/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "zstd.h"
#include "zstd_errors.h"

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

#define CHECK(cond, ...)            \
    do {                            \
        if (!(cond)) {              \
            DISPLAY(__VA_ARGS__);   \
            return 1;               \
        }                           \
    } while (0)

static const unsigned char invalidRepCode[] = {
    0x37, 0xa4, 0x30, 0xec, 0x2a, 0x00, 0x00, 0x00, 0x39, 0x10, 0xc0, 0xc2,
    0xa6, 0x00, 0x0c, 0x30, 0xc0, 0x00, 0x03, 0x0c, 0x30, 0x20, 0x72, 0xf8,
    0xb4, 0x6d, 0x4b, 0x9f, 0xfc, 0x97, 0x29, 0x49, 0xb2, 0xdf, 0x4b, 0x29,
    0x7d, 0x4a, 0xfc, 0x83, 0x18, 0x22, 0x75, 0x23, 0x24, 0x44, 0x4d, 0x02,
    0xb7, 0x97, 0x96, 0xf6, 0xcb, 0xd1, 0xcf, 0xe8, 0x22, 0xea, 0x27, 0x36,
    0xb7, 0x2c, 0x40, 0x46, 0x01, 0x08, 0x23, 0x01, 0x00, 0x00, 0x06, 0x1e,
    0x3c, 0x83, 0x81, 0xd6, 0x18, 0xd4, 0x12, 0x3a, 0x04, 0x00, 0x80, 0x03,
    0x08, 0x0e, 0x12, 0x1c, 0x12, 0x11, 0x0d, 0x0e, 0x0a, 0x0b, 0x0a, 0x09,
    0x10, 0x0c, 0x09, 0x05, 0x04, 0x03, 0x06, 0x06, 0x06, 0x02, 0x00, 0x03,
    0x00, 0x00, 0x02, 0x02, 0x00, 0x04, 0x06, 0x03, 0x06, 0x08, 0x24, 0x6b,
    0x0d, 0x01, 0x10, 0x04, 0x81, 0x07, 0x00, 0x00, 0x04, 0xb9, 0x58, 0x18,
    0x06, 0x59, 0x92, 0x43, 0xce, 0x28, 0xa5, 0x08, 0x88, 0xc0, 0x80, 0x88,
    0x8c, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x00, 0x00
};

typedef struct {
    const unsigned char* data;
    size_t size;
} dictionary_t;

typedef struct {
    unsigned char* data;
    size_t size;
} owned_buffer_t;

static const dictionary_t invalidDictionaries[] = {
    { invalidRepCode, sizeof(invalidRepCode) },
    { NULL, 0 }
};

static void freeOwnedBuffer(owned_buffer_t* buffer)
{
    free(buffer->data);
    buffer->data = NULL;
    buffer->size = 0;
}

static const char* findPathSeparator(const char* path)
{
    const char* slash = strrchr(path, '/');
    const char* backslash = strrchr(path, '\\');
    if (slash == NULL) {
        return backslash;
    }
    if (backslash == NULL) {
        return slash;
    }
    return slash > backslash ? slash : backslash;
}

static int buildPathFromFile(char* dst,
                             size_t dstCapacity,
                             const char* fileName,
                             const char* relativePath)
{
    const char* separator = findPathSeparator(fileName);
    size_t dirSize;
    size_t relSize = strlen(relativePath);

    if (separator == NULL) {
        return 0;
    }
    dirSize = (size_t)(separator - fileName) + 1U;
    if (dirSize + relSize + 1U > dstCapacity) {
        return 0;
    }
    memcpy(dst, fileName, dirSize);
    memcpy(dst + dirSize, relativePath, relSize + 1U);
    return 1;
}

static int tryLoadFile(const char* path, owned_buffer_t* buffer)
{
    FILE* file = fopen(path, "rb");
    long fileSize;
    size_t readSize;
    unsigned char* data;

    if (file == NULL) {
        return 0;
    }

    if (fseek(file, 0, SEEK_END) != 0) {
        DISPLAY("failed to seek %s\n", path);
        fclose(file);
        return -1;
    }
    fileSize = ftell(file);
    if (fileSize <= 0) {
        DISPLAY("unexpected size for %s\n", path);
        fclose(file);
        return -1;
    }
    if (fseek(file, 0, SEEK_SET) != 0) {
        DISPLAY("failed to rewind %s\n", path);
        fclose(file);
        return -1;
    }

    data = (unsigned char*)malloc((size_t)fileSize);
    if (data == NULL) {
        DISPLAY("allocation failure\n");
        fclose(file);
        return -1;
    }

    readSize = fread(data, 1, (size_t)fileSize, file);
    fclose(file);
    if (readSize != (size_t)fileSize) {
        DISPLAY("failed to read %s\n", path);
        free(data);
        return -1;
    }

    buffer->data = data;
    buffer->size = (size_t)fileSize;
    return 1;
}

static int loadDictionaryFixture(owned_buffer_t* buffer)
{
    static const char fixturePath[] = "golden-dictionaries/http-dict-missing-symbols";
    char derivedPath[4096];
    int loaded = tryLoadFile(fixturePath, buffer);

    if (loaded == 1) {
        return 0;
    }
    if (loaded < 0) {
        return 1;
    }

    if (buildPathFromFile(derivedPath, sizeof(derivedPath), __FILE__, fixturePath)) {
        loaded = tryLoadFile(derivedPath, buffer);
        if (loaded == 1) {
            return 0;
        }
        if (loaded < 0) {
            return 1;
        }
    }

    DISPLAY("could not locate dictionary fixture %s\n", fixturePath);
    return 1;
}

static void buildDictionaryBiasedSample(unsigned char* dst,
                                        size_t dstSize,
                                        const unsigned char* dict,
                                        size_t dictSize,
                                        unsigned seed)
{
    size_t pos = 0;
    size_t cursor = seed % dictSize;

    while (pos < dstSize) {
        size_t chunk = 48U + ((seed + (unsigned)pos) % 80U);
        if (chunk > dictSize) {
            chunk = dictSize;
        }
        if (chunk > dstSize - pos) {
            chunk = dstSize - pos;
        }
        if (cursor + chunk > dictSize) {
            cursor = (cursor + 131U + (seed % 29U)) % dictSize;
            if (cursor + chunk > dictSize) {
                chunk = dictSize - cursor;
            }
            if (chunk == 0) {
                cursor = 0;
                chunk = MIN(dictSize, dstSize - pos);
            }
        }

        memcpy(dst + pos, dict + cursor, chunk);
        if (chunk > 12U) {
            dst[pos + 3U] ^= (unsigned char)(0x11U + (unsigned)(pos >> 5));
            dst[pos + chunk / 2U] ^= (unsigned char)(0x5AU + (unsigned)(pos >> 4));
        }

        pos += chunk;
        if (pos < dstSize) {
            dst[pos++] = (unsigned char)'\n';
        }
        cursor = (cursor + 97U + (seed % 23U)) % dictSize;
    }
}

static int checkDictionaryRequiredError(size_t code, const char* action)
{
    CHECK(ZSTD_isError(code), "%s unexpectedly succeeded\n", action);
    CHECK(ZSTD_getErrorCode(code) == ZSTD_error_dictionary_wrong ||
          ZSTD_getErrorCode(code) == ZSTD_error_corruption_detected,
          "%s returned %s instead of a dictionary-related error\n",
          action, ZSTD_getErrorName(code));
    return 0;
}

static int checkFrameProperties(const void* compressed,
                                size_t compressedSize,
                                size_t srcSize,
                                unsigned dictID)
{
    CHECK(ZSTD_getFrameContentSize(compressed, compressedSize)
              == (unsigned long long)srcSize,
          "frame content size mismatch\n");
    CHECK(ZSTD_findFrameCompressedSize(compressed, compressedSize) == compressedSize,
          "frame compressed size mismatch\n");
    CHECK(ZSTD_getDictID_fromFrame(compressed, compressedSize) == dictID,
          "frame dictionary ID mismatch\n");
    return 0;
}

static int exerciseDictionaryCase(ZSTD_CCtx* cctx,
                                  ZSTD_DCtx* dctx,
                                  const ZSTD_CDict* cdict,
                                  const ZSTD_DDict* ddict,
                                  const unsigned char* dict,
                                  size_t dictSize,
                                  unsigned dictID,
                                  unsigned seed,
                                  size_t srcSize)
{
    unsigned char* src = (unsigned char*)malloc(srcSize);
    unsigned char* decoded = (unsigned char*)malloc(srcSize);
    unsigned char* compressedByCDict = (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    unsigned char* compressedByDict = (unsigned char*)malloc(ZSTD_compressBound(srcSize));
    size_t const compressedCapacity = ZSTD_compressBound(srcSize);
    size_t compressedSize = 0;
    size_t decodedSize = 0;

    if (src == NULL || decoded == NULL || compressedByCDict == NULL || compressedByDict == NULL) {
        DISPLAY("allocation failure\n");
        free(src);
        free(decoded);
        free(compressedByCDict);
        free(compressedByDict);
        return 1;
    }

    buildDictionaryBiasedSample(src, srcSize, dict, dictSize, seed);

    compressedSize = ZSTD_compress_usingCDict(cctx, compressedByCDict, compressedCapacity,
                                              src, srcSize, cdict);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compress_usingCDict failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    if (checkFrameProperties(compressedByCDict, compressedSize, srcSize, dictID) ||
        checkDictionaryRequiredError(ZSTD_decompress(decoded, srcSize,
                                                     compressedByCDict, compressedSize),
                                     "CDict decode without dictionary")) {
        free(src);
        free(decoded);
        free(compressedByCDict);
        free(compressedByDict);
        return 1;
    }

    decodedSize = ZSTD_decompress_usingDDict(dctx, decoded, srcSize,
                                             compressedByCDict, compressedSize, ddict);
    CHECK(!ZSTD_isError(decodedSize), "ZSTD_decompress_usingDDict failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "CDict/DDict decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "CDict/DDict round-trip mismatch\n");

    decodedSize = ZSTD_decompress_usingDict(dctx, decoded, srcSize,
                                            compressedByCDict, compressedSize,
                                            dict, dictSize);
    CHECK(!ZSTD_isError(decodedSize), "ZSTD_decompress_usingDict failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "CDict/raw-dict decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "CDict/raw-dict round-trip mismatch\n");

    compressedSize = ZSTD_compress_usingDict(cctx, compressedByDict, compressedCapacity,
                                             src, srcSize, dict, dictSize, 5);
    CHECK(!ZSTD_isError(compressedSize), "ZSTD_compress_usingDict failed: %s\n",
          ZSTD_getErrorName(compressedSize));
    if (checkFrameProperties(compressedByDict, compressedSize, srcSize, dictID) ||
        checkDictionaryRequiredError(ZSTD_decompress(decoded, srcSize,
                                                     compressedByDict, compressedSize),
                                     "raw-dict decode without dictionary")) {
        free(src);
        free(decoded);
        free(compressedByCDict);
        free(compressedByDict);
        return 1;
    }

    decodedSize = ZSTD_decompress_usingDDict(dctx, decoded, srcSize,
                                             compressedByDict, compressedSize, ddict);
    CHECK(!ZSTD_isError(decodedSize), "raw-dict/DDict decode failed: %s\n",
          ZSTD_getErrorName(decodedSize));
    CHECK(decodedSize == srcSize, "raw-dict/DDict decoded size mismatch\n");
    CHECK(memcmp(decoded, src, srcSize) == 0, "raw-dict/DDict round-trip mismatch\n");

    free(src);
    free(decoded);
    free(compressedByCDict);
    free(compressedByDict);
    return 0;
}

static int testInvalidDictionaryRejections(void)
{
    const dictionary_t* dict;

    for (dict = invalidDictionaries; dict->data != NULL; ++dict) {
        ZSTD_CDict* cdict = ZSTD_createCDict(dict->data, dict->size, 1);
        ZSTD_DDict* ddict = ZSTD_createDDict(dict->data, dict->size);

        if (cdict != NULL) {
            ZSTD_freeCDict(cdict);
            DISPLAY("invalid dictionary unexpectedly created a CDict\n");
            return 1;
        }
        if (ddict != NULL) {
            ZSTD_freeDDict(ddict);
            DISPLAY("invalid dictionary unexpectedly created a DDict\n");
            return 1;
        }
    }

    return 0;
}

static int testValidDictionaryCoverage(void)
{
    owned_buffer_t dictBuffer;
    ZSTD_CCtx* cctx = NULL;
    ZSTD_DCtx* dctx = NULL;
    ZSTD_CDict* cdict = NULL;
    ZSTD_DDict* ddict = NULL;
    unsigned dictID = 0;

    memset(&dictBuffer, 0, sizeof(dictBuffer));
    if (loadDictionaryFixture(&dictBuffer)) {
        freeOwnedBuffer(&dictBuffer);
        return 1;
    }

    dictID = ZSTD_getDictID_fromDict(dictBuffer.data, dictBuffer.size);
    CHECK(dictID != 0, "fixture dictionary should expose a non-zero dictionary ID\n");

    cdict = ZSTD_createCDict(dictBuffer.data, dictBuffer.size, 5);
    ddict = ZSTD_createDDict(dictBuffer.data, dictBuffer.size);
    cctx = ZSTD_createCCtx();
    dctx = ZSTD_createDCtx();
    if (cdict == NULL || ddict == NULL || cctx == NULL || dctx == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCDict(cdict);
        ZSTD_freeDDict(ddict);
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        freeOwnedBuffer(&dictBuffer);
        return 1;
    }

    CHECK(ZSTD_getDictID_fromCDict(cdict) == dictID, "CDict dictionary ID mismatch\n");
    CHECK(ZSTD_getDictID_fromDDict(ddict) == dictID, "DDict dictionary ID mismatch\n");

    if (exerciseDictionaryCase(cctx, dctx, cdict, ddict,
                               dictBuffer.data, dictBuffer.size, dictID,
                               0x12345U, 24U * 1024U + 131U) ||
        exerciseDictionaryCase(cctx, dctx, cdict, ddict,
                               dictBuffer.data, dictBuffer.size, dictID,
                               0xABCDEFU, 17U * 1024U + 59U)) {
        ZSTD_freeCDict(cdict);
        ZSTD_freeDDict(ddict);
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        freeOwnedBuffer(&dictBuffer);
        return 1;
    }

    ZSTD_freeCDict(cdict);
    ZSTD_freeDDict(ddict);
    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    freeOwnedBuffer(&dictBuffer);
    return 0;
}

int main(int argc, const char** argv)
{
    (void)argc;
    (void)argv;

    if (testInvalidDictionaryRejections() ||
        testValidDictionaryCoverage()) {
        return 1;
    }
    return 0;
}
