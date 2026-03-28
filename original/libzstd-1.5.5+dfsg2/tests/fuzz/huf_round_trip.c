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
 * Public literal-heavy round-trip fuzzer replacing the internal HUF coverage.
 */

#include <stddef.h>
#include <stdlib.h>

#include "fuzz_data_producer.h"
#include "fuzz_helpers.h"
#include "zstd_helpers.h"

static ZSTD_CCtx* cctx = NULL;
static ZSTD_DCtx* dctx = NULL;

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    uint8_t* literalBuf;
    void* cBuf;
    void* rBuf;
    size_t cBufSize;
    size_t i;
    FUZZ_dataProducer_t* producer = FUZZ_dataProducer_create(src, size);

    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    literalBuf = (uint8_t*)FUZZ_malloc(size == 0 ? 1 : size);
    for (i = 0; i < size; ++i) {
        literalBuf[i] = (uint8_t)('A' + (src[i] & 0x0F));
    }
    cBufSize = ZSTD_compressBound(size);
    cBuf = FUZZ_malloc(cBufSize);
    rBuf = FUZZ_malloc(size == 0 ? 1 : size);

    if (!cctx) {
        cctx = ZSTD_createCCtx();
        FUZZ_ASSERT(cctx);
    }
    if (!dctx) {
        dctx = ZSTD_createDCtx();
        FUZZ_ASSERT(dctx);
    }

    {
        int const level = FUZZ_dataProducer_int32Range(producer, 1, 5);
        size_t const cSize = ZSTD_compressCCtx(cctx, cBuf, cBufSize, literalBuf, size, level);
        size_t const dSize = ZSTD_decompressDCtx(dctx, rBuf, size == 0 ? 1 : size, cBuf, cSize);
        FUZZ_ZASSERT(cSize);
        FUZZ_ZASSERT(dSize);
        FUZZ_ASSERT(dSize == size);
        FUZZ_ASSERT(!FUZZ_memcmp(literalBuf, rBuf, size));
    }

    free(rBuf);
    free(cBuf);
    free(literalBuf);
    FUZZ_dataProducer_free(producer);
#ifndef STATEFUL_FUZZING
    ZSTD_freeCCtx(cctx);
    cctx = NULL;
    ZSTD_freeDCtx(dctx);
    dctx = NULL;
#endif
    return 0;
}
