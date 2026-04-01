#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include "zstd_errors.h"

#define DISPLAY(...) fprintf(stderr, __VA_ARGS__)
#define ARRAY_SIZE(a) (sizeof(a) / sizeof((a)[0]))
#define MAX(a, b) ((a) > (b) ? (a) : (b))

#define CHECK_Z(value)                                                       \
    do {                                                                     \
        size_t const check_z_result = (value);                               \
        if (ZSTD_isError(check_z_result)) {                                  \
            DISPLAY("%s: %s\n", #value, ZSTD_getErrorName(check_z_result));  \
            return 1;                                                        \
        }                                                                    \
    } while (0)

#define CHECK(cond, ...)            \
    do {                            \
        if (!(cond)) {              \
            DISPLAY(__VA_ARGS__);   \
            return 1;               \
        }                           \
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

static void generate_sample(void* buffer, size_t size, unsigned seed)
{
    static const char* const tokens[] = {
        "matchfinder-alpha-",
        "matchfinder-beta-",
        "matchfinder-gamma-",
        "matchfinder-delta-",
    };
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    unsigned char* out = (unsigned char*)buffer;
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        size_t const token = next_random(&state) % ARRAY_SIZE(tokens);
        size_t const token_len = strlen(tokens[token]);
        size_t i;
        for (i = 0; i < token_len && pos < size; ++i) {
            out[pos++] = (unsigned char)tokens[token][i];
        }
        for (i = 0; i < 96U && pos < size; ++i) {
            out[pos++] =
                (unsigned char)alphabet[next_random(&state) % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) {
            out[pos++] = '\n';
        }
    }
}

static int validate_frame_shape(const void* compressed,
                                size_t compressed_size,
                                size_t expected_size)
{
    CHECK(ZSTD_getFrameContentSize(compressed, compressed_size) ==
              (unsigned long long)expected_size,
          "frame content size mismatch\n");
    CHECK(ZSTD_findFrameCompressedSize(compressed, compressed_size) == compressed_size,
          "frame compressed size mismatch\n");
    return 0;
}

static int round_trip(const void* src,
                      size_t src_size,
                      const void* compressed,
                      size_t compressed_size)
{
    unsigned char* const decoded = (unsigned char*)malloc(MAX(src_size, (size_t)1));
    size_t decoded_size;

    if (decoded == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    decoded_size = ZSTD_decompress(decoded,
                                   MAX(src_size, (size_t)1),
                                   compressed,
                                   compressed_size);
    CHECK(!ZSTD_isError(decoded_size), "ZSTD_decompress failed: %s\n",
          ZSTD_getErrorName(decoded_size));
    CHECK(decoded_size == src_size, "decoded size mismatch\n");
    CHECK(memcmp(decoded, src, src_size) == 0, "decoded data mismatch\n");
    free(decoded);
    return validate_frame_shape(compressed, compressed_size, src_size);
}

static int compress_with_params(const void* src,
                                size_t src_size,
                                const void* prefix,
                                size_t prefix_size,
                                int strategy,
                                int enable_ldm,
                                void** compressed_out,
                                size_t* compressed_size_out)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    void* const compressed = malloc(ZSTD_compressBound(src_size));
    size_t compressed_size;
    int ldm_value = 0;

    if (cctx == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 5));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, strategy));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_enableLongDistanceMatching, enable_ldm));
    CHECK_Z(ZSTD_CCtx_getParameter(cctx, ZSTD_c_enableLongDistanceMatching, &ldm_value));
    CHECK(ldm_value == enable_ldm, "unexpected LDM setting %d\n", ldm_value);
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, 27));
    if (prefix != NULL && prefix_size != 0U) {
        CHECK_Z(ZSTD_CCtx_refPrefix(cctx, prefix, prefix_size));
    }

    compressed_size =
        ZSTD_compress2(cctx, compressed, ZSTD_compressBound(src_size), src, src_size);
    CHECK(!ZSTD_isError(compressed_size), "ZSTD_compress2 failed: %s\n",
          ZSTD_getErrorName(compressed_size));

    ZSTD_freeCCtx(cctx);
    *compressed_out = compressed;
    *compressed_size_out = compressed_size;
    return 0;
}

static int check_out_of_bounds_behavior(ZSTD_CCtx* cctx, ZSTD_cParameter param, int value)
{
    size_t const code = ZSTD_CCtx_setParameter(cctx, param, value);
    if (ZSTD_isError(code)) {
        CHECK(ZSTD_getErrorCode(code) == ZSTD_error_parameter_outOfBound,
              "parameter %d returned %s instead of parameter_outOfBound\n",
              (int)param, ZSTD_getErrorName(code));
    }
    return 0;
}

