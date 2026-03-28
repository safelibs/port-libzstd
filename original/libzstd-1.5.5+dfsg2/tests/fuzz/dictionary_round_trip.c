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
 * Public dictionary round-trip fuzzer.
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
    FUZZ_dict_t const dict = FUZZ_train(src, srcSize, producer);
    int const useLoadedDict = FUZZ_dataProducer_uint32Range(producer, 0, 1);
    size_t cSize;

    if (useLoadedDict) {
        size_t const remainingBytes = FUZZ_dataProducer_remainingBytes(producer);
        FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        FUZZ_setRandomParameters(cctx, srcSize, producer);
        FUZZ_ZASSERT(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 0));
        FUZZ_ZASSERT(ZSTD_CCtx_loadDictionary(cctx, dict.buff, dict.size));
        cSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
        FUZZ_ZASSERT(cSize);
        {
            uint64_t const hash0 = FUZZ_hashBuffer(compressed, cSize);
            size_t const cSize0 = cSize;
            FUZZ_dataProducer_rollBack(producer, remainingBytes);
            FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
            FUZZ_setRandomParameters(cctx, srcSize, producer);
            FUZZ_ZASSERT(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 0));
            FUZZ_ZASSERT(ZSTD_CCtx_loadDictionary(cctx, dict.buff, dict.size));
            cSize = ZSTD_compress2(cctx, compressed, compressedCapacity, src, srcSize);
            FUZZ_ASSERT(cSize == cSize0);
            FUZZ_ASSERT(FUZZ_hashBuffer(compressed, cSize) == hash0);
        }
        FUZZ_ZASSERT(ZSTD_DCtx_loadDictionary(dctx, dict.buff, dict.size));
        {
            size_t const ret = ZSTD_decompressDCtx(dctx, result, resultCapacity, compressed, cSize);
            free(dict.buff);
            return ret;
        }
    }

    {
        int const cLevel = FUZZ_dataProducer_int32Range(producer, kMinClevel, kMaxClevel);
        cSize = ZSTD_compress_usingDict(cctx, compressed, compressedCapacity,
                                        src, srcSize, dict.buff, dict.size, cLevel);
        FUZZ_ZASSERT(cSize);
        {
            uint64_t const hash0 = FUZZ_hashBuffer(compressed, cSize);
            size_t const cSize0 = cSize;
            cSize = ZSTD_compress_usingDict(cctx, compressed, compressedCapacity,
                                            src, srcSize, dict.buff, dict.size, cLevel);
            FUZZ_ASSERT(cSize == cSize0);
            FUZZ_ASSERT(FUZZ_hashBuffer(compressed, cSize) == hash0);
        }
        {
            size_t const ret = ZSTD_decompress_usingDict(dctx, result, resultCapacity,
                                                         compressed, cSize,
                                                         dict.buff, dict.size);
            free(dict.buff);
            return ret;
        }
    }
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
    cBufSize -= FUZZ_dataProducer_uint32Range(producer, 0, 1);
    cBuf = FUZZ_malloc(cBufSize);

    if (!cctx) {
        cctx = ZSTD_createCCtx();
        FUZZ_ASSERT(cctx);
    }
    if (!dctx) {
        dctx = ZSTD_createDCtx();
        FUZZ_ASSERT(dctx);
    }

    {
        size_t const result = roundTripTest(rBuf, size, cBuf, cBufSize, src, size, producer);
        FUZZ_ZASSERT(result);
        FUZZ_ASSERT_MSG(result == size, "Incorrect regenerated size");
        FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, rBuf, size), "Corruption!");
    }

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
