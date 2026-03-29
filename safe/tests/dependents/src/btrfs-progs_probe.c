#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    size_t code = 0;

    code ^= (size_t)ZSTD_getFrameContentSize(NULL, 0);
    code ^= ZSTD_findFrameCompressedSize(NULL, 0);
    code ^= ZSTD_DCtx_setParameter(dctx, ZSTD_d_windowLogMax, 23);
    code ^= ZSTD_decompressDCtx(dctx, NULL, 0, NULL, 0);
    code ^= ZSTD_freeDCtx(dctx);

    return (int)(code == (size_t)-1);
}
