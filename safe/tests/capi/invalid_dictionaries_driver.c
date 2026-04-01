#include <stdio.h>
#include <string.h>

#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include "zstd_errors.h"

#define CHECK(cond, ...)                                                      \
    do {                                                                      \
        if (!(cond)) {                                                        \
            fprintf(stderr, __VA_ARGS__);                                     \
            return 1;                                                         \
        }                                                                     \
    } while (0)

#define CHECK_Z(call)                                                         \
    do {                                                                      \
        size_t const check_z_result = (call);                                 \
        if (ZSTD_isError(check_z_result)) {                                   \
            fprintf(stderr, "%s: %s\n", #call, ZSTD_getErrorName(check_z_result)); \
            return 1;                                                         \
        }                                                                     \
    } while (0)

int main(void)
{
    static const unsigned char invalid_repcode_dict[] = {
        0x37, 0xa4, 0x30, 0xec, 0x2a, 0x00, 0x00, 0x00, 0x39, 0x10, 0xc0, 0xc2,
        0xa6, 0x00, 0x0c, 0x30, 0xc0, 0x00, 0x03, 0x0c, 0x30, 0x20, 0x72, 0xf8,
        0xb4, 0x6d, 0x4b, 0x9f, 0xfc, 0x97, 0x29, 0x49, 0xb2, 0xdf, 0x4b, 0x29,
        0x7d, 0x4a, 0xfc, 0x83, 0x18, 0x22, 0x75, 0x23, 0x24, 0x44, 0x4d, 0x02,
        0xb7, 0x97, 0x96, 0xf6, 0xcb, 0xd1, 0xcf, 0xe8, 0x22, 0xea, 0x27, 0x36,
        0xb7, 0x2c, 0x40, 0x46, 0x01, 0x08, 0x23, 0x01, 0x00, 0x00, 0x06, 0x1e,
        0x3c, 0x83, 0x81, 0xd6, 0x18, 0xd4, 0x12, 0x3a, 0x04, 0x00, 0x80, 0x03,
        0x08, 0x0e, 0x12, 0x1c, 0x12, 0x11, 0x0d, 0x0e, 0x0a, 0x0b, 0x0a, 0x09,
        0x10, 0x0c, 0x09, 0x05, 0x04, 0x03, 0x06, 0x06, 0x06, 0x02, 0x00, 0x03,
        0x00, 0x00, 0x02, 0x02, 0x00, 0x04, 0x06, 0x03, 0x06, 0x08, 0x24, 0x6b,
        0x0d, 0x01, 0x10, 0x04, 0x81, 0x07, 0x00, 0x00, 0x04, 0xb9, 0x58, 0x18,
        0x06, 0x59, 0x92, 0x43, 0xce, 0x28, 0xa5, 0x08, 0x88, 0xc0, 0x80, 0x88,
        0x8c, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x00, 0x00
    };
    ZSTD_CDict* const cdict =
        ZSTD_createCDict(invalid_repcode_dict, sizeof(invalid_repcode_dict), 1);
    ZSTD_CDict* const cdict_advanced =
        ZSTD_createCDict_advanced(invalid_repcode_dict,
                                  sizeof(invalid_repcode_dict),
                                  ZSTD_dlm_byCopy,
                                  ZSTD_dct_fullDict,
                                  ZSTD_getCParams(1, ZSTD_CONTENTSIZE_UNKNOWN,
                                                  sizeof(invalid_repcode_dict)),
                                  (ZSTD_customMem){ 0 });
    ZSTD_CCtx_params* const params = ZSTD_createCCtxParams();
    ZSTD_CDict* cdict_advanced2 = NULL;

    CHECK(params != NULL, "failed to allocate CCtx params\n");
    CHECK_Z(ZSTD_CCtxParams_init(params, 1));
    cdict_advanced2 = ZSTD_createCDict_advanced2(invalid_repcode_dict,
                                                 sizeof(invalid_repcode_dict),
                                                 ZSTD_dlm_byCopy,
                                                 ZSTD_dct_fullDict,
                                                 params,
                                                 (ZSTD_customMem){ 0 });

    CHECK(cdict == NULL, "invalid dictionary unexpectedly created a CDict\n");
    CHECK(cdict_advanced == NULL,
          "invalid dictionary unexpectedly created an advanced CDict\n");
    CHECK(cdict_advanced2 == NULL,
          "invalid dictionary unexpectedly created an advanced2 CDict\n");

    ZSTD_freeCCtxParams(params);
    return 0;
}
