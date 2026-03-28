/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

/**
 * Public-API round-trip fuzzer with determinism and overlapping-buffer checks.
 */

#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz_data_producer.h"
#include "fuzz_helpers.h"
#include "fuzz_third_party_seq_prod.h"
#include "zstd_helpers.h"

static ZSTD_CCtx* cctx = NULL;
static ZSTD_DCtx* dctx = NULL;

static size_t roundTripTest(void* result, size_t resultCapacity,
                            void* compressed, size_t compressedCapacity,
                            const void* src, size_t srcSize,
                            FUZZ_dataProducer_t* producer)
{
    size_t cSize;
    size_t dSize;

    if (FUZZ_dataProducer_uint32Range(producer, 0, 1) != 0) {
        size_t const remainingBytes = FUZZ_dataProducer_remainingBytes(producer);
        FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        FUZZ_setRandomParameters(cctx, srcSize, producer);
        cSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
        FUZZ_ZASSERT(cSize);
        {
            uint64_t const hash0 = FUZZ_hashBuffer(compressed, cSize);
            size_t const cSize0 = cSize;
            FUZZ_dataProducer_rollBack(producer, remainingBytes);
            FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
            FUZZ_setRandomParameters(cctx, srcSize, producer);
            cSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
            FUZZ_ASSERT(cSize == cSize0);
            FUZZ_ASSERT(FUZZ_hashBuffer(compressed, cSize) == hash0);
        }
    } else {
        int const cLevel = FUZZ_dataProducer_int32Range(producer, kMinClevel, kMaxClevel);
        cSize = ZSTD_compressCCtx(cctx, compressed, compressedCapacity, src, srcSize, cLevel);
        FUZZ_ZASSERT(cSize);
        {
            uint64_t const hash0 = FUZZ_hashBuffer(compressed, cSize);
            size_t const cSize0 = cSize;
            cSize = ZSTD_compressCCtx(cctx, compressed, compressedCapacity, src, srcSize, cLevel);
            FUZZ_ASSERT(cSize == cSize0);
            FUZZ_ASSERT(FUZZ_hashBuffer(compressed, cSize) == hash0);
        }
    }

    dSize = ZSTD_decompressDCtx(dctx, result, resultCapacity, compressed, cSize);
    FUZZ_ZASSERT(dSize);
    FUZZ_ASSERT_MSG(dSize == srcSize, "Incorrect regenerated size");
    FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, result, dSize), "Corruption!");

    {
        size_t const outputSize = srcSize + cSize + ZSTD_BLOCKSIZE_MAX + 64;
        char* const output = (char*)FUZZ_malloc(outputSize);
        char* const input = output + outputSize - cSize;
        dSize = 0;
        memcpy(input, compressed, cSize);
        dSize = ZSTD_decompressDCtx(dctx, output, outputSize, input, cSize);
        FUZZ_ZASSERT(dSize);
        FUZZ_ASSERT_MSG(dSize == srcSize, "Incorrect regenerated size");
        FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, output, srcSize), "Corruption!");
        free(output);
    }

    return dSize;
}

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    void* rBuf;
    void* cBuf;
    size_t cBufSize;
    FUZZ_dataProducer_t* producer;

    FUZZ_SEQ_PROD_SETUP();

    producer = FUZZ_dataProducer_create(src, size);
    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    rBuf = FUZZ_malloc(size);
    cBufSize = ZSTD_compressBound(size);
    cBuf = FUZZ_malloc(cBufSize);

    if (!cctx) {
        cctx = ZSTD_createCCtx();
        FUZZ_ASSERT(cctx);
    }
    if (!dctx) {
        dctx = ZSTD_createDCtx();
        FUZZ_ASSERT(dctx);
    }

    roundTripTest(rBuf, size, cBuf, cBufSize, src, size, producer);

    free(rBuf);
    free(cBuf);
    FUZZ_dataProducer_free(producer);
#ifndef STATEFUL_FUZZING
    ZSTD_freeCCtx(cctx);
    cctx = NULL;
    ZSTD_freeDCtx(dctx);
    dctx = NULL;
#endif
    FUZZ_SEQ_PROD_TEARDOWN();
    return 0;
}
