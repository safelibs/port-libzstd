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
 * This fuzz target exercises public frame-info helpers on arbitrary input.
 */

#include <stddef.h>
#include <stdint.h>

#include "zstd.h"

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size);

int LLVMFuzzerTestOneInput(const uint8_t* src, size_t size)
{
    if (size == 0) {
        src = NULL;
    }
    ZSTD_getFrameContentSize(src, size);
    ZSTD_getDecompressedSize(src, size);
    ZSTD_findFrameCompressedSize(src, size);
    ZSTD_getDictID_fromFrame(src, size);
    return 0;
}
