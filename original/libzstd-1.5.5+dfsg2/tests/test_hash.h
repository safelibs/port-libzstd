/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#ifndef TEST_HASH_H
#define TEST_HASH_H

#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef uint64_t TEST_hash64_t;

typedef struct {
    uint8_t digest[8];
} TEST_hash64_canonical_t;

typedef struct {
    uint64_t totalLen;
    uint64_t seed;
    uint64_t v1;
    uint64_t v2;
    uint64_t v3;
    uint64_t v4;
    size_t memsize;
    uint8_t mem[32];
} TEST_hash64_state_t;

#if defined(__GNUC__) || defined(__clang__)
#  define TEST_HASH_STATIC static inline __attribute__((unused))
#else
#  define TEST_HASH_STATIC static inline
#endif

TEST_HASH_STATIC uint64_t TEST_rotl64(uint64_t value, unsigned shift)
{
    return (value << shift) | (value >> (64 - shift));
}

TEST_HASH_STATIC uint32_t TEST_readLE32(void const* src)
{
    uint8_t const* const bytes = (const uint8_t*)src;
    return ((uint32_t)bytes[0])
         | ((uint32_t)bytes[1] << 8)
         | ((uint32_t)bytes[2] << 16)
         | ((uint32_t)bytes[3] << 24);
}

TEST_HASH_STATIC uint64_t TEST_readLE64(void const* src)
{
    uint8_t const* const bytes = (const uint8_t*)src;
    return ((uint64_t)bytes[0])
         | ((uint64_t)bytes[1] << 8)
         | ((uint64_t)bytes[2] << 16)
         | ((uint64_t)bytes[3] << 24)
         | ((uint64_t)bytes[4] << 32)
         | ((uint64_t)bytes[5] << 40)
         | ((uint64_t)bytes[6] << 48)
         | ((uint64_t)bytes[7] << 56);
}

TEST_HASH_STATIC uint64_t TEST_hash64_round(uint64_t acc, uint64_t input)
{
    acc += input * 14029467366897019727ULL;
    acc = TEST_rotl64(acc, 31);
    acc *= 11400714785074694791ULL;
    return acc;
}

TEST_HASH_STATIC uint64_t TEST_hash64_mergeRound(uint64_t acc, uint64_t value)
{
    acc ^= TEST_hash64_round(0, value);
    acc = acc * 11400714785074694791ULL + 9650029242287828579ULL;
    return acc;
}

TEST_HASH_STATIC void TEST_hash64_reset(TEST_hash64_state_t* state, uint64_t seed)
{
    state->seed = seed;
    state->totalLen = 0;
    state->memsize = 0;
    state->v1 = seed + 11400714785074694791ULL + 14029467366897019727ULL;
    state->v2 = seed + 14029467366897019727ULL;
    state->v3 = seed;
    state->v4 = seed - 11400714785074694791ULL;
}

TEST_HASH_STATIC void TEST_hash64_consumeStripe(TEST_hash64_state_t* state, uint8_t const* ptr)
{
    state->v1 = TEST_hash64_round(state->v1, TEST_readLE64(ptr));
    state->v2 = TEST_hash64_round(state->v2, TEST_readLE64(ptr + 8));
    state->v3 = TEST_hash64_round(state->v3, TEST_readLE64(ptr + 16));
    state->v4 = TEST_hash64_round(state->v4, TEST_readLE64(ptr + 24));
}

TEST_HASH_STATIC void TEST_hash64_update(TEST_hash64_state_t* state, void const* input, size_t length)
{
    uint8_t const* ptr = (const uint8_t*)input;
    uint8_t const* const end = ptr + length;

    if (length == 0) {
        return;
    }

    state->totalLen += length;

    if (state->memsize + length < sizeof(state->mem)) {
        memcpy(state->mem + state->memsize, input, length);
        state->memsize += length;
        return;
    }

    if (state->memsize > 0) {
        size_t const fill = sizeof(state->mem) - state->memsize;
        memcpy(state->mem + state->memsize, ptr, fill);
        TEST_hash64_consumeStripe(state, state->mem);
        ptr += fill;
        state->memsize = 0;
    }

    while (ptr + sizeof(state->mem) <= end) {
        TEST_hash64_consumeStripe(state, ptr);
        ptr += sizeof(state->mem);
    }

    if (ptr < end) {
        state->memsize = (size_t)(end - ptr);
        memcpy(state->mem, ptr, state->memsize);
    }
}

