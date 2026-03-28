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
 * Public metadata/decompression robustness fuzzer replacing the internal FSE test.
 */

#include <stddef.h>
#include <stdlib.h>
#include <stdint.h>

#include "fuzz_data_producer.h"
#include "zstd.h"

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size);

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    FUZZ_dataProducer_t* producer = FUZZ_dataProducer_create(src, size);
    size_t const maxOutSize = size * 4 + 64;
    size_t const outSize = FUZZ_dataProducer_uint32Range(
        producer, 1, maxOutSize == 0 ? (size_t)1 : maxOutSize);
    void* out = malloc(outSize);

    ZSTD_getFrameContentSize(src, size);
    ZSTD_getDecompressedSize(src, size);
    ZSTD_findFrameCompressedSize(src, size);
    ZSTD_getDictID_fromFrame(src, size);
    ZSTD_decompress(out, outSize, src, size);

    free(out);
    FUZZ_dataProducer_free(producer);
    return 0;
}
