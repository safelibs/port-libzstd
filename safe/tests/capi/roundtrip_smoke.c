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

static int test_one_shot_context_and_block(void)
{
    size_t const src_size = (256U << 10) + 19U;
    unsigned char* const src = (unsigned char*)malloc(src_size);
    size_t const bound = ZSTD_compressBound(src_size);
    unsigned char* const compressed = (unsigned char*)malloc(bound);
    unsigned char* const second = (unsigned char*)malloc(bound);
    unsigned char* const third = (unsigned char*)malloc(bound);
    unsigned char* const decoded = (unsigned char*)malloc(src_size);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    ZSTD_CCtx* const clone = ZSTD_createCCtx();
    ZSTD_CCtx* const copy_src = ZSTD_createCCtx();
    int level = 0;
    size_t size = 0;

    CHECK(src != NULL && compressed != NULL && second != NULL && third != NULL &&
              decoded != NULL && cctx != NULL && clone != NULL && copy_src != NULL,
          "allocation failure in one-shot smoke\n");

    generate_sample(src, src_size, 0x12345678U);
    CHECK(bound >= src_size, "compress bound smaller than source size\n");

    size = ZSTD_compress(compressed, bound, src, src_size, 1);
    CHECK(!ZSTD_isError(size), "ZSTD_compress failed: %s\n", ZSTD_getErrorName(size));
    if (decompress_exact(compressed, size, decoded, src_size, src, src_size)) return 1;

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
    if (decompress_exact(third, size, decoded, src_size, src, src_size)) return 1;

    {
        unsigned char block_src[32U << 10];
        unsigned char* const block_compressed =
            (unsigned char*)malloc(ZSTD_compressBound(sizeof(block_src)));
        size_t block_size;
        memset(block_src, 'A', sizeof(block_src));
        CHECK(block_compressed != NULL, "allocation failure for block smoke\n");
        CHECK_Z(ZSTD_compressBegin(cctx, 1));
        CHECK(ZSTD_getBlockSize(cctx) >= sizeof(block_src), "unexpected block size\n");
        block_size = ZSTD_compressBlock(cctx,
                                        block_compressed,
                                        ZSTD_compressBound(sizeof(block_src)),
                                        block_src,
                                        sizeof(block_src));
        CHECK(!ZSTD_isError(block_size), "ZSTD_compressBlock failed: %s\n",
              ZSTD_getErrorName(block_size));
        CHECK(block_size > 0, "block compression produced no output\n");
        free(block_compressed);
    }

    ZSTD_freeCCtx(copy_src);
    ZSTD_freeCCtx(clone);
    ZSTD_freeCCtx(cctx);
    free(decoded);
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
    size_t decoded_size = 0;

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
    CHECK(ZSTD_getDictID_fromFrame(compressed, compressed_size) == dict_id,
          "frame dictionary ID mismatch\n");
    if (expect_dictionary_error(ZSTD_decompress(decoded, src_size, compressed, compressed_size),
                                "ZSTD_decompress without dictionary")) {
        return 1;
    }

    decoded_size = ZSTD_decompress_usingDict(dctx, decoded, src_size,
                                             compressed, compressed_size, dict, dict_size);
    CHECK(!ZSTD_isError(decoded_size), "ZSTD_decompress_usingDict failed: %s\n",
          ZSTD_getErrorName(decoded_size));
    CHECK(decoded_size == src_size, "decoded size mismatch for dictionary bytes\n");
    CHECK(memcmp(decoded, src, src_size) == 0, "dictionary-byte round-trip mismatch\n");

    compressed_size = ZSTD_compress_usingDict(cctx, second, compressed_capacity,
                                              src, src_size, dict, dict_size, 5);
    CHECK(!ZSTD_isError(compressed_size), "ZSTD_compress_usingDict failed: %s\n",
          ZSTD_getErrorName(compressed_size));
    decoded_size = ZSTD_decompress_usingDict(dctx, decoded, src_size,
                                             second, compressed_size, dict, dict_size);
    CHECK(!ZSTD_isError(decoded_size), "raw-dict decode failed: %s\n",
          ZSTD_getErrorName(decoded_size));
    CHECK(decoded_size == src_size, "decoded size mismatch for raw dictionary encode\n");
    CHECK(memcmp(decoded, src, src_size) == 0, "raw dictionary round-trip mismatch\n");

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

        CHECK_Z(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters));
        CHECK_Z(ZSTD_DCtx_refPrefix(dctx, prefix, prefix_size));
        decoded_size = ZSTD_decompressDCtx(dctx, prefix_decoded, prefix_size * 3U,
                                           prefixed, prefixed_size);
        CHECK(!ZSTD_isError(decoded_size), "prefixed ZSTD_decompressDCtx failed: %s\n",
              ZSTD_getErrorName(decoded_size));
        CHECK(decoded_size == prefix_size * 3U, "prefix decoded size mismatch\n");
        CHECK(memcmp(prefix_decoded, prefix_src, prefix_size * 3U) == 0,
              "prefix round-trip mismatch\n");

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
