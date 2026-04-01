#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include "zstd_errors.h"

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
        "stream-alpha-",
        "stream-beta-",
        "stream-gamma-"
    };
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        const char* fragment = fragments[next_random(&state) % 3U];
        size_t const fragment_len = strlen(fragment);
        size_t i;
        for (i = 0; i < fragment_len && pos < size; ++i) {
            dst[pos++] = (unsigned char)fragment[i];
        }
        for (i = 0; i < 96U && pos < size; ++i) {
            dst[pos++] = (unsigned char)alphabet[next_random(&state) % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) dst[pos++] = '\n';
    }
}

static void generate_compressible_sample(unsigned char* dst, size_t size)
{
    static const char pattern[] =
        "stream-compressible-payload-alpha-beta-gamma-delta\n";
    size_t pos = 0;

    while (pos < size) {
        size_t const chunk = (size - pos) < (sizeof(pattern) - 1U)
                                 ? (size - pos)
                                 : (sizeof(pattern) - 1U);
        memcpy(dst + pos, pattern, chunk);
        pos += chunk;
    }
}

static void generate_dict_biased_sample(unsigned char* dst,
                                        size_t dst_size,
                                        const unsigned char* dict,
                                        size_t dict_size,
                                        unsigned seed)
{
    size_t pos = 0;
    size_t cursor = seed % dict_size;

    while (pos < dst_size) {
        size_t chunk = 64U + ((seed + (unsigned)pos) % 96U);
        if (chunk > dict_size) chunk = dict_size;
        if (chunk > dst_size - pos) chunk = dst_size - pos;
        if (cursor + chunk > dict_size) {
            cursor = (cursor + 97U + (seed % 29U)) % dict_size;
            if (cursor + chunk > dict_size) {
                chunk = dict_size - cursor;
            }
        }
        memcpy(dst + pos, dict + cursor, chunk);
        if (chunk > 12U) {
            dst[pos + 3U] ^= 0x11U;
            dst[pos + chunk / 2U] ^= 0x5AU;
        }
        pos += chunk;
        if (pos < dst_size) dst[pos++] = '\n';
        cursor = (cursor + 131U + (seed % 23U)) % dict_size;
    }
}

static int read_file(const char* path, unsigned char** data_out, size_t* size_out)
{
    FILE* file = fopen(path, "rb");
    long size;
    unsigned char* data;
    size_t read_size;

    CHECK(file != NULL, "failed to open %s\n", path);
    CHECK(fseek(file, 0, SEEK_END) == 0, "failed to seek %s\n", path);
    size = ftell(file);
    CHECK(size > 0, "unexpected size for %s\n", path);
    CHECK(fseek(file, 0, SEEK_SET) == 0, "failed to rewind %s\n", path);
    data = (unsigned char*)malloc((size_t)size);
    CHECK(data != NULL, "allocation failure reading %s\n", path);
    read_size = fread(data, 1, (size_t)size, file);
    fclose(file);
    CHECK(read_size == (size_t)size, "failed to read %s\n", path);

    *data_out = data;
    *size_out = (size_t)size;
    return 0;
}

static int decompress_using_dict_exact(const void* compressed,
                                       size_t compressed_size,
                                       const void* dict,
                                       size_t dict_size,
                                       unsigned char* decoded,
                                       size_t decoded_capacity,
                                       const unsigned char* expected,
                                       size_t expected_size)
{
    {
        ZSTD_DCtx* const dctx = ZSTD_createDCtx();
        size_t const decoded_size = ZSTD_decompress_usingDict(dctx, decoded, decoded_capacity,
                                                              compressed, compressed_size,
                                                              dict, dict_size);
        ZSTD_freeDCtx(dctx);
        CHECK(!ZSTD_isError(decoded_size),
              "ZSTD_decompress_usingDict failed: %s\n",
              ZSTD_getErrorName(decoded_size));
        CHECK(decoded_size == expected_size, "decoded size mismatch\n");
        CHECK(memcmp(decoded, expected, expected_size) == 0, "decoded payload mismatch\n");
    }
    return 0;
}

