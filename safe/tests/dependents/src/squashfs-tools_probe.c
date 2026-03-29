#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    size_t code = ZSTD_compressBound(128U << 10);

    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 15);
    code ^= ZSTD_compressCCtx(cctx, NULL, 0, NULL, 0, 15);
    code ^= ZSTD_decompressDCtx(dctx, NULL, 0, NULL, 0);
    code ^= ZSTD_freeDCtx(dctx);
    code ^= ZSTD_freeCCtx(cctx);

    return (int)(code == (size_t)-1);
}
