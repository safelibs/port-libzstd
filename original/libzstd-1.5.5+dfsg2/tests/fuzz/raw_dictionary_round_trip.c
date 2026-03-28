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
 * Public prefix-dictionary round-trip fuzzer using raw bytes.
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
                            const void* dict, size_t dictSize,
                            FUZZ_dataProducer_t* producer)
{
    size_t const remainingBytes = FUZZ_dataProducer_remainingBytes(producer);
    size_t cSize;

    FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    FUZZ_setRandomParameters(cctx, srcSize, producer);
    FUZZ_ZASSERT(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 0));
    FUZZ_ZASSERT(ZSTD_CCtx_refPrefix(cctx, dict, dictSize));
    cSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
    FUZZ_ZASSERT(cSize);
    {
        uint64_t const hash0 = FUZZ_hashBuffer(compressed, cSize);
        size_t const cSize0 = cSize;
        FUZZ_dataProducer_rollBack(producer, remainingBytes);
        FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        FUZZ_setRandomParameters(cctx, srcSize, producer);
        FUZZ_ZASSERT(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 0));
        FUZZ_ZASSERT(ZSTD_CCtx_refPrefix(cctx, dict, dictSize));
        cSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
        FUZZ_ASSERT(cSize == cSize0);
        FUZZ_ASSERT(FUZZ_hashBuffer(compressed, cSize) == hash0);
    }
    FUZZ_ZASSERT(ZSTD_DCtx_refPrefix(dctx, dict, dictSize));
    return ZSTD_decompressDCtx(dctx, result, resultCapacity, compressed, cSize);
}

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    uint8_t const* dictBuf;
    size_t dictSize;
    size_t srcSize;
    void* decompBuf;
    void* compBuf;
    size_t compSize;
    FUZZ_dataProducer_t* producer;

    FUZZ_SEQ_PROD_SETUP();

    producer = FUZZ_dataProducer_create(src, size);
    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    srcSize = FUZZ_dataProducer_uint32Range(producer, 0, size);
    dictBuf = src + srcSize;
    dictSize = size - srcSize;
    decompBuf = FUZZ_malloc(srcSize);
    compSize = ZSTD_compressBound(srcSize);
    compSize -= FUZZ_dataProducer_uint32Range(producer, 0, 1);
    compBuf = FUZZ_malloc(compSize);

    if (!cctx) {
        cctx = ZSTD_createCCtx();
        FUZZ_ASSERT(cctx);
    }
    if (!dctx) {
        dctx = ZSTD_createDCtx();
        FUZZ_ASSERT(dctx);
    }

    {
        size_t const result = roundTripTest(decompBuf, srcSize, compBuf, compSize,
                                            src, srcSize, dictBuf, dictSize, producer);
        FUZZ_ZASSERT(result);
        FUZZ_ASSERT_MSG(result == srcSize, "Incorrect regenerated size");
        FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, decompBuf, srcSize), "Corruption!");
    }

    free(decompBuf);
    free(compBuf);
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
