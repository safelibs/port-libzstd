#include <stddef.h>

#include <zstd.h>

int main(void)
{
    static const unsigned char input[] = "rpm-probe";
    unsigned char compressed[128];
    unsigned char output[sizeof(input)];
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    size_t code = 0;
    size_t compressed_size = 0;

    code ^= ZSTD_compressBound(sizeof(input));
    code ^= ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, 19);
    compressed_size = ZSTD_compress2(
        cctx,
        compressed,
        sizeof(compressed),
        input,
        sizeof(input)
    );
    code ^= compressed_size;
    code ^= ZSTD_decompressDCtx(
        dctx,
        output,
        sizeof(output),
        compressed,
        compressed_size
    );
    code ^= ZSTD_freeDCtx(dctx);
    code ^= ZSTD_freeCCtx(cctx);

    return (int)(code == (size_t)-1);
}
