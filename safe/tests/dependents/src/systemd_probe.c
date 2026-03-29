#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    ZSTD_inBuffer input = {NULL, 0, 0};
    ZSTD_outBuffer output = {NULL, 0, 0};
    size_t code = 0;

    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1);
    code ^= ZSTD_compress2(cctx, NULL, 0, NULL, 0);
    code ^= ZSTD_DCtx_setParameter(dctx, ZSTD_d_windowLogMax, 23);
    code ^= ZSTD_decompressStream(dctx, &output, &input);
    code ^= (size_t)ZSTD_getFrameContentSize(NULL, 0);
    code ^= ZSTD_freeDCtx(dctx);
    code ^= ZSTD_freeCCtx(cctx);

    return (int)(code == (size_t)-1);
}
