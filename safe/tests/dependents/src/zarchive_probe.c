#include <stddef.h>

#include <zstd.h>

int main(void)
{
    static const unsigned char input[] = "zarchive-probe";
    unsigned char compressed[256];
    unsigned char output[sizeof(input)];
    ZSTD_CStream* cstream = ZSTD_createCStream();
    ZSTD_DStream* dstream = ZSTD_createDStream();
    ZSTD_inBuffer input_buffer = {input, sizeof(input), 0};
    ZSTD_outBuffer compressed_buffer = {compressed, sizeof(compressed), 0};
    ZSTD_inBuffer compressed_input = {compressed, 0, 0};
    ZSTD_outBuffer output_buffer = {output, sizeof(output), 0};
    size_t code = 0;

    code ^= ZSTD_CStreamInSize();
    code ^= ZSTD_CStreamOutSize();
    code ^= ZSTD_initCStream(cstream, 3);
    code ^= ZSTD_compressStream2(cstream, &compressed_buffer, &input_buffer, ZSTD_e_end);
    compressed_input.size = compressed_buffer.pos;
    code ^= ZSTD_initDStream(dstream);
    code ^= ZSTD_decompressStream(dstream, &output_buffer, &compressed_input);
    code ^= ZSTD_freeDStream(dstream);
    code ^= ZSTD_freeCStream(cstream);

    return (int)(code == (size_t)-1);
}
