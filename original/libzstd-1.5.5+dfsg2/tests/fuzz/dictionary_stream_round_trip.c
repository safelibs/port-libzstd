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
 * Public streaming round-trip fuzzer with dictionary and prefix coverage.
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
    return buffer;
}

static ZSTD_inBuffer makeInBuffer(const uint8_t** src, size_t* size,
                                  FUZZ_dataProducer_t* producer)
{
    ZSTD_inBuffer buffer = { *src, 0, 0 };
    FUZZ_ASSERT(*size > 0);
    buffer.size = FUZZ_dataProducer_uint32Range(producer, 1, *size);
    *src += buffer.size;
    *size -= buffer.size;
    return buffer;
}

static size_t compress(uint8_t* dst, size_t capacity,
                       const uint8_t* src, size_t srcSize,
                       const uint8_t* dict, size_t dictSize,
                       FUZZ_dataProducer_t* producer, int refPrefix)
{
    size_t dstSize = 0;

    FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    FUZZ_setRandomParameters(cctx, srcSize, producer);
    FUZZ_ZASSERT(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 0));
    if (refPrefix) {
        FUZZ_ZASSERT(ZSTD_CCtx_refPrefix(cctx, dict, dictSize));
    } else {
        FUZZ_ZASSERT(ZSTD_CCtx_loadDictionary(cctx, dict, dictSize));
    }

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
        FUZZ_dict_t const dict = FUZZ_train(src, size, producer);
        int const refPrefix = FUZZ_dataProducer_uint32Range(producer, 0, 1) != 0;
        size_t const cSize = compress(cBuf, neededBufSize, src, size,
                                      (const uint8_t*)dict.buff, dict.size,
                                      producer, refPrefix);
        size_t rSize;

        if (refPrefix) {
            FUZZ_ZASSERT(ZSTD_DCtx_refPrefix(dctx, dict.buff, dict.size));
        } else {
            FUZZ_ZASSERT(ZSTD_DCtx_loadDictionary(dctx, dict.buff, dict.size));
        }
        rSize = ZSTD_decompressDCtx(dctx, rBuf, neededBufSize, cBuf, cSize);
        FUZZ_ZASSERT(rSize);
        FUZZ_ASSERT_MSG(rSize == size, "Incorrect regenerated size");
        FUZZ_ASSERT_MSG(!FUZZ_memcmp(src, rBuf, size), "Corruption!");
        free(dict.buff);
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
