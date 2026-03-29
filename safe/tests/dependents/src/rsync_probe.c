#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    size_t code = 0;

    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 3);
    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 1);
    code ^= ZSTD_compressCCtx(cctx, NULL, 0, NULL, 0, 3);
    code ^= ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
    code ^= ZSTD_decompressDCtx(dctx, NULL, 0, NULL, 0);
    code ^= ZSTD_freeDCtx(dctx);
    code ^= ZSTD_freeCCtx(cctx);

    return (int)(code == (size_t)-1);
}
