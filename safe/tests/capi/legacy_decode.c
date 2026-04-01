#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include "zstd_errors.h"

#define main upstream_legacy_main
#include "../../../original/libzstd-1.5.5+dfsg2/tests/legacy.c"
#undef main

#define LEGACY05_MAGIC 0xFD2FB525u
#define MODERN08_MAGIC 0xFD2FB528u
#define EXPECTED_REPEATS 5u
#define SUPPORTED_REPEATS 3u

static void die(const char* message)
{
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static void check_zstd(size_t code, const char* what)
{
    if (ZSTD_isError(code)) {
        fprintf(stderr, "%s: %s\n", what, ZSTD_getErrorName(code));
        exit(1);
    }
}

static void expect_error_code(size_t code, ZSTD_ErrorCode expected, const char* what)
{
    if (!ZSTD_isError(code)) {
        fprintf(stderr, "%s unexpectedly succeeded\n", what);
        exit(1);
    }
    if (ZSTD_getErrorCode(code) != expected) {
        fprintf(stderr, "%s: expected %d, got %d (%s)\n",
            what,
            (int)expected,
            (int)ZSTD_getErrorCode(code),
            ZSTD_getErrorName(code));
        exit(1);
    }
}

static const unsigned char* find_magic(const unsigned char* input, size_t input_size, uint32_t magic)
{
    unsigned char marker[4] = {
        (unsigned char)(magic & 0xff),
        (unsigned char)((magic >> 8) & 0xff),
        (unsigned char)((magic >> 16) & 0xff),
        (unsigned char)((magic >> 24) & 0xff)
    };
    size_t offset;

    for (offset = 0; offset + sizeof(marker) <= input_size; ++offset) {
        if (memcmp(input + offset, marker, sizeof(marker)) == 0) {
            return input + offset;
        }
    }

    return NULL;
}

static void supported_fixture(
    const unsigned char** compressed,
    size_t* compressed_size,
    const char** expected,
    size_t* expected_size)
{
    const unsigned char* all_frames = (const unsigned char*)COMPRESSED;
    const unsigned char* start = find_magic(all_frames, COMPRESSED_SIZE, LEGACY05_MAGIC);
    const unsigned char* modern = find_magic(all_frames, COMPRESSED_SIZE, MODERN08_MAGIC);
    size_t single_size = strlen(EXPECTED) / EXPECTED_REPEATS;

    if (start == NULL || modern == NULL || modern <= start) die("failed to locate supported legacy frames");
    if (single_size * EXPECTED_REPEATS != strlen(EXPECTED)) die("unexpected upstream legacy fixture layout");

    *compressed = start;
    *compressed_size = (size_t)(modern - start);
    *expected = EXPECTED + single_size;
    *expected_size = single_size * SUPPORTED_REPEATS;
}

static void expect_output(const char* decoded, const char* expected, size_t size)
{
    if (memcmp(decoded, expected, size) != 0) die("legacy decode produced wrong output");
}

static void test_simple(
    const unsigned char* compressed,
    size_t compressed_size,
    const char* expected,
    size_t expected_size)
{
    char* output = malloc(expected_size);
    size_t ret;

    if (output == NULL) die("failed to allocate legacy output buffer");
    ret = ZSTD_decompress(output, expected_size, compressed, compressed_size);
    check_zstd(ret, "ZSTD_decompress");
    if (ret != expected_size) die("legacy decode returned wrong size");
    expect_output(output, expected, expected_size);
    free(output);
}

static void test_streaming(
    const unsigned char* compressed,
    size_t compressed_size,
    const char* expected,
    size_t expected_size)
{
    ZSTD_DStream* stream = ZSTD_createDStream();
    size_t out_size = ZSTD_DStreamOutSize();
    char* output = malloc(out_size);
    ZSTD_inBuffer input = { compressed, compressed_size, 0 };
    size_t produced = 0;
    int needs_init = 1;

    if (stream == NULL || output == NULL) die("failed to allocate legacy streaming state");

    while (1) {
        ZSTD_outBuffer out = { output, out_size, 0 };
        size_t ret;

        if (needs_init) {
            ret = ZSTD_initDStream(stream);
            check_zstd(ret, "ZSTD_initDStream");
            needs_init = 0;
        }

        ret = ZSTD_decompressStream(stream, &out, &input);
        check_zstd(ret, "ZSTD_decompressStream");
        expect_output(output, expected + produced, out.pos);
        produced += out.pos;
        if (ret == 0) {
            needs_init = 1;
        }
        if (input.pos == input.size && out.pos < out.size) {
            break;
        }
    }

    if (produced != expected_size) die("legacy streaming decode produced wrong size");
    free(output);
    ZSTD_freeDStream(stream);
}

static void test_frame_walk(
    const unsigned char* compressed,
    size_t compressed_size,
    const char* expected,
    size_t expected_size)
{
    const unsigned char* cursor = compressed;
    size_t remaining = compressed_size;
    size_t produced = 0;
    char* output = malloc(expected_size);

    if (output == NULL) die("failed to allocate legacy frame-walk buffer");

    while (remaining != 0) {
        size_t frame_size = ZSTD_findFrameCompressedSize(cursor, remaining);
        size_t decoded;

        check_zstd(frame_size, "ZSTD_findFrameCompressedSize");
        if (frame_size == 0 || frame_size > remaining) die("legacy frame walk returned invalid size");
        if (frame_size > 1) {
            expect_error_code(
                ZSTD_findFrameCompressedSize(cursor, frame_size - 1),
                ZSTD_error_srcSize_wrong,
                "ZSTD_findFrameCompressedSize(truncated legacy)");
        }

        decoded = ZSTD_decompress(output + produced, expected_size - produced, cursor, frame_size);
        check_zstd(decoded, "ZSTD_decompress(frame walk)");
        expect_output(output + produced, expected + produced, decoded);

        produced += decoded;
        cursor += frame_size;
        remaining -= frame_size;
    }

    if (produced != expected_size) die("legacy frame walk produced wrong size");
    free(output);
}

int main(void)
{
    const unsigned char* compressed = NULL;
    const char* expected = NULL;
    size_t compressed_size = 0;
    size_t expected_size = 0;

    supported_fixture(&compressed, &compressed_size, &expected, &expected_size);

    if (!ZSTD_isFrame(compressed, 4)) die("supported legacy frame prefix was not recognized");

    test_simple(compressed, compressed_size, expected, expected_size);
    test_streaming(compressed, compressed_size, expected, expected_size);
    test_frame_walk(compressed, compressed_size, expected, expected_size);

    return 0;
}
