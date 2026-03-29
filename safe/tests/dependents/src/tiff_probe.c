#include <stddef.h>

#include <zstd.h>
#include <zstd_errors.h>

int main(void)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    size_t code = ZSTD_compressBound(64U << 10);

    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 9);
    code ^= ZSTD_compressCCtx(cctx, NULL, 0, NULL, 0, 9);
    code ^= ZSTD_decompressDCtx(dctx, NULL, 0, NULL, 0);
    code ^= (size_t)ZSTD_isError(code);
    code ^= ZSTD_freeDCtx(dctx);
    code ^= ZSTD_freeCCtx(cctx);

    return (int)(code == (size_t)-1);
}