static int test_match_parameter_bounds(void)
{
    static const ZSTD_cParameter params[] = {
        ZSTD_c_hashLog,
        ZSTD_c_chainLog,
        ZSTD_c_searchLog,
        ZSTD_c_minMatch,
        ZSTD_c_targetLength,
        ZSTD_c_strategy,
    };
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    size_t i;

    if (cctx == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    for (i = 0; i < ARRAY_SIZE(params); ++i) {
        ZSTD_bounds const bounds = ZSTD_cParam_getBounds(params[i]);
        CHECK(!ZSTD_isError(bounds.error), "could not query bounds for parameter %d\n",
              (int)params[i]);
        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, params[i], bounds.lowerBound));
        CHECK_Z(ZSTD_CCtx_setParameter(cctx, params[i], bounds.upperBound));
        if (bounds.upperBound < INT_MAX) {
            if (check_out_of_bounds_behavior(cctx, params[i], bounds.upperBound + 1)) {
                ZSTD_freeCCtx(cctx);
                return 1;
            }
        } else if (bounds.lowerBound > INT_MIN) {
            if (check_out_of_bounds_behavior(cctx, params[i], bounds.lowerBound - 1)) {
                ZSTD_freeCCtx(cctx);
                return 1;
            }
        }
    }

    ZSTD_freeCCtx(cctx);
    return 0;
}

static int test_strategy_matrix(void)
{
    static const int strategies[] = {
        ZSTD_fast,
        ZSTD_dfast,
        ZSTD_greedy,
        ZSTD_lazy2,
        ZSTD_btopt,
        ZSTD_btultra2,
    };
    size_t const src_size = 1024U * 1024U + 777U;
    unsigned char* const src = (unsigned char*)malloc(src_size);
    size_t fast_size = 0;
    size_t ultra_size = 0;
    size_t i;

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }
    generate_sample(src, src_size, 123U);

    for (i = 0; i < ARRAY_SIZE(strategies); ++i) {
        void* compressed = NULL;
        size_t compressed_size = 0;
        if (compress_with_params(src,
                                 src_size,
                                 NULL,
                                 0,
                                 strategies[i],
                                 0,
                                 &compressed,
                                 &compressed_size) ||
            round_trip(src, src_size, compressed, compressed_size)) {
            free(compressed);
            free(src);
            return 1;
        }
        if (strategies[i] == ZSTD_fast) {
            fast_size = compressed_size;
        }
        if (strategies[i] == ZSTD_btultra2) {
            ultra_size = compressed_size;
        }
        free(compressed);
    }

    CHECK(ultra_size <= fast_size,
          "btultra2 unexpectedly compressed worse than fast (%u > %u)\n",
          (unsigned)ultra_size, (unsigned)fast_size);
    free(src);
    return 0;
}

static int test_prefix_reference_benefit(void)
{
    size_t const dict_size = 64U * 1024U;
    size_t const src_size = dict_size * 4U;
    unsigned char* const dict = (unsigned char*)malloc(dict_size);
    unsigned char* const src = (unsigned char*)malloc(src_size);
    void* plain_compressed = NULL;
    void* prefixed_compressed = NULL;
    size_t plain_size = 0;
    size_t prefixed_size = 0;
    size_t chunk;

    if (dict == NULL || src == NULL) {
        DISPLAY("allocation failure\n");
        free(dict);
        free(src);
        return 1;
    }

    generate_sample(dict, dict_size, 321U);
    for (chunk = 0; chunk < 4U; ++chunk) {
        memcpy(src + chunk * dict_size, dict, dict_size);
    }

    if (compress_with_params(src,
                             src_size,
                             NULL,
                             0,
                             ZSTD_greedy,
                             0,
                             &plain_compressed,
                             &plain_size) ||
        compress_with_params(src,
                             src_size,
                             dict,
                             dict_size,
                             ZSTD_greedy,
                             0,
                             &prefixed_compressed,
                             &prefixed_size) ||
        round_trip(src, src_size, plain_compressed, plain_size) ||
        validate_frame_shape(prefixed_compressed, prefixed_size, src_size)) {
        free(plain_compressed);
        free(prefixed_compressed);
        free(dict);
        free(src);
        return 1;
    }

    CHECK(prefixed_size < plain_size,
          "prefix reference did not improve compression (%u >= %u)\n",
          (unsigned)prefixed_size, (unsigned)plain_size);
    CHECK(ZSTD_getDictID_fromFrame(prefixed_compressed, prefixed_size) == 0U,
          "prefix reference unexpectedly wrote a dictionary id\n");

    free(plain_compressed);
    free(prefixed_compressed);
    free(dict);
    free(src);
    return 0;
}