static int decompress_exact(const void* compressed,
                            size_t compressed_size,
                            unsigned char* decoded,
                            size_t decoded_capacity,
                            const unsigned char* expected,
                            size_t expected_size)
{
    size_t const decoded_size =
        ZSTD_decompress(decoded, decoded_capacity, compressed, compressed_size);
    CHECK(!ZSTD_isError(decoded_size), "ZSTD_decompress failed: %s\n",
          ZSTD_getErrorName(decoded_size));
    CHECK(decoded_size == expected_size, "decoded size mismatch\n");
    CHECK(memcmp(decoded, expected, expected_size) == 0, "decoded payload mismatch\n");
    return 0;
}

static unsigned frame_first_block_type(const unsigned char* frame, size_t frame_size)
{
    size_t const header_size = ZSTD_frameHeaderSize(frame, frame_size);
    unsigned block_header;

    CHECK(header_size >= 5U, "invalid frame header size\n");
    CHECK(frame_size >= header_size + 3U, "frame too short for a block header\n");
    block_header = (unsigned)frame[header_size]
                 | ((unsigned)frame[header_size + 1U] << 8)
                 | ((unsigned)frame[header_size + 2U] << 16);
    return (block_header >> 1) & 0x3U;
}

static unsigned frame_descriptor(const unsigned char* frame, size_t frame_size)
{
    size_t const header_size = ZSTD_frameHeaderSize(frame, frame_size);

    CHECK(header_size >= 5U, "invalid frame header size\n");
    CHECK(frame_size >= header_size, "frame too short for header\n");
    return frame[4];
}

static int frame_has_checksum(const unsigned char* frame, size_t frame_size)
{
    return (frame_descriptor(frame, frame_size) & 0x4U) != 0U;
}

static int compress_stream_legacy(ZSTD_CStream* zcs,
                                  const unsigned char* src,
                                  size_t src_size,
                                  unsigned char* compressed,
                                  size_t compressed_capacity,
                                  size_t* compressed_size,
                                  int* produced_before_end)
{
    ZSTD_inBuffer input = { src, src_size, 0 };
    size_t written = 0;

    for (;;) {
        ZSTD_outBuffer output = { compressed + written, compressed_capacity - written, 0 };
        size_t const remaining = ZSTD_compressStream(zcs, &output, &input);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream failed: %s\n",
              ZSTD_getErrorName(remaining));
        if (produced_before_end != NULL && output.pos > 0U) {
            *produced_before_end = 1;
        }
        written += output.pos;
        if (input.pos == input.size) {
            break;
        }
    }

    for (;;) {
        ZSTD_outBuffer output = { compressed + written, compressed_capacity - written, 0 };
        size_t const remaining = ZSTD_endStream(zcs, &output);
        CHECK(!ZSTD_isError(remaining), "ZSTD_endStream failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
        if (remaining == 0U) {
            break;
        }
    }

    *compressed_size = written;
    return 0;
}

static int compress_stream2_continue_then_end(ZSTD_CCtx* cctx,
                                              const unsigned char* src,
                                              size_t src_size,
                                              unsigned char* compressed,
                                              size_t compressed_capacity,
                                              size_t* compressed_size,
                                              int* produced_before_end)
{
    size_t const split = src_size / 2U;
    ZSTD_inBuffer first = { src, split, 0 };
    ZSTD_inBuffer second = { src + split, src_size - split, 0 };
    size_t written = 0;

    for (;;) {
        ZSTD_outBuffer output = { compressed + written, compressed_capacity - written, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &output, &first, ZSTD_e_continue);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(continue) failed: %s\n",
              ZSTD_getErrorName(remaining));
        if (produced_before_end != NULL && output.pos > 0U) {
            *produced_before_end = 1;
        }
        written += output.pos;
        if (first.pos == first.size && remaining == 0U) {
            break;
        }
    }

    for (;;) {
        ZSTD_outBuffer output = { compressed + written, compressed_capacity - written, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &output, &second, ZSTD_e_end);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(end) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
        if (remaining == 0U) {
            break;
        }
    }

    *compressed_size = written;
    return 0;
}

