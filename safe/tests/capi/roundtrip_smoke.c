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
        "{\"tenant\":\"alpha\",\"region\":\"west\",\"kind\":\"session\",\"payload\":\"",
        "{\"tenant\":\"beta\",\"region\":\"east\",\"kind\":\"metric\",\"payload\":\"",
        "{\"tenant\":\"gamma\",\"region\":\"north\",\"kind\":\"record\",\"payload\":\""
    };
    static const char alphabet[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        const char* fragment = fragments[next_random(&state) % 3U];
        size_t const frag_len = strlen(fragment);
        size_t i;
        for (i = 0; i < frag_len && pos < size; ++i) {
            dst[pos++] = (unsigned char)fragment[i];
        }
        for (i = 0; i < 96U && pos < size; ++i) {
            dst[pos++] = (unsigned char)alphabet[next_random(&state) % (sizeof(alphabet) - 1U)];
        }
        if (pos < size) dst[pos++] = '"';
        if (pos < size) dst[pos++] = '}';
        if (pos < size) dst[pos++] = '\n';
    }
}

static void generate_compressible_sample(unsigned char* dst, size_t size)
{
    static const char pattern[] =
        "fn compressible_payload() { return \"alpha-beta-gamma-delta\"; }\n";
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
        size_t chunk = 48U + ((seed + (unsigned)pos) % 80U);
        if (chunk > dict_size) chunk = dict_size;
        if (chunk > dst_size - pos) chunk = dst_size - pos;
        if (cursor + chunk > dict_size) {
            cursor = (cursor + 131U + (seed % 29U)) % dict_size;
            if (cursor + chunk > dict_size) {
                chunk = dict_size - cursor;
            }
            if (chunk == 0) {
                cursor = 0;
                chunk = dict_size < (dst_size - pos) ? dict_size : (dst_size - pos);
            }
        }

        memcpy(dst + pos, dict + cursor, chunk);
        if (chunk > 12U) {
            dst[pos + 3U] ^= (unsigned char)(0x11U + (unsigned)(pos >> 5));
            dst[pos + chunk / 2U] ^= (unsigned char)(0x5AU + (unsigned)(pos >> 4));
        }
        pos += chunk;
        if (pos < dst_size) {
            dst[pos++] = (unsigned char)'\n';
        }
        cursor = (cursor + 97U + (seed % 23U)) % dict_size;
    }
}

