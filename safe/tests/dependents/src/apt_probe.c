#include <stddef.h>

#include <zstd.h>
#include <zstd_errors.h>

int main(void)
{
    ZSTD_DStream* stream = ZSTD_createDStream();
    ZSTD_inBuffer input = {NULL, 0, 0};
    ZSTD_outBuffer output = {NULL, 0, 0};
    size_t code = 0;

    code ^= ZSTD_initDStream(stream);
    code ^= ZSTD_decompressStream(stream, &output, &input);
    code ^= (size_t)ZSTD_getFrameContentSize(NULL, 0);
    code ^= (size_t)ZSTD_isError(code);
    code ^= ZSTD_freeDStream(stream);

    return (int)(code == (size_t)-1);
}