static int compress_stream_legacy_flush_then_end(ZSTD_CStream* zcs,
                                                 const unsigned char* src,
                                                 size_t src_size,
                                                 unsigned char* compressed,
                                                 size_t compressed_capacity,
                                                 size_t* compressed_size,
                                                 int* flushed_output)
{
    size_t const chunk_capacity = 97U;
    size_t const split = src_size / 2U;
    ZSTD_inBuffer first = { src, split, 0 };
    ZSTD_inBuffer second = { src + split, src_size - split, 0 };
    size_t written = 0;

    {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_compressStream(zcs, &output, &first);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream(pre-flush) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
    }

    for (;;) {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_flushStream(zcs, &output);
        CHECK(!ZSTD_isError(remaining), "ZSTD_flushStream failed: %s\n",
              ZSTD_getErrorName(remaining));
        if (flushed_output != NULL && output.pos > 0U) {
            *flushed_output = 1;
        }
        written += output.pos;
        if (remaining == 0U) {
            break;
        }
    }

    for (;;) {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_compressStream(zcs, &output, &first);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream(post-flush first) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
        if (first.pos == first.size) {
            break;
        }
    }

    for (;;) {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_compressStream(zcs, &output, &second);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream(post-flush) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
        if (second.pos == second.size) {
            break;
        }
    }

    for (;;) {
        ZSTD_outBuffer output = { compressed + written, compressed_capacity - written, 0 };
        size_t const remaining = ZSTD_endStream(zcs, &output);
        CHECK(!ZSTD_isError(remaining), "ZSTD_endStream(post-flush) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
        if (remaining == 0U) {
            break;
        }
    }

    *compressed_size = written;
    return 0;
}

static int compress_stream2_continue_flush_then_end(ZSTD_CCtx* cctx,
                                                    const unsigned char* src,
                                                    size_t src_size,
                                                    unsigned char* compressed,
                                                    size_t compressed_capacity,
                                                    size_t* compressed_size,
                                                    int* flushed_output)
{
    size_t const chunk_capacity = 113U;
    size_t const split = src_size / 2U;
    ZSTD_inBuffer first = { src, split, 0 };
    ZSTD_inBuffer second = { src + split, src_size - split, 0 };
    ZSTD_inBuffer flush_input = { NULL, 0, 0 };
    size_t written = 0;

    {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &output, &first, ZSTD_e_continue);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(pre-flush) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
    }

    for (;;) {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &output, &flush_input, ZSTD_e_flush);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(flush) failed: %s\n",
              ZSTD_getErrorName(remaining));
        if (flushed_output != NULL && output.pos > 0U) {
            *flushed_output = 1;
        }
        written += output.pos;
        if (remaining == 0U) {
            break;
        }
    }

    for (;;) {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &output, &first, ZSTD_e_continue);
        CHECK(!ZSTD_isError(remaining),
              "ZSTD_compressStream2(post-flush first) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
        if (first.pos == first.size && remaining == 0U) {
            break;
        }
    }

    for (;;) {
        size_t const capacity =
            (compressed_capacity - written) < chunk_capacity ? (compressed_capacity - written)
                                                             : chunk_capacity;
        ZSTD_outBuffer output = { compressed + written, capacity, 0 };
        size_t const remaining = ZSTD_compressStream2(cctx, &output, &second, ZSTD_e_end);
        CHECK(!ZSTD_isError(remaining), "ZSTD_compressStream2(end after flush) failed: %s\n",
              ZSTD_getErrorName(remaining));
        written += output.pos;
        if (remaining == 0U) {
            break;
        }
    }

    *compressed_size = written;
    return 0;
}

