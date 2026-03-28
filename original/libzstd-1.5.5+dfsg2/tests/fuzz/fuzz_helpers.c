/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */
#include "fuzz_helpers.h"

#include <stddef.h>
#include <stdlib.h>
#include <string.h>

void* FUZZ_malloc(size_t size)
{
    if (size > 0) {
        void* const mem = malloc(size);
        FUZZ_ASSERT(mem);
        return mem;
    }
    return NULL;
}

void* FUZZ_malloc_rand(size_t size, FUZZ_dataProducer_t *producer)
{
    if (size > 0) {
        void* const mem = malloc(size);
        FUZZ_ASSERT(mem);
        return mem;
    } else {
        uintptr_t ptr = 0;
        /* Add +- 1M 50% of the time */
        if (FUZZ_dataProducer_uint32Range(producer, 0, 1))
            FUZZ_dataProducer_int32Range(producer, -1000000, 1000000);
        return (void*)ptr;
    }

}

int FUZZ_memcmp(void const* lhs, void const* rhs, size_t size)
{
    if (size == 0) {
        return 0;
    }
    return memcmp(lhs, rhs, size);
}

uint64_t FUZZ_hashBuffer(void const* ptr, size_t size)
{
    unsigned char const* const bytes = (const unsigned char*)ptr;
    uint64_t hash = 1469598103934665603ULL;
    size_t i;

    for (i = 0; i < size; ++i) {
        hash ^= bytes[i];
        hash *= 1099511628211ULL;
    }

    return hash;
}
