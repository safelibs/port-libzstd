#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_CStream* cstream = ZSTD_createCStream();
    ZSTD_DStream* dstream = ZSTD_createDStream();
    ZSTD_inBuffer input = {NULL, 0, 0};
    ZSTD_outBuffer output = {NULL, 0, 0};
    size_t code = 0;

    code ^= (size_t)ZSTD_CStreamInSize();
    code ^= (size_t)ZSTD_CStreamOutSize();
    code ^= ZSTD_compressStream2(cstream, &output, &input, ZSTD_e_end);
    code ^= ZSTD_endStream(cstream, &output);
    code ^= ZSTD_initDStream(dstream);
    code ^= ZSTD_decompressStream(dstream, &output, &input);
    code ^= ZSTD_freeDStream(dstream);
    code ^= ZSTD_freeCStream(cstream);

    return (int)(code == (size_t)-1);
}
