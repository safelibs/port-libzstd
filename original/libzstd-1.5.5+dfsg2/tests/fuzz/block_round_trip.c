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
 * Public block-sized one-shot round-trip fuzzer.
 */

#include <stddef.h>
#include <stdlib.h>

#include "fuzz_data_producer.h"
#include "fuzz_helpers.h"
#include "fuzz_third_party_seq_prod.h"
#include "zstd_helpers.h"

static ZSTD_CCtx* cctx = NULL;
static ZSTD_DCtx* dctx = NULL;
static void* cBuf = NULL;
static void* rBuf = NULL;
static size_t bufSize = 0;

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    size_t neededBufSize;
    FUZZ_dataProducer_t* producer;

    FUZZ_SEQ_PROD_SETUP();

    producer = FUZZ_dataProducer_create(src, size);
    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    if (size > ZSTD_BLOCKSIZE_MAX) {
        size = ZSTD_BLOCKSIZE_MAX;
    }
    neededBufSize = ZSTD_compressBound(size);

    if (neededBufSize > bufSize) {
        free(cBuf);
        free(rBuf);
        cBuf = FUZZ_malloc(neededBufSize);
        rBuf = FUZZ_malloc(MAX(size, (size_t)1));
        bufSize = neededBufSize;
    }
    if (!cctx) {
        cctx = ZSTD_createCCtx();
        FUZZ_ASSERT(cctx);
    }
    if (!dctx) {
        dctx = ZSTD_createDCtx();
        FUZZ_ASSERT(dctx);
    }

    {
        int const cLevel = FUZZ_dataProducer_int32Range(producer, kMinClevel, kMaxClevel);
        size_t const cSize = ZSTD_compressCCtx(cctx, cBuf, neededBufSize, src, size, cLevel);
        size_t const dSize = ZSTD_decompressDCtx(dctx, rBuf, MAX(size, (size_t)1), cBuf, cSize);
        FUZZ_ZASSERT(cSize);
        FUZZ_ZASSERT(dSize);
        FUZZ_ASSERT_MSG(dSize == size, "Incorrect regenerated size");
        FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, rBuf, size), "Corruption!");
    }

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