int main(int argc, char** argv)
{
    size_t const src_size = (384U << 10) + 37U;
    size_t const compressible_size = (96U << 10) + 29U;
    size_t const tuned_src_size = (8U << 10) + 17U;
    unsigned char* dict = NULL;
    size_t dict_size = 0;
    unsigned char* const src = (unsigned char*)malloc(src_size);
    unsigned char* const compressible = (unsigned char*)malloc(compressible_size);
    unsigned char* const dict_src = (unsigned char*)malloc(src_size);
    unsigned char* const decoded = (unsigned char*)malloc(src_size);
    unsigned char* const compressed = (unsigned char*)malloc(ZSTD_compressBound(src_size));
    ZSTD_CStream* const zcs = ZSTD_createCStream();
    ZSTD_CStream* const zcs2 = ZSTD_createCStream();
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    size_t compressed_size = 0;
    int produced_before_end = 0;
    ZSTD_CDict* dict_handle;
    ZSTD_CDict* tuned_dict_handle = NULL;
    ZSTD_CCtx_params* tuned_params = NULL;
    int tuned_window_log = 0;

    if (argc != 2) {
        fprintf(stderr, "usage: %s DICTIONARY\n", argv[0]);
        return 1;
    }
    if (read_file(argv[1], &dict, &dict_size)) {
        return 1;
    }

    CHECK(src != NULL && compressible != NULL && dict_src != NULL &&
              decoded != NULL && compressed != NULL && zcs != NULL &&
              zcs2 != NULL && cctx != NULL,
          "allocation failure in zstream driver\n");

    generate_sample(src, src_size, 0x1234ABCDU);
    generate_compressible_sample(compressible, compressible_size);
    generate_dict_biased_sample(dict_src, src_size, dict, dict_size, 0x55AA11CCU);
    dict_handle = ZSTD_createCDict(dict, dict_size, 4);
    tuned_params = ZSTD_createCCtxParams();
    CHECK(dict_handle != NULL && tuned_params != NULL, "failed to create test CDict state\n");
    CHECK_Z(ZSTD_CCtxParams_init(tuned_params, 4));
    CHECK_Z(ZSTD_CCtxParams_setParameter(tuned_params, ZSTD_c_windowLog, 14));
    tuned_dict_handle = ZSTD_createCDict_advanced2(
        dict,
        dict_size,
        ZSTD_dlm_byRef,
        ZSTD_dct_fullDict,
        tuned_params,
        (ZSTD_customMem){ 0 });
    CHECK(tuned_dict_handle != NULL, "failed to create tuned CDict\n");

    CHECK_Z(ZSTD_CCtx_setParameter(zcs, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_initCStream(zcs, 4));
    {
        size_t const hint_len = 32U << 10;
        ZSTD_inBuffer hint_input = { compressible, hint_len, 0 };
        ZSTD_outBuffer hint_output = { compressed, ZSTD_compressBound(hint_len), 0 };
        size_t const hint = ZSTD_compressStream(zcs, &hint_output, &hint_input);
        CHECK(!ZSTD_isError(hint), "ZSTD_compressStream(hint) failed: %s\n",
              ZSTD_getErrorName(hint));
        CHECK(hint_input.pos == hint_len, "ZSTD_compressStream(hint) did not consume input\n");
        CHECK(hint == ZSTD_CStreamInSize() - hint_len,
              "ZSTD_compressStream returned %zu instead of next-input hint %zu\n",
              hint, ZSTD_CStreamInSize() - hint_len);
    }
    CHECK_Z(ZSTD_initCStream(zcs, 4));
    produced_before_end = 0;
    if (compress_stream_legacy_flush_then_end(zcs, compressible, compressible_size,
                                              compressed, ZSTD_compressBound(src_size),
                                              &compressed_size, &produced_before_end)) {
        return 1;
    }
    CHECK(frame_first_block_type(compressed, compressed_size) == 2U,
          "ZSTD_flushStream path did not emit a compressed block for compressible input\n");
    CHECK(compressed_size < compressible_size,
          "ZSTD_flushStream path failed to shrink compressible input\n");
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_flushStream path did not preserve checksum flag\n");
    if (decompress_exact(compressed, compressed_size, decoded,
                         compressible_size, compressible, compressible_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_initCStream(zcs, 4));
    produced_before_end = 0;
    if (compress_stream_legacy(zcs, compressible, compressible_size,
                               compressed, ZSTD_compressBound(src_size),
                               &compressed_size, &produced_before_end)) {
        return 1;
    }
    CHECK(produced_before_end, "ZSTD_compressStream withheld all output until end\n");
    CHECK(frame_first_block_type(compressed, compressed_size) == 2U,
          "ZSTD_initCStream did not emit a compressed block for compressible input\n");
    CHECK(compressed_size < compressible_size,
          "ZSTD_initCStream failed to shrink compressible input\n");
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_initCStream did not preserve checksum flag\n");
    if (decompress_exact(compressed, compressed_size, decoded,
                         compressible_size, compressible, compressible_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_resetCStream(zcs, 0));
    if (compress_stream_legacy(zcs, src, src_size / 2U, compressed, ZSTD_compressBound(src_size),
                               &compressed_size, NULL)) {
        return 1;
    }
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_resetCStream(0) did not preserve checksum flag\n");
    CHECK(ZSTD_getFrameContentSize(compressed, compressed_size) == ZSTD_CONTENTSIZE_UNKNOWN,
          "ZSTD_resetCStream(0) did not normalize pledged size to unknown\n");
    if (decompress_exact(compressed, compressed_size, decoded, src_size / 2U, src, src_size / 2U)) {
        return 1;
    }

    CHECK_Z(ZSTD_initCStream_srcSize(zcs, 4, 0));
    if (compress_stream_legacy(zcs, src, (96U << 10), compressed, ZSTD_compressBound(src_size),
                               &compressed_size, NULL)) {
        return 1;
    }
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_initCStream_srcSize(0) did not preserve checksum flag\n");
    CHECK(ZSTD_getFrameContentSize(compressed, compressed_size) == ZSTD_CONTENTSIZE_UNKNOWN,
          "ZSTD_initCStream_srcSize(0) did not normalize pledged size to unknown\n");
    if (decompress_exact(compressed, compressed_size, decoded, (96U << 10), src, (96U << 10))) {
        return 1;
    }

    CHECK_Z(ZSTD_initCStream_srcSize(zcs, 4, src_size));
    if (compress_stream_legacy(zcs, src, src_size, compressed, ZSTD_compressBound(src_size),
                               &compressed_size, NULL)) {
        return 1;
    }
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_initCStream_srcSize did not preserve checksum flag\n");
    CHECK(ZSTD_getFrameContentSize(compressed, compressed_size) == src_size,
          "ZSTD_initCStream_srcSize lost exact pledged size\n");
    if (decompress_exact(compressed, compressed_size, decoded, src_size, src, src_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_initCStream_usingDict(zcs, dict, dict_size, 4));
    produced_before_end = 0;
    if (compress_stream_legacy(zcs, dict_src, src_size, compressed, ZSTD_compressBound(src_size),
                               &compressed_size, &produced_before_end)) {
        return 1;
    }
    CHECK(produced_before_end, "ZSTD_initCStream_usingDict buffered until end\n");
    CHECK(frame_first_block_type(compressed, compressed_size) == 2U,
          "ZSTD_initCStream_usingDict did not emit a compressed block\n");
    CHECK(compressed_size < src_size,
          "ZSTD_initCStream_usingDict failed to shrink the source\n");
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_initCStream_usingDict did not preserve checksum flag\n");
    if (decompress_using_dict_exact(compressed, compressed_size, dict, dict_size,
                                    decoded, src_size, dict_src, src_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_setParameter(zcs2, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_initCStream_usingCDict(zcs2, dict_handle));
    if (compress_stream_legacy(zcs2, dict_src, src_size, compressed, ZSTD_compressBound(src_size),
                               &compressed_size, NULL)) {
        return 1;
    }
    CHECK(frame_first_block_type(compressed, compressed_size) == 2U,
          "ZSTD_initCStream_usingCDict did not emit a compressed block\n");
    CHECK(compressed_size < src_size,
          "ZSTD_initCStream_usingCDict failed to shrink the source\n");
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_initCStream_usingCDict did not preserve checksum flag\n");
    if (decompress_using_dict_exact(compressed, compressed_size, dict, dict_size,
                                    decoded, src_size, dict_src, src_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_initCStream_advanced(
        zcs2, NULL, 0,
        (ZSTD_parameters){
            ZSTD_getCParams(4, src_size, dict_size),
            (ZSTD_frameParameters){ 0, 1, 1 }
        },
        0));
    if (compress_stream_legacy(zcs2, src, (80U << 10), compressed, ZSTD_compressBound(src_size),
                               &compressed_size, NULL)) {
        return 1;
    }
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_initCStream_advanced(0) did not preserve checksum flag\n");
    CHECK(ZSTD_getFrameContentSize(compressed, compressed_size) == ZSTD_CONTENTSIZE_UNKNOWN,
          "ZSTD_initCStream_advanced(0) did not normalize pledged size to unknown\n");
    if (decompress_exact(compressed, compressed_size, decoded, (80U << 10), src, (80U << 10))) {
        return 1;
    }

    CHECK_Z(ZSTD_initCStream_usingCDict_advanced(
        zcs2, dict_handle,
        (ZSTD_frameParameters){ 1, 1, 0 },
        src_size));
    if (compress_stream_legacy(zcs2, dict_src, src_size, compressed, ZSTD_compressBound(src_size),
                               &compressed_size, NULL)) {
        return 1;
    }
    CHECK(frame_first_block_type(compressed, compressed_size) == 2U,
          "ZSTD_initCStream_usingCDict_advanced did not emit a compressed block\n");
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_initCStream_usingCDict_advanced lost checksum flag\n");
    if (decompress_using_dict_exact(compressed, compressed_size, dict, dict_size,
                                    decoded, src_size, dict_src, src_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_compressBegin_usingCDict(cctx, tuned_dict_handle));
    CHECK(ZSTD_getBlockSize(cctx) == (1U << 14),
          "tuned CDict did not preserve its windowLog-derived block size\n");
    CHECK_Z(ZSTD_CCtx_getParameter(cctx, ZSTD_c_windowLog, &tuned_window_log));
    CHECK(tuned_window_log == 14, "unexpected tuned CDict windowLog %d\n", tuned_window_log);
    compressed_size = ZSTD_compress_usingCDict_advanced(
        cctx,
        compressed,
        ZSTD_compressBound(tuned_src_size),
        dict_src,
        tuned_src_size,
        tuned_dict_handle,
        (ZSTD_frameParameters){ 1, 0, 0 });
    CHECK(!ZSTD_isError(compressed_size), "ZSTD_compress_usingCDict_advanced(tuned) failed: %s\n",
          ZSTD_getErrorName(compressed_size));
    if (decompress_using_dict_exact(compressed, compressed_size, dict, dict_size,
                                    decoded, tuned_src_size, dict_src, tuned_src_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    produced_before_end = 0;
    if (compress_stream2_continue_flush_then_end(cctx, compressible, compressible_size,
                                                 compressed, ZSTD_compressBound(src_size),
                                                 &compressed_size, &produced_before_end)) {
        return 1;
    }
    CHECK(frame_first_block_type(compressed, compressed_size) == 2U,
          "ZSTD_e_flush path did not emit a compressed block for compressible input\n");
    CHECK(compressed_size < compressible_size,
          "ZSTD_e_flush path failed to shrink compressible input\n");
    CHECK(frame_has_checksum(compressed, compressed_size),
          "ZSTD_e_flush path did not preserve checksum flag\n");
    if (decompress_exact(compressed, compressed_size, decoded,
                         compressible_size, compressible, compressible_size)) {
        return 1;
    }

    CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    produced_before_end = 0;
    if (compress_stream2_continue_then_end(cctx, src, src_size, compressed,
                                           ZSTD_compressBound(src_size),
                                           &compressed_size, &produced_before_end)) {
        return 1;
    }
    CHECK(produced_before_end, "ZSTD_compressStream2 withheld all output until end\n");
    if (decompress_exact(compressed, compressed_size, decoded, src_size, src, src_size)) {
        return 1;
    }

    ZSTD_freeCCtxParams(tuned_params);
    ZSTD_freeCDict(tuned_dict_handle);
    ZSTD_freeCDict(dict_handle);
    ZSTD_freeCCtx(cctx);
    ZSTD_freeCStream(zcs2);
    ZSTD_freeCStream(zcs);
    free(compressed);
    free(decoded);
    free(compressible);
    free(dict_src);
    free(src);
    free(dict);
    return 0;
}
