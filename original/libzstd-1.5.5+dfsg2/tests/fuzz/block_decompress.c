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
 * Public one-shot decompression robustness fuzzer sized like the old block test.
 */

#include <stddef.h>
#include <stdlib.h>

#include "fuzz_data_producer.h"
#include "fuzz_helpers.h"
#include "zstd.h"

static ZSTD_DCtx* dctx = NULL;
static void* rBuf = NULL;
static size_t bufSize = 0;

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    size_t const neededBufSize = MAX(ZSTD_BLOCKSIZE_MAX, size * 4 + 1);
    FUZZ_dataProducer_t* producer = FUZZ_dataProducer_create(src, size);

    if (neededBufSize > bufSize) {
        free(rBuf);
        rBuf = FUZZ_malloc_rand(neededBufSize, producer);
        bufSize = neededBufSize;
    }
    if (!dctx) {
        dctx = ZSTD_createDCtx();
        FUZZ_ASSERT(dctx);
    }

    ZSTD_decompressDCtx(dctx, rBuf, neededBufSize, src, size);

    FUZZ_dataProducer_free(producer);
#ifndef STATEFUL_FUZZING
    ZSTD_freeDCtx(dctx);
    dctx = NULL;
#endif
    return 0;
}