static void generate_noise_sample(unsigned char* dst, size_t size, unsigned seed)
{
    size_t pos = 0;
    unsigned state = seed | 1U;

    while (pos < size) {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        dst[pos++] = (unsigned char)(state & 0xFFU);
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

static int expect_dictionary_error(size_t code, const char* action)
{
    CHECK(ZSTD_isError(code), "%s unexpectedly succeeded\n", action);
    CHECK(ZSTD_getErrorCode(code) == ZSTD_error_dictionary_wrong ||
              ZSTD_getErrorCode(code) == ZSTD_error_corruption_detected,
          "%s returned %s instead of a dictionary-related error\n",
          action,
          ZSTD_getErrorName(code));
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

static int decompress_with_prefix_exact(const void* compressed,
                                        size_t compressed_size,
                                        const void* prefix,
                                        size_t prefix_size,
                                        unsigned char* decoded,
                                        size_t decoded_capacity,
                                        const unsigned char* expected,
                                        size_t expected_size)
{
    ZSTD_DCtx* const dctx = ZSTD_createDCtx();
    size_t decoded_size;

    CHECK(dctx != NULL, "failed to create dctx for prefix decode\n");
    CHECK_Z(ZSTD_DCtx_refPrefix(dctx, prefix, prefix_size));
    decoded_size = ZSTD_decompressDCtx(dctx, decoded, decoded_capacity, compressed, compressed_size);
    CHECK(!ZSTD_isError(decoded_size), "ZSTD_decompressDCtx(prefix) failed: %s\n",
          ZSTD_getErrorName(decoded_size));
    CHECK(decoded_size == expected_size, "prefixed decoded size mismatch\n");
    CHECK(memcmp(decoded, expected, expected_size) == 0, "prefixed decoded payload mismatch\n");
    ZSTD_freeDCtx(dctx);
    return 0;
}

static size_t wrap_single_block_frame(unsigned char* dst,
                                      size_t dst_capacity,
                                      unsigned block_type,
                                      size_t block_header_size,
                                      size_t decompressed_size,
                                      const unsigned char* block,
                                      size_t block_size)
{
    unsigned const descriptor = (1U << 5) | (3U << 6);
    unsigned const block_header = 1U | (block_type << 1) | ((unsigned)block_header_size << 3);
    uint64_t const content_size = (uint64_t)decompressed_size;

    CHECK(dst_capacity >= 4U + 1U + 8U + 3U + block_size, "wrapped frame buffer too small\n");
    memcpy(dst, "\x28\xB5\x2F\xFD", 4U);
    dst[4] = (unsigned char)descriptor;
    memcpy(dst + 5U, &content_size, 8U);
    memcpy(dst + 13U, &block_header, 3U);
    memcpy(dst + 16U, block, block_size);
    return 16U + block_size;
}

static size_t build_raw_block_frame(unsigned char* dst,
                                    size_t dst_capacity,
                                    const unsigned char* const* chunks,
                                    const size_t* chunk_sizes,
                                    size_t chunk_count)
{
    uint64_t total_size = 0;
    size_t offset = 0;
    size_t i;

    for (i = 0; i < chunk_count; ++i) {
        total_size += chunk_sizes[i];
    }
    CHECK(dst_capacity >= 13U + (chunk_count * 3U) + (size_t)total_size,
          "raw frame buffer too small\n");

    memcpy(dst + offset, "\x28\xB5\x2F\xFD", 4U);
    offset += 4U;
    dst[offset++] = (unsigned char)((1U << 5) | (3U << 6));
    memcpy(dst + offset, &total_size, 8U);
    offset += 8U;

    for (i = 0; i < chunk_count; ++i) {
        unsigned const block_header =
            (unsigned)(i + 1U == chunk_count) | ((unsigned)chunk_sizes[i] << 3);
        memcpy(dst + offset, &block_header, 3U);
        offset += 3U;
        memcpy(dst + offset, chunks[i], chunk_sizes[i]);
        offset += chunk_sizes[i];
    }

    return offset;
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

static unsigned frame_has_checksum(const unsigned char* frame, size_t frame_size)
{
    size_t const header_size = ZSTD_frameHeaderSize(frame, frame_size);
    return header_size >= 5U && frame_size >= header_size && (frame[4] & 0x04U) != 0U;
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

static int emit_legacy_frame(ZSTD_CCtx* cctx,
                             unsigned char* dst,
                             size_t dst_capacity,
                             size_t* written_out,
                             const unsigned char* const* segments,
                             const size_t* segment_sizes,
                             size_t segment_count)
{
    size_t written = 0;
    size_t i;

    for (i = 0; i + 1U < segment_count; ++i) {
        size_t const produced =
            ZSTD_compressContinue(cctx, dst + written, dst_capacity - written,
                                  segments[i], segment_sizes[i]);
        CHECK(!ZSTD_isError(produced), "ZSTD_compressContinue failed: %s\n",
              ZSTD_getErrorName(produced));
        CHECK(produced > 0U || segment_sizes[i] == 0U,
              "ZSTD_compressContinue buffered all output until end\n");
        written += produced;
    }

    {
        size_t const last_index = segment_count ? segment_count - 1U : 0U;
        size_t const produced =
            ZSTD_compressEnd(cctx, dst + written, dst_capacity - written,
                             segments[last_index], segment_sizes[last_index]);
        CHECK(!ZSTD_isError(produced), "ZSTD_compressEnd failed: %s\n",
              ZSTD_getErrorName(produced));
        written += produced;
    }

    *written_out = written;
    return 0;
}

static int test_one_shot_context_and_block(void)
{
    size_t const src_size = (256U << 10) + 19U;
    size_t const compressible_size = (128U << 10) + 17U;
    unsigned char* const src = (unsigned char*)malloc(src_size);
    unsigned char* const compressible_src = (unsigned char*)malloc(compressible_size);
    size_t const bound = ZSTD_compressBound(src_size);
    size_t const compressible_bound = ZSTD_compressBound(compressible_size);
    unsigned char* const compressed = (unsigned char*)malloc(bound);
    unsigned char* const compressible_out = (unsigned char*)malloc(compressible_bound);
    unsigned char* const second = (unsigned char*)malloc(bound);
    unsigned char* const third = (unsigned char*)malloc(bound);
    unsigned char* const decoded = (unsigned char*)malloc(src_size);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    ZSTD_CCtx* const clone = ZSTD_createCCtx();
    ZSTD_CCtx* const copy_src = ZSTD_createCCtx();
    int level = 0;
    size_t size = 0;

    CHECK(src != NULL && compressible_src != NULL && compressed != NULL &&
              compressible_out != NULL && second != NULL && third != NULL &&
              decoded != NULL && cctx != NULL && clone != NULL && copy_src != NULL,
          "allocation failure in one-shot smoke\n");

    generate_sample(src, src_size, 0x12345678U);
    generate_compressible_sample(compressible_src, compressible_size);
    CHECK(bound >= src_size, "compress bound smaller than source size\n");

    size = ZSTD_compress(compressed, bound, src, src_size, 1);
    CHECK(!ZSTD_isError(size), "ZSTD_compress failed: %s\n", ZSTD_getErrorName(size));
    if (decompress_exact(compressed, size, decoded, src_size, src, src_size)) return 1;

    size = ZSTD_compress(compressible_out, compressible_bound,
                         compressible_src, compressible_size, 1);
    CHECK(!ZSTD_isError(size), "ZSTD_compress(compressible) failed: %s\n",
          ZSTD_getErrorName(size));
    CHECK(frame_first_block_type(compressible_out, size) == 2U,
          "one-shot compression did not emit a compressed block for compressible input\n");
    CHECK(size < compressible_size,
          "one-shot compression failed to shrink compressible input\n");
    if (decompress_exact(compressible_out, size, decoded,
                         compressible_size, compressible_src, compressible_size)) {
        return 1;
    }

    CHECK(ZSTD_sizeof_CCtx(cctx) > 0, "expected non-zero CCtx size\n");
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 5));
    CHECK_Z(ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1));
    CHECK_Z(ZSTD_CCtx_getParameter(cctx, ZSTD_c_compressionLevel, &level));
    CHECK(level == 5, "unexpected compression level %d\n", level);
    CHECK_Z(ZSTD_CCtx_setPledgedSrcSize(cctx, src_size));

    size = ZSTD_compress2(cctx, second, bound, src, src_size);
    CHECK(!ZSTD_isError(size), "ZSTD_compress2 failed: %s\n", ZSTD_getErrorName(size));
    if (decompress_exact(second, size, decoded, src_size, src, src_size)) return 1;

    CHECK_Z(ZSTD_compressBegin(copy_src, 1));
    CHECK_Z(ZSTD_copyCCtx(clone, copy_src, ZSTD_CONTENTSIZE_UNKNOWN));

    size = ZSTD_compressCCtx(cctx, third, bound, src, src_size, 3);
    CHECK(!ZSTD_isError(size), "ZSTD_compressCCtx failed: %s\n", ZSTD_getErrorName(size));
    CHECK(!frame_has_checksum(third, size),
          "ZSTD_compressCCtx preserved sticky checksum parameters\n");
    if (decompress_exact(third, size, decoded, src_size, src, src_size)) return 1;

    {
        const unsigned char* segments[] = {
            src,
            src + (96U << 10),
            src + (160U << 10)
        };
        const size_t segment_sizes[] = {
            96U << 10,
            64U << 10,
            src_size - (160U << 10)
        };
        size_t legacy_size = 0;
        CHECK_Z(ZSTD_CCtx_reset(copy_src, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_compressBegin(copy_src, 3));
        if (emit_legacy_frame(copy_src, third, bound, &legacy_size, segments, segment_sizes,
                              sizeof(segments) / sizeof(segments[0]))) {
            return 1;
        }
        if (decompress_exact(third, legacy_size, decoded, src_size, src, src_size)) return 1;
    }

    {
        size_t block_limit;
        unsigned char* block_src = NULL;
        unsigned char* block_compressed = NULL;
        unsigned char* block_frame = NULL;
        unsigned char* noise = NULL;
        unsigned char* too_large = NULL;
        size_t block_size;
        size_t wrapped_size;
        CHECK_Z(ZSTD_compressBegin(cctx, 1));
        block_limit = ZSTD_getBlockSize(cctx);
        CHECK(block_limit == ZSTD_BLOCKSIZE_MAX, "unexpected ordinary block size %zu\n", block_limit);
        block_src = (unsigned char*)malloc(block_limit);
        block_compressed = (unsigned char*)malloc(ZSTD_compressBound(block_limit));
        block_frame = (unsigned char*)malloc(ZSTD_compressBound(block_limit) + 32U);
        CHECK(block_src != NULL && block_compressed != NULL && block_frame != NULL,
              "allocation failure for block smoke\n");
        memset(block_src, 'A', block_limit);
        block_size = ZSTD_compressBlock(cctx,
                                        block_compressed,
                                        ZSTD_compressBound(block_limit),
                                        block_src,
                                        block_limit);
        CHECK(!ZSTD_isError(block_size), "ZSTD_compressBlock failed: %s\n",
              ZSTD_getErrorName(block_size));
        CHECK(block_size > 0, "block compression produced no output\n");
        wrapped_size = wrap_single_block_frame(block_frame,
                                               ZSTD_compressBound(block_limit) + 32U,
                                               1U,
                                               block_limit,
                                               block_limit,
                                               block_compressed,
                                               block_size);
        if (decompress_exact(block_frame, wrapped_size, decoded, block_limit,
                             block_src, block_limit)) {
            return 1;
        }
        too_large = (unsigned char*)malloc(block_limit + 1U);
        CHECK(too_large != NULL, "allocation failure for oversized block smoke\n");
        memset(too_large, 'B', block_limit + 1U);
        block_size = ZSTD_compressBlock(cctx,
                                        block_compressed,
                                        ZSTD_compressBound(block_limit + 1U),
                                        too_large,
                                        block_limit + 1U);
        CHECK(ZSTD_isError(block_size), "oversized ZSTD_compressBlock unexpectedly succeeded\n");
        CHECK(ZSTD_getErrorCode(block_size) == ZSTD_error_srcSize_wrong,
              "oversized ZSTD_compressBlock returned %s\n",
              ZSTD_getErrorName(block_size));
        noise = (unsigned char*)malloc(block_limit);
        CHECK(noise != NULL, "allocation failure for block history smoke\n");
        generate_noise_sample(noise, block_limit, 0xA51C9E77U);
        CHECK_Z(ZSTD_compressBegin(cctx, 1));
        block_size = ZSTD_compressBlock(cctx,
                                        block_compressed,
                                        ZSTD_compressBound(block_limit),
                                        noise,
                                        block_limit);
        CHECK(block_size == 0U,
              "ZSTD_compressBlock unexpectedly produced an incompressible block\n");
        block_size = ZSTD_compressBlock(cctx,
                                        block_compressed,
                                        ZSTD_compressBound(block_limit),
                                        noise,
                                        block_limit);
        CHECK(!ZSTD_isError(block_size), "history-aware ZSTD_compressBlock failed: %s\n",
              ZSTD_getErrorName(block_size));
        CHECK(block_size > 0U, "repeated block did not use prior block history\n");
        free(noise);
        free(too_large);
        free(block_frame);
        free(block_compressed);
        free(block_src);
    }

    {
        ZSTD_parameters const advanced_params = ZSTD_getParams(3, src_size, 0);
        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_compressBegin_advanced(cctx, NULL, 0, advanced_params, src_size + 1U));
        size = ZSTD_compressEnd(cctx, third, bound, src, src_size);
        CHECK(ZSTD_isError(size), "ZSTD_compressEnd unexpectedly accepted pledged-size mismatch\n");
        CHECK(ZSTD_getErrorCode(size) == ZSTD_error_srcSize_wrong,
              "ZSTD_compressEnd returned %s for pledged-size mismatch\n",
              ZSTD_getErrorName(size));
    }

    {
        ZSTD_DCtx* const dctx = ZSTD_createDCtx();
        const unsigned char* raw_chunks[] = {
            src,
            src + 8195U
        };
        const size_t raw_chunk_sizes[] = {
            8195U,
            6149U
        };
        unsigned char* const raw_frame =
            (unsigned char*)malloc(13U + 6U + raw_chunk_sizes[0] + raw_chunk_sizes[1]);
        unsigned char* const inserted =
            (unsigned char*)malloc(raw_chunk_sizes[0] + raw_chunk_sizes[1]);
        size_t raw_frame_size;
        size_t header_size;
        size_t offset = 0;
        size_t inserted_size;

        CHECK(dctx != NULL && raw_frame != NULL && inserted != NULL,
              "allocation failure for insertBlock smoke\n");
        raw_frame_size = build_raw_block_frame(raw_frame,
                                               13U + 6U + raw_chunk_sizes[0] + raw_chunk_sizes[1],
                                               raw_chunks,
                                               raw_chunk_sizes,
                                               2U);
        header_size = ZSTD_frameHeaderSize(raw_frame, raw_frame_size);

        CHECK_Z(ZSTD_decompressBegin(dctx));
        CHECK_Z(ZSTD_decompressContinue(dctx, NULL, 0, raw_frame, 5U));
        offset += 5U;
        CHECK_Z(ZSTD_decompressContinue(dctx, NULL, 0, raw_frame + offset, header_size - offset));
        offset = header_size;
        CHECK_Z(ZSTD_decompressContinue(dctx, NULL, 0, raw_frame + offset, 3U));
        offset += 3U;
        CHECK(ZSTD_nextSrcSizeToDecompress(dctx) == raw_chunk_sizes[0],
              "unexpected insertBlock first body size\n");
        memcpy(inserted, raw_frame + offset, raw_chunk_sizes[0]);
        inserted_size = ZSTD_insertBlock(dctx, raw_frame + offset, raw_chunk_sizes[0]);
        CHECK(!ZSTD_isError(inserted_size), "ZSTD_insertBlock(first) failed: %s\n",
              ZSTD_getErrorName(inserted_size));
        CHECK(inserted_size == raw_chunk_sizes[0], "unexpected first insertBlock size\n");
        offset += raw_chunk_sizes[0];
        CHECK(ZSTD_nextSrcSizeToDecompress(dctx) == 3U,
              "insertBlock did not advance to next header\n");

        CHECK_Z(ZSTD_decompressContinue(dctx, NULL, 0, raw_frame + offset, 3U));
        offset += 3U;
        CHECK(ZSTD_nextSrcSizeToDecompress(dctx) == raw_chunk_sizes[1],
              "unexpected insertBlock second body size\n");
        memcpy(inserted + raw_chunk_sizes[0], raw_frame + offset, raw_chunk_sizes[1]);
        inserted_size = ZSTD_insertBlock(dctx, raw_frame + offset, raw_chunk_sizes[1]);
        CHECK(!ZSTD_isError(inserted_size), "ZSTD_insertBlock(second) failed: %s\n",
              ZSTD_getErrorName(inserted_size));
        CHECK(inserted_size == raw_chunk_sizes[1], "unexpected second insertBlock size\n");
        offset += raw_chunk_sizes[1];
        CHECK(offset == raw_frame_size, "raw frame offset mismatch\n");
        CHECK(ZSTD_nextSrcSizeToDecompress(dctx) == 0U,
              "insertBlock did not finish raw frame\n");
        CHECK(memcmp(inserted, src, raw_chunk_sizes[0]) == 0, "first inserted block mismatch\n");
        CHECK(memcmp(inserted + raw_chunk_sizes[0], src + 8195U, raw_chunk_sizes[1]) == 0,
              "second inserted block mismatch\n");

        ZSTD_freeDCtx(dctx);
        free(inserted);
        free(raw_frame);
    }

    ZSTD_freeCCtx(copy_src);
    ZSTD_freeCCtx(clone);
    ZSTD_freeCCtx(cctx);
    free(decoded);
    free(compressible_out);
    free(compressible_src);
    free(third);
    free(second);
    free(compressed);
    free(src);
    return 0;
}

static int test_dictionary_and_prefix(const char* dict_path)
{
    unsigned char* dict = NULL;
    size_t dict_size = 0;
    unsigned dict_id = 0;
    size_t const src_size = (64U << 10) + 131U;
    unsigned char* src = NULL;
    unsigned char* decoded = NULL;
    unsigned char* compressed = NULL;
    unsigned char* second = NULL;
    size_t compressed_capacity = 0;
    ZSTD_CCtx* cctx = NULL;
    ZSTD_DCtx* dctx = NULL;
    ZSTD_CDict* cdict = NULL;
    size_t compressed_size = 0;

    if (read_file(dict_path, &dict, &dict_size)) return 1;
    dict_id = ZSTD_getDictID_fromDict(dict, dict_size);
    CHECK(dict_id != 0U, "expected formatted dictionary fixture\n");

    src = (unsigned char*)malloc(src_size);
    decoded = (unsigned char*)malloc(src_size * 2U);
    compressed_capacity = ZSTD_compressBound(src_size);
    compressed = (unsigned char*)malloc(compressed_capacity);
    second = (unsigned char*)malloc(compressed_capacity);
    cctx = ZSTD_createCCtx();
    dctx = ZSTD_createDCtx();
    cdict = ZSTD_createCDict(dict, dict_size, 5);
    CHECK(src != NULL && decoded != NULL && compressed != NULL && second != NULL &&
              cctx != NULL && dctx != NULL && cdict != NULL,
          "allocation failure in dictionary smoke\n");

    generate_dict_biased_sample(src, src_size, dict, dict_size, 0x12345U);
    CHECK(ZSTD_getDictID_fromCDict(cdict) == dict_id, "CDict ID mismatch\n");
    CHECK(ZSTD_sizeof_CDict(cdict) > 0, "expected non-zero CDict size\n");

    compressed_size = ZSTD_compress_usingCDict(cctx, compressed, compressed_capacity,
                                               src, src_size, cdict);
    CHECK(!ZSTD_isError(compressed_size), "ZSTD_compress_usingCDict failed: %s\n",
          ZSTD_getErrorName(compressed_size));
    CHECK(compressed_size < src_size,
          "ZSTD_compress_usingCDict failed to shrink the source\n");
    CHECK(frame_first_block_type(compressed, compressed_size) == 2U,
          "CDict compression did not emit a compressed block\n");
    CHECK(ZSTD_getDictID_fromFrame(compressed, compressed_size) == dict_id,
          "frame dictionary ID mismatch\n");
    if (expect_dictionary_error(ZSTD_decompress(decoded, src_size, compressed, compressed_size),
                                "ZSTD_decompress without dictionary")) {
        return 1;
    }

    if (decompress_using_dict_exact(compressed, compressed_size, dict, dict_size,
                                    decoded, src_size, src, src_size)) {
        return 1;
    }

    compressed_size = ZSTD_compress_usingDict(cctx, second, compressed_capacity,
                                              src, src_size, dict, dict_size, 5);
    CHECK(!ZSTD_isError(compressed_size), "ZSTD_compress_usingDict failed: %s\n",
          ZSTD_getErrorName(compressed_size));
    CHECK(compressed_size < src_size,
          "ZSTD_compress_usingDict failed to shrink the source\n");
    CHECK(frame_first_block_type(second, compressed_size) == 2U,
          "usingDict compression did not emit a compressed block\n");
    if (decompress_using_dict_exact(second, compressed_size, dict, dict_size,
                                    decoded, src_size, src, src_size)) {
        return 1;
    }

    {
        ZSTD_CDict* const by_copy = ZSTD_createCDict_advanced(
            dict, dict_size,
            ZSTD_dlm_byCopy, ZSTD_dct_fullDict,
            ZSTD_getCParams(5, ZSTD_CONTENTSIZE_UNKNOWN, dict_size),
            (ZSTD_customMem){ 0 });
        ZSTD_CDict* const by_ref_raw = ZSTD_createCDict_advanced(
            dict, dict_size,
            ZSTD_dlm_byRef, ZSTD_dct_rawContent,
            ZSTD_getCParams(5, ZSTD_CONTENTSIZE_UNKNOWN, dict_size),
            (ZSTD_customMem){ 0 });
        CHECK(by_copy != NULL && by_ref_raw != NULL,
              "advanced CDict creation failed\n");
        CHECK(ZSTD_sizeof_CDict(by_ref_raw) < ZSTD_sizeof_CDict(by_copy),
              "dictLoadMethod did not affect CDict ownership size\n");

        compressed_size = ZSTD_compress_usingCDict(cctx, compressed, compressed_capacity,
                                                   src, src_size, by_ref_raw);
        CHECK(!ZSTD_isError(compressed_size),
              "ZSTD_compress_usingCDict(raw advanced) failed: %s\n",
              ZSTD_getErrorName(compressed_size));
        CHECK(ZSTD_getDictID_fromFrame(compressed, compressed_size) == 0U,
              "raw-content CDict unexpectedly preserved dict ID\n");

        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_CCtx_loadDictionary_advanced(cctx, dict, dict_size,
                                                  ZSTD_dlm_byRef, ZSTD_dct_rawContent));
        compressed_size = ZSTD_compress2(cctx, second, compressed_capacity, src, src_size);
        CHECK(!ZSTD_isError(compressed_size),
              "ZSTD_compress2(loadDictionary_advanced raw) failed: %s\n",
              ZSTD_getErrorName(compressed_size));
        CHECK(ZSTD_getDictID_fromFrame(second, compressed_size) == 0U,
              "raw-content loadDictionary_advanced unexpectedly preserved dict ID\n");

        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_CCtx_loadDictionary_advanced(cctx, dict, dict_size,
                                                  ZSTD_dlm_byCopy, ZSTD_dct_fullDict));
        compressed_size = ZSTD_compress2(cctx, second, compressed_capacity, src, src_size);
        CHECK(!ZSTD_isError(compressed_size),
              "ZSTD_compress2(loadDictionary_advanced full) failed: %s\n",
              ZSTD_getErrorName(compressed_size));
        CHECK(ZSTD_getDictID_fromFrame(second, compressed_size) == dict_id,
              "full-dict loadDictionary_advanced lost dict ID\n");

        ZSTD_freeCDict(by_ref_raw);
        ZSTD_freeCDict(by_copy);
    }

    {
        const unsigned char* segments[] = {
            src,
            src + (24U << 10)
        };
        const size_t segment_sizes[] = {
            24U << 10,
            src_size - (24U << 10)
        };
        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_compressBegin_usingDict(cctx, dict, dict_size, 5));
        if (emit_legacy_frame(cctx, second, compressed_capacity, &compressed_size, segments,
                              segment_sizes, sizeof(segments) / sizeof(segments[0]))) {
            return 1;
        }
        CHECK(frame_first_block_type(second, compressed_size) == 2U,
              "legacy usingDict compression did not emit a compressed block\n");
        if (decompress_using_dict_exact(second, compressed_size, dict, dict_size,
                                        decoded, src_size, src, src_size)) {
            return 1;
        }
    }

    {
        size_t const prefix_size = 12U << 10;
        unsigned char* const prefix = (unsigned char*)malloc(prefix_size);
        unsigned char* const prefix_src = (unsigned char*)malloc(prefix_size * 3U);
        unsigned char* const plain = (unsigned char*)malloc(ZSTD_compressBound(prefix_size * 3U));
        unsigned char* const prefixed = (unsigned char*)malloc(ZSTD_compressBound(prefix_size * 3U));
        unsigned char* const prefix_decoded = (unsigned char*)malloc(prefix_size * 3U);
        size_t plain_size;
        size_t prefixed_size;

        CHECK(prefix != NULL && prefix_src != NULL && plain != NULL &&
                  prefixed != NULL && prefix_decoded != NULL,
              "allocation failure for prefix smoke\n");
        generate_sample(prefix, prefix_size, 0x55AA7711U);
        memcpy(prefix_src, prefix, prefix_size);
        memcpy(prefix_src + prefix_size, prefix, prefix_size);
        memcpy(prefix_src + (prefix_size * 2U), prefix, prefix_size);

        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        plain_size = ZSTD_compress2(cctx, plain, ZSTD_compressBound(prefix_size * 3U),
                                    prefix_src, prefix_size * 3U);
        CHECK(!ZSTD_isError(plain_size), "plain ZSTD_compress2 failed: %s\n",
              ZSTD_getErrorName(plain_size));

        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_CCtx_refPrefix(cctx, prefix, prefix_size));
        prefixed_size = ZSTD_compress2(cctx, prefixed, ZSTD_compressBound(prefix_size * 3U),
                                       prefix_src, prefix_size * 3U);
        CHECK(!ZSTD_isError(prefixed_size), "prefixed ZSTD_compress2 failed: %s\n",
              ZSTD_getErrorName(prefixed_size));
        CHECK(prefixed_size < plain_size, "prefix reference did not improve size\n");
        CHECK(frame_first_block_type(prefixed, prefixed_size) == 2U,
              "prefix compression did not emit a compressed block\n");
        if (decompress_with_prefix_exact(prefixed, prefixed_size,
                                         prefix, prefix_size,
                                         prefix_decoded, prefix_size * 3U,
                                         prefix_src, prefix_size * 3U)) {
            return 1;
        }

        CHECK_Z(ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_CCtx_refPrefix(cctx, prefix, prefix_size));
        CHECK_Z(ZSTD_CCtx_refPrefix(cctx, NULL, 0));
        compressed_size = ZSTD_compress2(cctx, prefixed, ZSTD_compressBound(prefix_size * 3U),
                                         prefix_src, prefix_size * 3U);
        CHECK(!ZSTD_isError(compressed_size), "cleared-prefix ZSTD_compress2 failed: %s\n",
              ZSTD_getErrorName(compressed_size));
        CHECK(compressed_size == plain_size, "cleared prefix did not restore plain size\n");
        CHECK(memcmp(prefixed, plain, plain_size) == 0,
              "cleared prefix did not restore plain encoding\n");

        free(prefix_decoded);
        free(prefixed);
        free(plain);
        free(prefix_src);
        free(prefix);
    }

    ZSTD_freeCDict(cdict);
    ZSTD_freeDCtx(dctx);
    ZSTD_freeCCtx(cctx);
    free(second);
    free(compressed);
    free(decoded);
    free(src);
    free(dict);
    return 0;
}

int main(int argc, char** argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: %s DICTIONARY\n", argv[0]);
        return 1;
    }

    if (test_one_shot_context_and_block()) return 1;
    if (test_dictionary_and_prefix(argv[1])) return 1;
    return 0;
}
