/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

/*
  This program takes a file in input,
  performs a zstd round-trip test (compression - decompress)
  compares the result with original
  and generates a crash (double free) on corruption detection.
*/

/*===========================================
*   Dependencies
*==========================================*/
#include <stddef.h>     /* size_t */
#include <stdlib.h>     /* malloc, free, exit */
#include <stdio.h>      /* fprintf */
#include <string.h>     /* strcmp */
#include <sys/types.h>  /* stat */
#include <sys/stat.h>   /* stat */

#include "zstd.h"

/*===========================================
*   Macros
*==========================================*/
#define MIN(a,b)  ( (a) < (b) ? (a) : (b) )

static unsigned levelSeed(const void* srcBuff, size_t srcBuffSize)
{
    unsigned const char* const bytes = (const unsigned char*)srcBuff;
    size_t const sampleSize = MIN(128, srcBuffSize);
    unsigned seed = 2166136261U;
    size_t pos;

    for (pos = 0; pos < sampleSize; ++pos) {
        seed ^= bytes[pos];
        seed *= 16777619U;
    }

    return seed;
}

static void crash(int errorCode){
    /* abort if AFL/libfuzzer, exit otherwise */
    #ifdef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION /* could also use __AFL_COMPILER */
        abort();
    #else
        exit(errorCode);
    #endif
}

#define CHECK_Z(f) {                            \
    size_t const err = f;                       \
    if (ZSTD_isError(err)) {                    \
        fprintf(stderr,                         \
                "Error=> %s: %s",               \
                #f, ZSTD_getErrorName(err));    \
        crash(1);                                \
}   }

/** roundTripTest() :
*   Compresses `srcBuff` into `compressedBuff`,
*   then decompresses `compressedBuff` into `resultBuff`.
*   Compression level used is derived from first content byte.
*   @return : result of decompression, which should be == `srcSize`
*          or an error code if either compression or decompression fails.
*   Note : `compressedBuffCapacity` should be `>= ZSTD_compressBound(srcSize)`
*          for compression to be guaranteed to work */
static size_t roundTripTest(void* resultBuff, size_t resultBuffCapacity,
                            void* compressedBuff, size_t compressedBuffCapacity,
                      const void* srcBuff, size_t srcBuffSize)
{
    static const int maxClevel = 19;
    unsigned const seed = levelSeed(srcBuff, srcBuffSize);
    int const cLevel = (int)(seed % maxClevel);
    size_t const cSize = ZSTD_compress(compressedBuff, compressedBuffCapacity, srcBuff, srcBuffSize, cLevel);
    if (ZSTD_isError(cSize)) {
        fprintf(stderr, "Compression error : %s \n", ZSTD_getErrorName(cSize));
        return cSize;
    }
    return ZSTD_decompress(resultBuff, resultBuffCapacity, compressedBuff, cSize);
}

/** advancedParamRoundTripTest() :
*  Same as roundTripTest() except uses the public advanced parameter setters. */
static size_t advancedParamRoundTripTest(void* resultBuff, size_t resultBuffCapacity,
                                   void* compressedBuff, size_t compressedBuffCapacity,
                             const void* srcBuff, size_t srcBuffSize)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    static const int maxClevel = 19;
    unsigned const seed = levelSeed(srcBuff, srcBuffSize);
    int const cLevel = (int)(seed % maxClevel);
    size_t cSize;

    if (cctx == NULL) {
        crash(1);
    }

    /* Set parameters */
    CHECK_Z( ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, cLevel) );
    CHECK_Z( ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, 2) );
    CHECK_Z( ZSTD_CCtx_setParameter(cctx, ZSTD_c_overlapLog, 5) );

    cSize = ZSTD_compress2(cctx, compressedBuff, compressedBuffCapacity, srcBuff, srcBuffSize);
    if (ZSTD_isError(cSize)) {
        ZSTD_freeCCtx(cctx);
        return cSize;
    }
    ZSTD_freeCCtx(cctx);

    return ZSTD_decompress(resultBuff, resultBuffCapacity, compressedBuff, cSize);
}

