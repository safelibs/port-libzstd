#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    ZSTD_inBuffer input = {NULL, 0, 0};
    ZSTD_outBuffer output = {NULL, 0, 0};
    size_t code = 0;

    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 19);
    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_nbWorkers, 0);
    code ^= ZSTD_compressStream2(cctx, &output, &input, ZSTD_e_end);
    code ^= ZSTD_CCtx_reset(cctx, ZSTD_reset_session_only);
    code ^= ZSTD_decompressDCtx(dctx, NULL, 0, NULL, 0);
    code ^= ZSTD_freeDCtx(dctx);
    code ^= ZSTD_freeCCtx(cctx);

    return (int)(code == (size_t)-1);
}
