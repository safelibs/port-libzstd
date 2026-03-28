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
 * Public decompression robustness fuzzer replacing the internal HUF decoder test.
 */

#include <stddef.h>
#include <stdlib.h>

#include "fuzz_data_producer.h"
#include "zstd.h"

static ZSTD_DCtx* dctx = NULL;

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size);

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    FUZZ_dataProducer_t* producer = FUZZ_dataProducer_create(src, size);
    size_t const maxBufSize = 8 * size + 500;
    size_t const dBufSize = FUZZ_dataProducer_uint32Range(
        producer, 1, maxBufSize == 0 ? (size_t)1 : maxBufSize);
    void* dBuf = malloc(dBufSize);

    if (!dctx) {
        dctx = ZSTD_createDCtx();
    }
    if (dctx != NULL) {
        ZSTD_decompressDCtx(dctx, dBuf, dBufSize, src, size);
    }

    free(dBuf);
    FUZZ_dataProducer_free(producer);
#ifndef STATEFUL_FUZZING
    ZSTD_freeDCtx(dctx);
    dctx = NULL;
#endif
    return 0;
}
