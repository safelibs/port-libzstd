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
 * Public streaming round-trip fuzzer.
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
static uint8_t* cBuf = NULL;
static uint8_t* rBuf = NULL;
static size_t bufSize = 0;

static ZSTD_outBuffer makeOutBuffer(uint8_t* dst, size_t capacity,
                                    FUZZ_dataProducer_t* producer)
{
    ZSTD_outBuffer buffer = { dst, 0, 0 };

    FUZZ_ASSERT(capacity > 0);
    buffer.size = FUZZ_dataProducer_uint32Range(producer, 1, capacity);
    FUZZ_ASSERT(buffer.size <= capacity);
    return buffer;
}

static ZSTD_inBuffer makeInBuffer(const uint8_t** src, size_t* size,
                                  FUZZ_dataProducer_t* producer)
{
    ZSTD_inBuffer buffer = { *src, 0, 0 };

    FUZZ_ASSERT(*size > 0);
    buffer.size = FUZZ_dataProducer_uint32Range(producer, 1, *size);
    FUZZ_ASSERT(buffer.size <= *size);
    *src += buffer.size;
    *size -= buffer.size;
    return buffer;
}

static size_t compress(uint8_t* dst, size_t capacity,
                       const uint8_t* src, size_t srcSize,
                       FUZZ_dataProducer_t* producer)
{
    size_t dstSize = 0;

    FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    FUZZ_setRandomParameters(cctx, srcSize, producer);
    while (srcSize > 0) {
        ZSTD_inBuffer in = makeInBuffer(&src, &srcSize, producer);
        while (in.pos < in.size) {
            ZSTD_EndDirective const op =
                FUZZ_dataProducer_uint32Range(producer, 0, 3) == 0
                    ? ZSTD_e_flush
                    : ZSTD_e_continue;
            ZSTD_outBuffer out = makeOutBuffer(dst, capacity, producer);
            size_t const ret = ZSTD_compressStream2(cctx, &out, &in, op);
            FUZZ_ZASSERT(ret);
            dst += out.pos;
            dstSize += out.pos;
            capacity -= out.pos;
        }
    }

    for (;;) {
        ZSTD_inBuffer in = { NULL, 0, 0 };
        ZSTD_outBuffer out = makeOutBuffer(dst, capacity, producer);
        size_t const ret = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_end);
        FUZZ_ZASSERT(ret);
        dst += out.pos;
        dstSize += out.pos;
        capacity -= out.pos;
        if (ret == 0) {
            break;
        }
    }

    return dstSize;
}

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    size_t neededBufSize;
    FUZZ_dataProducer_t* producer;

    FUZZ_SEQ_PROD_SETUP();

    producer = FUZZ_dataProducer_create(src, size);
    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    neededBufSize = ZSTD_compressBound(size) * 8 + ZSTD_BLOCKSIZE_MAX;

    if (neededBufSize > bufSize) {
        free(cBuf);
        free(rBuf);
        cBuf = (uint8_t*)FUZZ_malloc(neededBufSize);
        rBuf = (uint8_t*)FUZZ_malloc(neededBufSize);
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
        size_t const cSize = compress(cBuf, neededBufSize, src, size, producer);
        size_t const rSize = ZSTD_decompressDCtx(dctx, rBuf, neededBufSize, cBuf, cSize);
        FUZZ_ZASSERT(rSize);
        FUZZ_ASSERT_MSG(rSize == size, "Incorrect regenerated size");
        FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, rBuf, size), "Corruption!");

        {
            size_t const overlapSize = size + cSize + ZSTD_BLOCKSIZE_MAX + 64;
            char* const output = (char*)FUZZ_malloc(overlapSize);
            char* const input = output + overlapSize - cSize;
            size_t dSize;
            memcpy(input, cBuf, cSize);
            dSize = ZSTD_decompressDCtx(dctx, output, overlapSize, input, cSize);
            FUZZ_ZASSERT(dSize);
            FUZZ_ASSERT_MSG(dSize == size, "Incorrect regenerated size");
            FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, output, size), "Corruption!");
            free(output);
        }
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
