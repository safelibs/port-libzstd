#include <stddef.h>
#include <stdio.h>

#include "zstd.h"

#define PRINT_LAYOUT(type_name) \
    printf("%s size=%zu align=%zu\n", #type_name, sizeof(type_name), _Alignof(type_name))

#define PRINT_POINTER_LAYOUT(label, pointee_type) \
    printf("%s* size=%zu align=%zu\n", label, sizeof(pointee_type*), _Alignof(pointee_type*))

int main(void) {
    PRINT_LAYOUT(ZSTD_inBuffer);
    PRINT_LAYOUT(ZSTD_outBuffer);
    PRINT_LAYOUT(ZSTD_customMem);
    PRINT_LAYOUT(ZSTD_frameHeader);
    PRINT_LAYOUT(ZSTD_Sequence);
    PRINT_LAYOUT(ZSTD_bounds);
    PRINT_LAYOUT(ZSTD_frameProgression);

    PRINT_POINTER_LAYOUT("ZSTD_CCtx", ZSTD_CCtx);
    PRINT_POINTER_LAYOUT("ZSTD_DCtx", ZSTD_DCtx);
    PRINT_POINTER_LAYOUT("ZSTD_CDict", ZSTD_CDict);
    PRINT_POINTER_LAYOUT("ZSTD_DDict", ZSTD_DDict);
    PRINT_POINTER_LAYOUT("ZSTD_CCtx_params", ZSTD_CCtx_params);
    PRINT_POINTER_LAYOUT("ZSTD_threadPool", ZSTD_threadPool);
    return 0;
}
