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
 * Public streaming decompression robustness fuzzer.
 */

#include <stddef.h>
#include <stdlib.h>

#include "fuzz_data_producer.h"
#include "fuzz_helpers.h"
#include "zstd.h"

static ZSTD_DStream* dstream = NULL;

static ZSTD_outBuffer makeOutBuffer(FUZZ_dataProducer_t* producer, void* buf, size_t bufSize)
{
    ZSTD_outBuffer buffer = { buf, 0, 0 };

    if (FUZZ_dataProducer_empty(producer)) {
        buffer.size = bufSize;
    } else {
        buffer.size = FUZZ_dataProducer_uint32Range(producer, 0, bufSize);
    }
    if (buffer.size == 0) {
        buffer.dst = NULL;
    }
    return buffer;
}

static ZSTD_inBuffer makeInBuffer(const uint8_t** src, size_t* size,
                                  FUZZ_dataProducer_t* producer)
{
    ZSTD_inBuffer buffer = { *src, 0, 0 };

    FUZZ_ASSERT(*size > 0);
    if (FUZZ_dataProducer_empty(producer)) {
        buffer.size = *size;
    } else {
        buffer.size = FUZZ_dataProducer_uint32Range(producer, 0, *size);
    }
    *src += buffer.size;
    *size -= buffer.size;
    if (buffer.size == 0) {
        buffer.src = NULL;
    }
    return buffer;
}

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    FUZZ_dataProducer_t* producer = FUZZ_dataProducer_create(src, size);
    void* buf;
    size_t bufSize;
    ZSTD_outBuffer out;

    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    bufSize = MAX((size_t)1, MAX(10 * size, ZSTD_BLOCKSIZE_MAX));
    buf = FUZZ_malloc(bufSize);

    if (!dstream) {
        dstream = ZSTD_createDStream();
        FUZZ_ASSERT(dstream);
    } else {
        FUZZ_ZASSERT(ZSTD_DCtx_reset(dstream, ZSTD_reset_session_only));
    }

    out = makeOutBuffer(producer, buf, bufSize);
    while (size > 0) {
        ZSTD_inBuffer in = makeInBuffer(&src, &size, producer);
        do {
            size_t const rc = ZSTD_decompressStream(dstream, &out, &in);
            if (ZSTD_isError(rc)) {
                goto out;
            }
            if (out.pos == out.size) {
                out = makeOutBuffer(producer, buf, bufSize);
            }
        } while (in.pos != in.size);
    }

out:
#ifndef STATEFUL_FUZZING
    ZSTD_freeDStream(dstream);
    dstream = NULL;
#endif
    FUZZ_dataProducer_free(producer);
    free(buf);
    return 0;
}
