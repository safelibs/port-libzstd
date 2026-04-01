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
    if (file == NULL) die("failed to open input");
    if (fseek(file, 0, SEEK_END) != 0) die("failed to seek input");
    size = ftell(file);
    if (size < 0) die("failed to stat input");
    if (fseek(file, 0, SEEK_SET) != 0) die("failed to rewind input");
    buffer = malloc((size_t)size);
    if (buffer == NULL) die("failed to allocate input buffer");
    if (fread(buffer, 1, (size_t)size, file) != (size_t)size) die("failed to read input");
    fclose(file);
    *size_out = (size_t)size;
    return buffer;
}

static void expect_zeroes(const unsigned char* data, size_t size)
{
    size_t i;
    for (i = 0; i < size; ++i) {
        if (data[i] != 0) {
            fprintf(stderr, "expected zero output at offset %zu\n", i);
            exit(1);
        }
    }
}

static void test_one_shot(const unsigned char* compressed, size_t compressed_size, size_t decompressed_size)
{
    unsigned char* output = malloc(decompressed_size);
    size_t result;
    if (output == NULL) die("failed to allocate output buffer");

    result = ZSTD_decompress(output, decompressed_size, compressed, compressed_size);
    check_zstd(result, "ZSTD_decompress");
    if (result != decompressed_size) die("unexpected one-shot decompressed size");
    expect_zeroes(output, result);

    free(output);
}

static void test_context(const unsigned char* compressed, size_t compressed_size, size_t decompressed_size)
{
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    ZSTD_DCtx* clone = ZSTD_createDCtx();
    unsigned char* output = malloc(decompressed_size);
    unsigned char* alt = malloc(decompressed_size);
    size_t result;
    int window_log = 0;
    if (dctx == NULL || clone == NULL) die("failed to create dctx");
    if (output == NULL || alt == NULL) die("failed to allocate context buffers");

    check_zstd(ZSTD_DCtx_setParameter(dctx, ZSTD_d_windowLogMax, 23), "ZSTD_DCtx_setParameter");
    check_zstd(ZSTD_DCtx_getParameter(dctx, ZSTD_d_windowLogMax, &window_log), "ZSTD_DCtx_getParameter");
    if (window_log != 23) die("unexpected dctx parameter value");
    check_zstd(ZSTD_DCtx_setFormat(dctx, ZSTD_f_zstd1), "ZSTD_DCtx_setFormat");
    check_zstd(ZSTD_DCtx_setMaxWindowSize(dctx, 1u << 23), "ZSTD_DCtx_setMaxWindowSize");
    check_zstd(ZSTD_DCtx_refPrefix(dctx, NULL, 0), "ZSTD_DCtx_refPrefix");
    if (ZSTD_sizeof_DCtx(dctx) == 0) die("expected non-zero dctx size");

    ZSTD_copyDCtx(clone, dctx);
    check_zstd(ZSTD_DCtx_getParameter(clone, ZSTD_d_windowLogMax, &window_log), "ZSTD_copyDCtx/ZSTD_DCtx_getParameter");
    result = ZSTD_decompressDCtx(clone, alt, decompressed_size, compressed, compressed_size);
    check_zstd(result, "ZSTD_copyDCtx/ZSTD_decompressDCtx");
    if (result != decompressed_size) die("unexpected cloned dctx decompressed size");
    expect_zeroes(alt, result);

    result = ZSTD_decompressDCtx(dctx, output, decompressed_size, compressed, compressed_size);
    check_zstd(result, "ZSTD_decompressDCtx");
    if (result != decompressed_size) die("unexpected dctx decompressed size");
    expect_zeroes(output, result);

    check_zstd(ZSTD_DCtx_loadDictionary(dctx, NULL, 0), "ZSTD_DCtx_loadDictionary");
    result = ZSTD_decompress_usingDict(dctx, alt, decompressed_size, compressed, compressed_size, NULL, 0);
    check_zstd(result, "ZSTD_decompress_usingDict");
    if (result != decompressed_size) die("unexpected dict decompressed size");
    expect_zeroes(alt, result);

    check_zstd(ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters), "ZSTD_DCtx_reset");

    free(output);
    free(alt);
    ZSTD_freeDCtx(clone);
    ZSTD_freeDCtx(dctx);
}

