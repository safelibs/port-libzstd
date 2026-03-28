/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#include <stdlib.h>
#include <string.h>

#include "zdict.h"
#include "zstd_errors.h"

#include "fuzz_helpers.h"
#include "zstd_helpers.h"

const int kMinClevel = -3;
const int kMaxClevel = 19;

static int clampUpper(ZSTD_cParameter param, int upperBound, size_t srcSize)
{
    int upper = upperBound;

    if (param == ZSTD_c_windowLog || param == ZSTD_c_hashLog || param == ZSTD_c_chainLog) {
        upper = MIN(upper, 23);
    } else if (param == ZSTD_c_searchLog) {
        upper = MIN(upper, 7);
    } else if (param == ZSTD_c_minMatch) {
        upper = MIN(upper, 7);
    } else if (param == ZSTD_c_targetLength) {
        upper = MIN(upper, 128);
    } else if (param == ZSTD_c_nbWorkers) {
        upper = MIN(upper, 2);
    } else if (param == ZSTD_c_jobSize) {
        upper = MIN(upper, (int)MAX(srcSize, (size_t)1));
    }

    return upper;
}

static int randomBoundedValue(ZSTD_cParameter param, size_t srcSize,
                              FUZZ_dataProducer_t* producer)
{
    ZSTD_bounds const bounds = ZSTD_cParam_getBounds(param);
    int lower;
    int upper;

    FUZZ_ASSERT(!ZSTD_isError(bounds.error));
    lower = bounds.lowerBound;
    upper = clampUpper(param, bounds.upperBound, srcSize);
    if (upper < lower) {
        upper = lower;
    }
    return FUZZ_dataProducer_int32Range(producer, lower, upper);
}

static void setParameterIfSupported(ZSTD_CCtx* cctx, ZSTD_cParameter param, int value)
{
    size_t const code = ZSTD_CCtx_setParameter(cctx, param, value);

    if (ZSTD_isError(code) &&
        ZSTD_getErrorCode(code) != ZSTD_error_parameter_unsupported) {
        FUZZ_ZASSERT(code);
    }
}

void FUZZ_setRandomParameters(ZSTD_CCtx* cctx, size_t srcSize, FUZZ_dataProducer_t* producer)
{
    setParameterIfSupported(cctx, ZSTD_c_compressionLevel,
        FUZZ_dataProducer_int32Range(producer, kMinClevel, kMaxClevel));
    setParameterIfSupported(cctx, ZSTD_c_windowLog,
        randomBoundedValue(ZSTD_c_windowLog, srcSize, producer));
    setParameterIfSupported(cctx, ZSTD_c_hashLog,
        randomBoundedValue(ZSTD_c_hashLog, srcSize, producer));
    setParameterIfSupported(cctx, ZSTD_c_chainLog,
        randomBoundedValue(ZSTD_c_chainLog, srcSize, producer));
    setParameterIfSupported(cctx, ZSTD_c_searchLog,
        randomBoundedValue(ZSTD_c_searchLog, srcSize, producer));
    setParameterIfSupported(cctx, ZSTD_c_minMatch,
        randomBoundedValue(ZSTD_c_minMatch, srcSize, producer));
    setParameterIfSupported(cctx, ZSTD_c_targetLength,
        randomBoundedValue(ZSTD_c_targetLength, srcSize, producer));
    setParameterIfSupported(cctx, ZSTD_c_strategy,
        randomBoundedValue(ZSTD_c_strategy, srcSize, producer));
    setParameterIfSupported(cctx, ZSTD_c_contentSizeFlag,
        FUZZ_dataProducer_int32Range(producer, 0, 1));
    setParameterIfSupported(cctx, ZSTD_c_checksumFlag,
        FUZZ_dataProducer_int32Range(producer, 0, 1));
    setParameterIfSupported(cctx, ZSTD_c_dictIDFlag,
        FUZZ_dataProducer_int32Range(producer, 0, 1));
    setParameterIfSupported(cctx, ZSTD_c_enableLongDistanceMatching,
        FUZZ_dataProducer_int32Range(producer, 0, 1));
    setParameterIfSupported(cctx, ZSTD_c_nbWorkers,
        randomBoundedValue(ZSTD_c_nbWorkers, srcSize, producer));
}

FUZZ_dict_t FUZZ_train(void const* src, size_t srcSize, FUZZ_dataProducer_t* producer)
{
    FUZZ_dict_t dict = { NULL, 0 };
    unsigned nbSamples;
    size_t* samplesSizes;
    size_t dictCapacity;
    size_t pos = 0;
    unsigned i;
    (void)producer;

    if (src == NULL || srcSize == 0) {
        return dict;
    }

    dictCapacity = MIN(MAX(srcSize / 8, (size_t)1024), (size_t)16384);
    dict.buff = FUZZ_malloc(dictCapacity);
    dict.size = dictCapacity;

    nbSamples = (unsigned)MIN(srcSize, (size_t)16);
    if (nbSamples == 0) {
        return dict;
    }
    samplesSizes = (size_t*)FUZZ_malloc(sizeof(*samplesSizes) * nbSamples);
    for (i = 0; i < nbSamples; ++i) {
        size_t const remaining = srcSize - pos;
        size_t const slots = nbSamples - i;
        size_t const sampleSize = remaining / slots + ((remaining % slots) != 0);
        samplesSizes[i] = sampleSize;
        pos += sampleSize;
    }

    dict.size = ZDICT_trainFromBuffer(dict.buff, dictCapacity, src, samplesSizes, nbSamples);
    if (ZSTD_isError(dict.size) || dict.size == 0) {
        dict.size = MIN(dictCapacity, srcSize);
        memcpy(dict.buff, src, dict.size);
    }

    free(samplesSizes);
    return dict;
}
