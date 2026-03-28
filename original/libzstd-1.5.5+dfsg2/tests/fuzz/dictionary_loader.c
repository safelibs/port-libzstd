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
 * Public dictionary-loading round-trip fuzzer.
 */

#include <stddef.h>
#include <stdlib.h>

#include "fuzz_data_producer.h"
#include "fuzz_helpers.h"
#include "fuzz_third_party_seq_prod.h"
#include "zstd_helpers.h"

static size_t compress(void* compressed, size_t compressedCapacity,
                       void const* source, size_t sourceSize,
                       void const* dict, size_t dictSize,
                       int refPrefix)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    size_t compressedSize;
    FUZZ_ASSERT(cctx != NULL);
    if (refPrefix) {
        FUZZ_ZASSERT(ZSTD_CCtx_refPrefix(cctx, dict, dictSize));
    } else {
        FUZZ_ZASSERT(ZSTD_CCtx_loadDictionary(cctx, dict, dictSize));
    }
    compressedSize = ZSTD_compress2(cctx, compressed, compressedCapacity, source, sourceSize);
    ZSTD_freeCCtx(cctx);
    return compressedSize;
}

static size_t decompress(void* result, size_t resultCapacity,
                         void const* compressed, size_t compressedSize,
                         void const* dict, size_t dictSize,
                         int refPrefix)
{
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    size_t resultSize;
    FUZZ_ASSERT(dctx != NULL);
    if (refPrefix) {
        FUZZ_ZASSERT(ZSTD_DCtx_refPrefix(dctx, dict, dictSize));
    } else {
        FUZZ_ZASSERT(ZSTD_DCtx_loadDictionary(dctx, dict, dictSize));
    }
    resultSize = ZSTD_decompressDCtx(dctx, result, resultCapacity, compressed, compressedSize);
    ZSTD_freeDCtx(dctx);
    return resultSize;
}

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    FUZZ_dataProducer_t* producer;
    FUZZ_dict_t dict;
    int refPrefix;
    void* rBuf;
    void* cBuf;
    size_t cBufSize;
    size_t cSize;
    size_t rSize;

    FUZZ_SEQ_PROD_SETUP();
    producer = FUZZ_dataProducer_create(src, size);
    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    dict = FUZZ_train(src, size, producer);
    refPrefix = FUZZ_dataProducer_uint32Range(producer, 0, 1) != 0;
    rBuf = FUZZ_malloc(size);
    cBufSize = ZSTD_compressBound(size);
    cBuf = FUZZ_malloc(cBufSize);

    cSize = compress(cBuf, cBufSize, src, size, dict.buff, dict.size, refPrefix);
    FUZZ_ZASSERT(cSize);
    rSize = decompress(rBuf, size, cBuf, cSize, dict.buff, dict.size, refPrefix);
    FUZZ_ZASSERT(rSize);
    FUZZ_ASSERT_MSG(rSize == size, "Incorrect regenerated size");
    FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, rBuf, size), "Corruption!");

    free(dict.buff);
    free(cBuf);
    free(rBuf);
    FUZZ_dataProducer_free(producer);
    FUZZ_SEQ_PROD_TEARDOWN();
    return 0;
}
