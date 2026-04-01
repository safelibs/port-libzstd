#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "zstd.h"

#define CHECK(cond, ...)                                                      \
    do {                                                                      \
        if (!(cond)) {                                                        \
            fprintf(stderr, __VA_ARGS__);                                     \
            return 1;                                                         \
        }                                                                     \
    } while (0)

#define CHECK_Z(call)                                                         \
    do {                                                                      \
        size_t const check_z_result = (call);                                 \
        if (ZSTD_isError(check_z_result)) {                                   \
            fprintf(stderr, "%s: %s\n", #call, ZSTD_getErrorName(check_z_result)); \
            return 1;                                                         \
        }                                                                     \
    } while (0)

static unsigned next_random(unsigned* state)
{
    unsigned value = *state;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    *state = value ? value : 1U;
    return *state;
}

static void generate_sample(unsigned char* dst, size_t size, unsigned seed)
{
    static const char* const fragments[] = {
        "bigdict-alpha-",
        "bigdict-beta-",
        "bigdict-gamma-",
        "bigdict-delta-"
    };
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        const char* fragment = fragments[next_random(&state) % 4U];
        size_t const frag_len = strlen(fragment);
        size_t i;
        for (i = 0; i < frag_len && pos < size; ++i) {
            dst[pos++] = (unsigned char)fragment[i];
        }
        for (i = 0; i < 120U && pos < size; ++i) {
            dst[pos++] = (unsigned char)alphabet[next_random(&state) % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) dst[pos++] = '\n';
    }
}

static int pump_decompress(ZSTD_DCtx* dctx,
                           const unsigned char* input,
                           size_t input_size,
                           unsigned char* decoded,
                           size_t decoded_capacity,
                           size_t* decoded_pos)
{
    ZSTD_inBuffer in = { input, input_size, 0 };
    while (in.pos < in.size) {
        ZSTD_outBuffer out = { decoded + *decoded_pos, decoded_capacity - *decoded_pos, 0 };
        size_t const ret = ZSTD_decompressStream(dctx, &out, &in);
        CHECK(!ZSTD_isError(ret), "ZSTD_decompressStream failed: %s\n", ZSTD_getErrorName(ret));
        *decoded_pos += out.pos;
        CHECK(out.pos != 0 || in.pos != 0 || ret == 0, "bigdict decoder made no progress\n");
    }
    return 0;
}

static int compress_and_roundtrip(ZSTD_CCtx* cctx,
                                  ZSTD_DCtx* dctx,
                                  const unsigned char* src,
                                  size_t src_size,
                                  ZSTD_EndDirective end,
                                  unsigned char* decoded,
                                  size_t decoded_capacity,
                                  size_t* decoded_pos)
{
    unsigned char out_buffer[1U << 17];
    ZSTD_inBuffer in = { src, src_size, 0 };
    int finished = 0;

    while (!finished) {
        ZSTD_outBuffer out = { out_buffer, sizeof(out_buffer), 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &out, &in, end);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2 failed: %s\n",
              ZSTD_getErrorName(remaining));
        if (out.pos != 0U) {
            if (pump_decompress(dctx, out_buffer, out.pos, decoded, decoded_capacity, decoded_pos)) {
                return 1;
            }
        }
        if (end == ZSTD_e_end && remaining == 0U) {
            finished = 1;
        } else if (end == ZSTD_e_continue && in.pos == in.size && out.pos == 0U) {
            finished = 1;
        }
        if (end == ZSTD_e_continue && in.pos == in.size && out.pos != 0U) {
            finished = 0;
        }
    }

    return 0;
}

int main(void)
{
    size_t const chunk_size = 2U << 20;
    size_t const repetitions = 4U;
    size_t const total_size = chunk_size * repetitions;
    unsigned char* const chunk = (unsigned char*)malloc(chunk_size);
    unsigned char* const expected = (unsigned char*)malloc(total_size);
    unsigned char* const decoded = (unsigned char*)malloc(total_size);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    size_t decoded_pos = 0;
    size_t i;

    CHECK(chunk != NULL && expected != NULL && decoded != NULL && cctx != NULL && dctx != NULL,
          "allocation failure in bigdict driver\n");

    generate_sample(chunk, chunk_size, 0xBEEFCAFEU);
    for (i = 0; i < repetitions; ++i) {
        memcpy(expected + (i * chunk_size), chunk, chunk_size);
    }

    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, 27));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, 0));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_overlapLog, 9));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, ZSTD_btopt));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_targetLength, 7));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_minMatch, 7));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_searchLog, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_hashLog, 10));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_chainLog, 10));
    CHECK_Z(ZSTD_DCtx_setParameter(dctx, ZSTD_d_windowLogMax, 27));
    CHECK_Z(ZSTD_initDStream((ZSTD_DStream*)dctx));

    for (i = 0; i < repetitions; ++i) {
        ZSTD_EndDirective const end = (i + 1U == repetitions) ? ZSTD_e_end : ZSTD_e_continue;
        if (compress_and_roundtrip(cctx,
                                   dctx,
                                   chunk,
                                   chunk_size,
                                   end,
                                   decoded,
                                   total_size,
                                   &decoded_pos)) {
            return 1;
        }
    }

    CHECK(decoded_pos == total_size, "bigdict decoded size mismatch\n");
    CHECK(memcmp(decoded, expected, total_size) == 0, "bigdict round-trip mismatch\n");

    ZSTD_freeDCtx(dctx);
    ZSTD_freeCCtx(cctx);
    free(decoded);
    free(expected);
    free(chunk);
    return 0;
}
