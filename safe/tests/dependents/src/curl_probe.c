#include <stddef.h>

#include <zstd.h>

int main(void)
{
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    ZSTD_inBuffer input = {NULL, 0, 0};
    ZSTD_outBuffer output = {NULL, 0, 0};
    size_t code = 0;

    code ^= ZSTD_DCtx_setParameter(dctx, ZSTD_d_windowLogMax, 23);
    code ^= ZSTD_decompressStream(dctx, &output, &input);
    code ^= ZSTD_DCtx_reset(dctx, ZSTD_reset_session_only);
    code ^= ZSTD_freeDCtx(dctx);

    return (int)(code == (size_t)-1);
}
