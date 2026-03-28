#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include "zstd_errors.h"

#define main upstream_legacy_main
#include "../../../original/libzstd-1.5.5+dfsg2/tests/legacy.c"
#undef main

int main(void)
{
    size_t size = strlen(EXPECTED);
    char* output = malloc(size);
    size_t ret;

    if (output == NULL) {
        fprintf(stderr, "failed to allocate legacy output buffer\n");
        return 1;
    }

    if (!ZSTD_isFrame(COMPRESSED, 4)) {
        fprintf(stderr, "legacy frame prefix was not recognized\n");
        free(output);
        return 1;
    }

    ret = ZSTD_decompress(output, size, COMPRESSED, COMPRESSED_SIZE);
    if (ZSTD_isError(ret)) {
        fprintf(stderr, "legacy decode failed: %s\n", ZSTD_getErrorName(ret));
        free(output);
        return 1;
    }
    if (ret != size) {
        fprintf(stderr, "legacy decode returned wrong size\n");
        free(output);
        return 1;
    }
    if (memcmp(output, EXPECTED, size) != 0) {
        fprintf(stderr, "legacy decode produced wrong output\n");
        free(output);
        return 1;
    }

    free(output);
    return 0;
}
