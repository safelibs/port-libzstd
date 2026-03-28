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
 * Public generated-source round-trip fuzzer replacing the sequence-compression target.
 */

#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz_data_producer.h"
#include "fuzz_helpers.h"
#include "zstd_helpers.h"

static ZSTD_CCtx* cctx = NULL;
static ZSTD_DCtx* dctx = NULL;
static uint8_t* generatedSrc = NULL;
static uint8_t* compressed = NULL;
static uint8_t* regenerated = NULL;
static size_t capacity = 0;

static void fillGeneratedSource(uint8_t* dst, size_t size, FUZZ_dataProducer_t* producer)
{
    size_t i;
    for (i = 0; i < size; ++i) {
        dst[i] = (uint8_t)FUZZ_dataProducer_uint32Range(producer, 0, 255);
    }
}

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    FUZZ_dataProducer_t* producer = FUZZ_dataProducer_create(src, size);
    size_t generatedSize;
    FUZZ_dict_t dict = { NULL, 0 };
    int usePrefix;
    int useDict;
    size_t cSize;
    size_t rSize;

    size = FUZZ_dataProducer_reserveDataPrefix(producer);
    generatedSize = FUZZ_dataProducer_uint32Range(producer, 0, MIN((size_t)(1 << 20), MAX((size_t)1, size * 8 + 1)));
    if (generatedSize > capacity) {
        free(generatedSrc);
        free(compressed);
        free(regenerated);
        generatedSrc = (uint8_t*)FUZZ_malloc(generatedSize == 0 ? 1 : generatedSize);
        compressed = (uint8_t*)FUZZ_malloc(ZSTD_compressBound(generatedSize));
        regenerated = (uint8_t*)FUZZ_malloc(generatedSize == 0 ? 1 : generatedSize);
        capacity = generatedSize;
    }
    fillGeneratedSource(generatedSrc, generatedSize, producer);
    useDict = FUZZ_dataProducer_uint32Range(producer, 0, 1);
    usePrefix = FUZZ_dataProducer_uint32Range(producer, 0, 1);
    if (useDict) {
        dict = FUZZ_train(generatedSrc, generatedSize, producer);
    }

    if (!cctx) {
        cctx = ZSTD_createCCtx();
        FUZZ_ASSERT(cctx);
    }
    if (!dctx) {
        dctx = ZSTD_createDCtx();
        FUZZ_ASSERT(dctx);
    }

    FUZZ_ZASSERT(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    FUZZ_setRandomParameters(cctx, generatedSize, producer);
    if (useDict) {
        if (usePrefix) {
            FUZZ_ZASSERT(ZSTD_CCtx_refPrefix(cctx, dict.buff, dict.size));
        } else {
            FUZZ_ZASSERT(ZSTD_CCtx_loadDictionary(cctx, dict.buff, dict.size));
        }
    }
    cSize = ZSTD_compress2(cctx, compressed, ZSTD_compressBound(generatedSize),
                           generatedSrc, generatedSize);
    FUZZ_ZASSERT(cSize);

    if (useDict) {
        if (usePrefix) {
            FUZZ_ZASSERT(ZSTD_DCtx_refPrefix(dctx, dict.buff, dict.size));
        } else {
            FUZZ_ZASSERT(ZSTD_DCtx_loadDictionary(dctx, dict.buff, dict.size));
        }
    }
    rSize = ZSTD_decompressDCtx(dctx, regenerated, generatedSize == 0 ? 1 : generatedSize,
                                compressed, cSize);
    FUZZ_ZASSERT(rSize);
    FUZZ_ASSERT(rSize == generatedSize);
    FUZZ_ASSERT(!FUZZ_memcmp(generatedSrc, regenerated, generatedSize));

    free(dict.buff);
    FUZZ_dataProducer_free(producer);
#ifndef STATEFUL_FUZZING
    ZSTD_freeCCtx(cctx);
    cctx = NULL;
    ZSTD_freeDCtx(dctx);
    dctx = NULL;
#endif
    return 0;
}
