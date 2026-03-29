#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    size_t code = 0;

    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 15);
    code ^= ZSTD_CCtx_setPledgedSrcSize(cctx, 1ULL << 20);
    code ^= ZSTD_compress2(cctx, NULL, 0, NULL, 0);
    code ^= ZSTD_freeCCtx(cctx);

    return (int)(code == (size_t)-1);
}