static int stream_compress_ldm(const void* src,
                               size_t src_size,
                               void** compressed_out,
                               size_t* compressed_size_out)
{
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    unsigned char* const compressed =
        (unsigned char*)malloc(ZSTD_compressBound(src_size) + 4096U);
    size_t const capacity = ZSTD_compressBound(src_size) + 4096U;
    size_t dst_pos = 0;
    size_t src_pos = 0;
    int ldm_value = 0;

    if (cctx == NULL || compressed == NULL) {
        DISPLAY("allocation failure\n");
        ZSTD_freeCCtx(cctx);
        free(compressed);
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 5));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, ZSTD_btopt));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_enableLongDistanceMatching, 1));
    CHECK_Z(ZSTD_CCtx_getParameter(cctx, ZSTD_c_enableLongDistanceMatching, &ldm_value));
    CHECK(ldm_value == 1, "unexpected streaming LDM setting %d\n", ldm_value);
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, 27));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_contentSizeFlag, 1));
    CHECK_Z(ZSTD_CCtx_setPledgedSrcSize(cctx, src_size));

    while (src_pos < src_size) {
        ZSTD_inBuffer in;
        ZSTD_outBuffer out;
        size_t const chunk_size =
            (src_size - src_pos) > (64U * 1024U) ? (64U * 1024U) : (src_size - src_pos);
        size_t remaining;

        in.src = (const unsigned char*)src + src_pos;
        in.size = chunk_size;
        in.pos = 0;
        do {
            out.dst = compressed + dst_pos;
            out.size = capacity - dst_pos;
            out.pos = 0;
            remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_continue);
            CHECK(!ZSTD_isError(remaining), "stream LDM compression failed: %s\n",
                  ZSTD_getErrorName(remaining));
            dst_pos += out.pos;
            CHECK(out.pos != 0 || in.pos == in.size, "stream LDM compression stalled\n");
        } while (in.pos < in.size);
        src_pos += chunk_size;
    }

    while (1) {
        ZSTD_inBuffer in;
        ZSTD_outBuffer out;
        size_t remaining;

        in.src = "";
        in.size = 0;
        in.pos = 0;
        out.dst = compressed + dst_pos;
        out.size = capacity - dst_pos;
        out.pos = 0;
        remaining = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_end);
        CHECK(!ZSTD_isError(remaining), "stream LDM finalization failed: %s\n",
              ZSTD_getErrorName(remaining));
        dst_pos += out.pos;
        if (remaining == 0) {
            break;
        }
        CHECK(out.pos != 0, "stream LDM finalization stalled\n");
    }

    ZSTD_freeCCtx(cctx);
    *compressed_out = compressed;
    *compressed_size_out = dst_pos;
    return 0;
}

static int test_long_distance_streaming(void)
{
    size_t const pattern_size = 256U * 1024U;
    size_t const gap_size = 1024U * 1024U;
    size_t const src_size = pattern_size + gap_size + pattern_size + gap_size + pattern_size;
    unsigned char* const src = (unsigned char*)malloc(src_size);
    void* compressed = NULL;
    size_t compressed_size = 0;

    if (src == NULL) {
        DISPLAY("allocation failure\n");
        return 1;
    }

    generate_sample(src, pattern_size, 777U);
    generate_sample(src + pattern_size, gap_size, 111U);
    memcpy(src + pattern_size + gap_size, src, pattern_size);
    generate_sample(src + pattern_size + gap_size + pattern_size, gap_size, 222U);
    memcpy(src + pattern_size + gap_size + pattern_size + gap_size, src, pattern_size);

    if (stream_compress_ldm(src, src_size, &compressed, &compressed_size) ||
        round_trip(src, src_size, compressed, compressed_size)) {
        free(compressed);
        free(src);
        return 1;
    }

    free(compressed);
    free(src);
    return 0;
}

int main(void)
{
    if (test_match_parameter_bounds() || test_strategy_matrix() ||
        test_prefix_reference_benefit() || test_long_distance_streaming()) {
        return 1;
    }

    DISPLAY("external_matchfinder: compression-core parameter tests passed\n");
    return 0;
}