static void test_streaming(const unsigned char* compressed, size_t compressed_size, size_t decompressed_size)
{
    ZSTD_DStream* zds = ZSTD_createDStream();
    size_t in_chunk = 7;
    size_t out_chunk = 8192;
    unsigned char* output = malloc(decompressed_size);
    size_t produced = 0;
    size_t src_offset = 0;
    if (zds == NULL) die("failed to create dstream");
    if (output == NULL) die("failed to allocate streaming buffer");
    if (ZSTD_DStreamInSize() == 0 || ZSTD_DStreamOutSize() == 0) die("expected recommended stream sizes");
    if (ZSTD_sizeof_DStream(zds) == 0) die("expected non-zero dstream size");

    check_zstd(ZSTD_initDStream(zds), "ZSTD_initDStream");
    while (src_offset < compressed_size || produced < decompressed_size) {
        size_t chunk = compressed_size - src_offset;
        ZSTD_inBuffer input;
        ZSTD_outBuffer output_buf;
        size_t result;

        if (chunk > in_chunk) chunk = in_chunk;
        input.src = compressed + src_offset;
        input.size = chunk;
        input.pos = 0;
        output_buf.dst = output + produced;
        output_buf.size = decompressed_size - produced < out_chunk ? decompressed_size - produced : out_chunk;
        output_buf.pos = 0;

        result = ZSTD_decompressStream(zds, &output_buf, &input);
        check_zstd(result, "ZSTD_decompressStream");
        produced += output_buf.pos;
        src_offset += input.pos;
        if (result == 0 && src_offset == compressed_size && produced == decompressed_size) break;
        if (input.pos == 0 && output_buf.pos == 0) die("streaming decoder made no progress");
    }

    expect_zeroes(output, produced);
    if (produced != decompressed_size) die("unexpected streaming decompressed size");

    check_zstd(ZSTD_resetDStream(zds), "ZSTD_resetDStream");
    check_zstd(ZSTD_initDStream_usingDict(zds, NULL, 0), "ZSTD_initDStream_usingDict");
    check_zstd(ZSTD_initDStream_usingDDict(zds, NULL), "ZSTD_initDStream_usingDDict");

    free(output);
    ZSTD_freeDStream(zds);
}