TEST_HASH_STATIC uint64_t TEST_hash64_digest(TEST_hash64_state_t const* state)
{
    uint64_t hash;
    uint8_t const* ptr = state->mem;
    uint8_t const* const end = ptr + state->memsize;

    if (state->totalLen >= sizeof(state->mem)) {
        hash = TEST_rotl64(state->v1, 1)
             + TEST_rotl64(state->v2, 7)
             + TEST_rotl64(state->v3, 12)
             + TEST_rotl64(state->v4, 18);
        hash = TEST_hash64_mergeRound(hash, state->v1);
        hash = TEST_hash64_mergeRound(hash, state->v2);
        hash = TEST_hash64_mergeRound(hash, state->v3);
        hash = TEST_hash64_mergeRound(hash, state->v4);
    } else {
        hash = state->seed + 2870177450012600261ULL;
    }

    hash += state->totalLen;

    while (ptr + 8 <= end) {
        uint64_t const lane = TEST_hash64_round(0, TEST_readLE64(ptr));
        hash ^= lane;
        hash = TEST_rotl64(hash, 27) * 11400714785074694791ULL + 9650029242287828579ULL;
        ptr += 8;
    }

    if (ptr + 4 <= end) {
        hash ^= (uint64_t)TEST_readLE32(ptr) * 11400714785074694791ULL;
        hash = TEST_rotl64(hash, 23) * 14029467366897019727ULL + 1609587929392839161ULL;
        ptr += 4;
    }

    while (ptr < end) {
        hash ^= (*ptr) * 2870177450012600261ULL;
        hash = TEST_rotl64(hash, 11) * 11400714785074694791ULL;
        ++ptr;
    }

    hash ^= hash >> 33;
    hash *= 14029467366897019727ULL;
    hash ^= hash >> 29;
    hash *= 1609587929392839161ULL;
    hash ^= hash >> 32;
    return hash;
}

TEST_HASH_STATIC TEST_hash64_t TEST_hash64(void const* input, size_t length, uint64_t seed)
{
    TEST_hash64_state_t state;
    TEST_hash64_reset(&state, seed);
    TEST_hash64_update(&state, input, length);
    return TEST_hash64_digest(&state);
}

TEST_HASH_STATIC void TEST_hash64_canonicalFromHash(TEST_hash64_canonical_t* dst, TEST_hash64_t hash)
{
    dst->digest[0] = (uint8_t)(hash >> 56);
    dst->digest[1] = (uint8_t)(hash >> 48);
    dst->digest[2] = (uint8_t)(hash >> 40);
    dst->digest[3] = (uint8_t)(hash >> 32);
    dst->digest[4] = (uint8_t)(hash >> 24);
    dst->digest[5] = (uint8_t)(hash >> 16);
    dst->digest[6] = (uint8_t)(hash >> 8);
    dst->digest[7] = (uint8_t)hash;
}

TEST_HASH_STATIC TEST_hash64_t TEST_hash64_fromCanonical(TEST_hash64_canonical_t const* src)
{
    return ((uint64_t)src->digest[0] << 56)
         | ((uint64_t)src->digest[1] << 48)
         | ((uint64_t)src->digest[2] << 40)
         | ((uint64_t)src->digest[3] << 32)
         | ((uint64_t)src->digest[4] << 24)
         | ((uint64_t)src->digest[5] << 16)
         | ((uint64_t)src->digest[6] << 8)
         | ((uint64_t)src->digest[7]);
}

#endif