static size_t checkBuffers(const void* buff1, const void* buff2, size_t buffSize)
{
    const char* ip1 = (const char*)buff1;
    const char* ip2 = (const char*)buff2;
    size_t pos;

    for (pos=0; pos<buffSize; pos++)
        if (ip1[pos]!=ip2[pos])
            break;

    return pos;
}

static void roundTripCheck(const void* srcBuff, size_t srcBuffSize, int testCCtxParams)
{
    size_t const cBuffSize = ZSTD_compressBound(srcBuffSize);
    void* cBuff = malloc(cBuffSize);
    void* rBuff = malloc(cBuffSize);

    if (!cBuff || !rBuff) {
        fprintf(stderr, "not enough memory ! \n");
        exit (1);
    }

    {   size_t const result = testCCtxParams ?
                  advancedParamRoundTripTest(rBuff, cBuffSize, cBuff, cBuffSize, srcBuff, srcBuffSize)
                : roundTripTest(rBuff, cBuffSize, cBuff, cBuffSize, srcBuff, srcBuffSize);
        if (ZSTD_isError(result)) {
            fprintf(stderr, "roundTripTest error : %s \n", ZSTD_getErrorName(result));
            crash(1);
        }
        if (result != srcBuffSize) {
            fprintf(stderr, "Incorrect regenerated size : %u != %u\n", (unsigned)result, (unsigned)srcBuffSize);
            crash(1);
        }
        if (checkBuffers(srcBuff, rBuff, srcBuffSize) != srcBuffSize) {
            fprintf(stderr, "Silent decoding corruption !!!");
            crash(1);
        }
    }

    free(cBuff);
    free(rBuff);
}


static size_t getFileSize(const char* infilename)
{
    int r;
#if defined(_MSC_VER)
    struct _stat64 statbuf;
    r = _stat64(infilename, &statbuf);
    if (r || !(statbuf.st_mode & S_IFREG)) return 0;   /* No good... */
#else
    struct stat statbuf;
    r = stat(infilename, &statbuf);
    if (r || !S_ISREG(statbuf.st_mode)) return 0;   /* No good... */
#endif
    return (size_t)statbuf.st_size;
}


static int isDirectory(const char* infilename)
{
    int r;
#if defined(_MSC_VER)
    struct _stat64 statbuf;
    r = _stat64(infilename, &statbuf);
    if (!r && (statbuf.st_mode & _S_IFDIR)) return 1;
#else
    struct stat statbuf;
    r = stat(infilename, &statbuf);
    if (!r && S_ISDIR(statbuf.st_mode)) return 1;
#endif
    return 0;
}


/** loadFile() :
*   requirement : `buffer` size >= `fileSize` */
static void loadFile(void* buffer, const char* fileName, size_t fileSize)
{
    FILE* const f = fopen(fileName, "rb");
    if (isDirectory(fileName)) {
        fprintf(stderr, "Ignoring %s directory \n", fileName);
        exit(2);
    }
    if (f==NULL) {
        fprintf(stderr, "Impossible to open %s \n", fileName);
        exit(3);
    }
    {   size_t const readSize = fread(buffer, 1, fileSize, f);
        if (readSize != fileSize) {
            fprintf(stderr, "Error reading %s \n", fileName);
            exit(5);
    }   }
    fclose(f);
}


static void fileCheck(const char* fileName, int testCCtxParams)
{
    size_t const fileSize = getFileSize(fileName);
    void* const buffer = malloc(fileSize + !fileSize /* avoid 0 */);
    if (!buffer) {
        fprintf(stderr, "not enough memory \n");
        exit(4);
    }
    loadFile(buffer, fileName, fileSize);
    roundTripCheck(buffer, fileSize, testCCtxParams);
    free (buffer);
}

int main(int argCount, const char** argv) {
    int argNb = 1;
    int testCCtxParams = 0;
    if (argCount < 2) {
        fprintf(stderr, "Error : no argument : need input file \n");
        exit(9);
    }

    if (!strcmp(argv[argNb], "--cctxParams")) {
      testCCtxParams = 1;
      argNb++;
    }

    fileCheck(argv[argNb], testCCtxParams);
    fprintf(stderr, "no pb detected\n");
    return 0;
}