static void test_bufferless(const unsigned char* compressed, size_t compressed_size, size_t decompressed_size)
{
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    unsigned char* output = malloc(decompressed_size);
    static const unsigned char skippable[] = {
        0x53, 0x2A, 0x4D, 0x18, 0x05, 0x00, 0x00, 0x00, 's', 'a', 'f', 'e', '!'
    };
    size_t offset = 0;
    size_t produced = 0;
    if (dctx == NULL) die("failed to create bufferless dctx");
    if (output == NULL) die("failed to allocate bufferless output");

    check_zstd(ZSTD_decompressBegin(dctx), "ZSTD_decompressBegin");
    while (1) {
        size_t need = ZSTD_nextSrcSizeToDecompress(dctx);
        ZSTD_nextInputType_e next = ZSTD_nextInputType(dctx);
        size_t result;
        if (need == 0) break;
        if (offset + need > compressed_size) die("bufferless decoder requested beyond input");
        result = ZSTD_decompressContinue(dctx, output + produced, decompressed_size - produced, compressed + offset, need);
        check_zstd(result, "ZSTD_decompressContinue");
        produced += result;
        offset += need;
        (void)next;
    }

    if (offset != compressed_size) die("bufferless decoder did not consume full frame");
    if (produced != decompressed_size) die("unexpected bufferless decompressed size");
    expect_zeroes(output, produced);

    check_zstd(ZSTD_decompressBegin_usingDict(dctx, NULL, 0), "ZSTD_decompressBegin_usingDict");
    check_zstd(ZSTD_decompressBegin_usingDDict(dctx, NULL), "ZSTD_decompressBegin_usingDDict");
    check_zstd(ZSTD_decompressBegin(dctx), "ZSTD_decompressBegin(skippable)");
    if (ZSTD_nextSrcSizeToDecompress(dctx) != 5) die("unexpected initial skippable header size");
    if (ZSTD_nextInputType(dctx) != ZSTDnit_frameHeader) die("unexpected initial skippable input type");
    check_zstd(
        ZSTD_decompressContinue(dctx, NULL, 0, skippable, 5),
        "ZSTD_decompressContinue(skippable-prefix)");
    if (ZSTD_nextSrcSizeToDecompress(dctx) != 3) die("unexpected skippable header remainder size");
    if (ZSTD_nextInputType(dctx) != ZSTDnit_skippableFrame) die("expected skippable-frame input type");
    check_zstd(
        ZSTD_decompressContinue(dctx, NULL, 0, skippable + 5, 3),
        "ZSTD_decompressContinue(skippable-header)");
    if (ZSTD_nextSrcSizeToDecompress(dctx) != 5) die("unexpected skippable payload size");
    if (ZSTD_nextInputType(dctx) != ZSTDnit_skippableFrame) die("expected skippable payload input type");
    check_zstd(
        ZSTD_decompressContinue(dctx, NULL, 0, skippable + 8, 5),
        "ZSTD_decompressContinue(skippable-payload)");
    if (ZSTD_nextSrcSizeToDecompress(dctx) != 0) die("expected skippable bufferless completion");

    {
        size_t header_size = ZSTD_frameHeaderSize(compressed, compressed_size);
        size_t block_size;
        size_t offset;
        size_t produced_after_block;
        size_t produced_total;

        check_zstd(ZSTD_decompressBegin(dctx), "ZSTD_decompressBegin(block)");
        check_zstd(
            ZSTD_decompressContinue(dctx, output, decompressed_size, compressed, 5),
            "ZSTD_decompressContinue(block-prefix)");
        check_zstd(
            ZSTD_decompressContinue(
                dctx,
                output,
                decompressed_size,
                compressed + 5,
                header_size - 5),
            "ZSTD_decompressContinue(block-header)");
        check_zstd(
            ZSTD_decompressContinue(
                dctx,
                output,
                decompressed_size,
                compressed + header_size,
                3),
            "ZSTD_decompressContinue(block-header3)");
        block_size = ZSTD_nextSrcSizeToDecompress(dctx);
        offset = header_size + 3;
        produced_after_block = ZSTD_decompressBlock(
            dctx,
            output,
            decompressed_size,
            compressed + offset,
            block_size);
        check_zstd(
            produced_after_block,
            "ZSTD_decompressBlock");
        if (produced_after_block == 0) die("expected ZSTD_decompressBlock to produce output");
        expect_zeroes(output, produced_after_block);
        offset += block_size;
        produced_total = produced_after_block;
        while (ZSTD_nextSrcSizeToDecompress(dctx) != 0) {
            size_t need = ZSTD_nextSrcSizeToDecompress(dctx);
            size_t wrote;
            check_zstd(
                wrote = ZSTD_decompressContinue(
                    dctx,
                    output + produced_total,
                    decompressed_size - produced_total,
                    compressed + offset,
                    need),
                "ZSTD_decompressContinue(block-finish)");
            produced_total += wrote;
            offset += need;
        }
        if (produced_total != decompressed_size) die("unexpected block-assisted decompressed size");
        if (offset != compressed_size) die("block-assisted bufferless decoder did not consume full frame");
        expect_zeroes(output, produced_total);
    }

    free(output);
    ZSTD_freeDCtx(dctx);
}

static void test_corruption(const unsigned char* compressed, size_t compressed_size, size_t decompressed_size)
{
    unsigned char* corrupt = malloc(compressed_size);
    unsigned char* output = malloc(decompressed_size);
    size_t header_size;
    size_t result;

    if (corrupt == NULL || output == NULL) die("failed to allocate corruption buffers");
    memcpy(corrupt, compressed, compressed_size);
    header_size = ZSTD_frameHeaderSize(compressed, compressed_size);
    check_zstd(header_size, "ZSTD_frameHeaderSize(corrupt frame)");
    corrupt[header_size] |= 0x06;

    result = ZSTD_decompress(output, decompressed_size, corrupt, compressed_size);
    expect_error_code(result, ZSTD_error_corruption_detected, "ZSTD_decompress(corrupt frame)");

    free(output);
    free(corrupt);
}

int main(int argc, char** argv)
{
    size_t compressed_size = 0;
    unsigned char* compressed;
    unsigned long long decompressed_size;

    if (argc != 2) {
        fprintf(stderr, "usage: %s FILE\n", argv[0]);
        return 1;
    }

    compressed = read_file(argv[1], &compressed_size);
    decompressed_size = ZSTD_getFrameContentSize(compressed, compressed_size);
    if (decompressed_size == ZSTD_CONTENTSIZE_ERROR || decompressed_size == ZSTD_CONTENTSIZE_UNKNOWN) {
        die("unexpected frame content size");
    }

    test_one_shot(compressed, compressed_size, (size_t)decompressed_size);
    test_context(compressed, compressed_size, (size_t)decompressed_size);
    test_streaming(compressed, compressed_size, (size_t)decompressed_size);
    test_bufferless(compressed, compressed_size, (size_t)decompressed_size);
    test_corruption(compressed, compressed_size, (size_t)decompressed_size);

    free(compressed);
    return 0;
}
