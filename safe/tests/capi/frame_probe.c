#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include "zstd_errors.h"

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

static void* read_file(const char* path, size_t* size_out)
{
    FILE* file = fopen(path, "rb");
    long size;
    void* buffer;
    if (file == NULL) die("failed to open probe input");
    if (fseek(file, 0, SEEK_END) != 0) die("failed to seek probe input");
    size = ftell(file);
    if (size < 0) die("failed to stat probe input");
    if (fseek(file, 0, SEEK_SET) != 0) die("failed to rewind probe input");
    buffer = malloc((size_t)size);
    if (buffer == NULL) die("failed to allocate probe buffer");
    if (fread(buffer, 1, (size_t)size, file) != (size_t)size) die("failed to read probe input");
    fclose(file);
    *size_out = (size_t)size;
    return buffer;
}

static void probe_frame(const void* compressed, size_t compressed_size, unsigned long long expected_size)
{
    ZSTD_frameHeader header;
    size_t header_size = ZSTD_frameHeaderSize(compressed, compressed_size);
    unsigned long long content_size = ZSTD_getFrameContentSize(compressed, compressed_size);
    unsigned long long deprecated_size = ZSTD_getDecompressedSize(compressed, compressed_size);
    unsigned long long series_size = ZSTD_findDecompressedSize(compressed, compressed_size);
    unsigned long long bound = ZSTD_decompressBound(compressed, compressed_size);
    size_t frame_size = ZSTD_findFrameCompressedSize(compressed, compressed_size);
    size_t margin = ZSTD_decompressionMargin(compressed, compressed_size);

    if (!ZSTD_isFrame(compressed, compressed_size)) die("expected valid zstd frame");
    if (ZSTD_isSkippableFrame(compressed, compressed_size)) die("unexpected skippable frame classification");
    check_zstd(header_size, "ZSTD_frameHeaderSize");
    check_zstd(ZSTD_getFrameHeader(&header, compressed, compressed_size), "ZSTD_getFrameHeader");
    check_zstd(ZSTD_getFrameHeader_advanced(&header, compressed, compressed_size, ZSTD_f_zstd1), "ZSTD_getFrameHeader_advanced");
    check_zstd(frame_size, "ZSTD_findFrameCompressedSize");
    check_zstd(margin, "ZSTD_decompressionMargin");

    if (frame_size != compressed_size) die("frame size probe did not match buffer length");
    if (compressed_size > 1) {
        expect_error_code(
            ZSTD_findFrameCompressedSize(compressed, compressed_size - 1),
            ZSTD_error_srcSize_wrong,
            "ZSTD_findFrameCompressedSize(truncated)");
    }
    if (content_size != expected_size) die("unexpected frame content size");
    if (deprecated_size != expected_size) die("unexpected deprecated content size");
    if (series_size != expected_size) die("unexpected series decompressed size");
    if (bound != expected_size) die("unexpected decompression bound");
    if (header.frameType != ZSTD_frame) die("unexpected frame type");
    if (header.frameContentSize != expected_size) die("unexpected frame header content size");
    if (ZSTD_getDictID_fromFrame(compressed, compressed_size) != 0) die("unexpected frame dict id");
}

static void probe_skippable(void)
{
    const unsigned char payload[] = { 's', 'a', 'f', 'e', '!' };
    unsigned char frame[8 + sizeof(payload)] = {
        0x53, 0x2A, 0x4D, 0x18,
        (unsigned char)sizeof(payload), 0, 0, 0,
        's', 'a', 'f', 'e', '!'
    };
    unsigned char output[sizeof(payload)];
    unsigned variant = 0;
    size_t read;

    if (!ZSTD_isFrame(frame, sizeof(frame))) die("expected skippable frame to count as a frame");
    if (!ZSTD_isSkippableFrame(frame, sizeof(frame))) die("expected skippable frame classification");
    if (ZSTD_findFrameCompressedSize(frame, sizeof(frame)) != sizeof(frame)) die("unexpected skippable frame size");

    read = ZSTD_readSkippableFrame(output, sizeof(output), &variant, frame, sizeof(frame));
    check_zstd(read, "ZSTD_readSkippableFrame");
    if (read != sizeof(payload)) die("unexpected skippable payload size");
    if (variant != 3) die("unexpected skippable magic variant");
    if (memcmp(output, payload, sizeof(payload)) != 0) die("skippable payload mismatch");
}

int main(int argc, char** argv)
{
    size_t modern_size = 0;
    size_t empty_size = 0;
    void* modern;
    void* empty;

    if (argc != 3) {
        fprintf(stderr, "usage: %s MODERN EMPTY\n", argv[0]);
        return 1;
    }

    modern = read_file(argv[1], &modern_size);
    empty = read_file(argv[2], &empty_size);

    probe_frame(modern, modern_size, 1024ULL * 1024ULL);
    probe_frame(empty, empty_size, 0);
    probe_skippable();

    free(modern);
    free(empty);
    return 0;
}
