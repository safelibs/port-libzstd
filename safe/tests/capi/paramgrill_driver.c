#define ZSTD_STATIC_LINKING_ONLY
#include "zstd.h"
#include <stdio.h>
#include <stdlib.h>

#define main upstream_paramgrill_main
#include "../../../original/libzstd-1.5.5+dfsg2/tests/paramgrill.c"
#undef main

static void check_zstd(size_t code, const char* label)
{
    if (ZSTD_isError(code)) {
        fprintf(stderr, "%s: %s\n", label, ZSTD_getErrorName(code));
        exit(1);
    }
}

static void verify_negative_compression_levels(void)
{
    ZSTD_bounds const level_bounds = ZSTD_cParam_getBounds(ZSTD_c_compressionLevel);
    ZSTD_bounds const window_bounds = ZSTD_cParam_getBounds(ZSTD_c_windowLog);
    ZSTD_bounds const checksum_bounds = ZSTD_cParam_getBounds(ZSTD_c_checksumFlag);
    ZSTD_bounds const target_bounds = ZSTD_cParam_getBounds(ZSTD_c_targetLength);
    ZSTD_compressionParameters const negative = ZSTD_getCParams(-5, 0, 0);
    ZSTD_compressionParameters const negative_dict = ZSTD_getCParams(-5, 0, 1);
    ZSTD_compressionParameters const level_one = ZSTD_getCParams(1, 0, 0);
    ZSTD_compressionParameters valid = ZSTD_getCParams(3, 0, 0);
    ZSTD_CCtx* const cctx = ZSTD_createCCtx();
    int current_level = 0;
    int current_checksum = 0;

    if (level_bounds.error != 0 || level_bounds.lowerBound != ZSTD_minCLevel() || level_bounds.lowerBound >= 0) {
        fprintf(stderr, "negative compression-level bounds drifted\n");
        exit(1);
    }
    if (level_bounds.upperBound != ZSTD_maxCLevel()) {
        fprintf(stderr, "compression-level upper bound drifted\n");
        exit(1);
    }
    if (window_bounds.error != 0 || window_bounds.lowerBound != 10 || window_bounds.upperBound != 31) {
        fprintf(stderr, "windowLog bounds drifted\n");
        exit(1);
    }
    if (checksum_bounds.error != 0 || checksum_bounds.lowerBound != 0 || checksum_bounds.upperBound != 1) {
        fprintf(stderr, "checksumFlag bounds drifted\n");
        exit(1);
    }
    valid.windowLog = 0;
    if (!ZSTD_isError(ZSTD_checkCParams(valid))) {
        fprintf(stderr, "ZSTD_checkCParams accepted zero windowLog\n");
        exit(1);
    }
    valid = ZSTD_getCParams(3, 0, 0);
    valid.windowLog = (unsigned)(window_bounds.lowerBound - 1);
    if (!ZSTD_isError(ZSTD_checkCParams(valid))) {
        fprintf(stderr, "ZSTD_checkCParams accepted below-minimum windowLog\n");
        exit(1);
    }
    if (target_bounds.error != 0 || target_bounds.upperBound != (int)ZSTD_BLOCKSIZE_MAX) {
        fprintf(stderr, "targetLength upper bound drifted\n");
        exit(1);
    }
    if (negative.strategy != ZSTD_fast
            || negative.targetLength != 5
            || negative.windowLog != 19
            || negative.chainLog != 12
            || negative.hashLog != 13
            || negative.searchLog != 1
            || negative.minMatch != 6) {
        fprintf(stderr, "negative compression-level cparams drifted\n");
        exit(1);
    }
    if (negative.targetLength == level_one.targetLength || negative.hashLog == level_one.hashLog) {
        fprintf(stderr, "negative compression level collapsed to level 1\n");
        exit(1);
    }
    if (negative_dict.windowLog == negative.windowLog) {
        fprintf(stderr, "dictionary size no longer influences ZSTD_getCParams\n");
        exit(1);
    }
    if (cctx == NULL) {
        fprintf(stderr, "ZSTD_createCCtx failed\n");
        exit(1);
    }
    check_zstd(
        ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, 14),
        "ZSTD_CCtx_setParameter(windowLog tuned)"
    );
    check_zstd(
        ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, 0),
        "ZSTD_CCtx_setParameter(windowLog default reset)"
    );
    check_zstd(
        ZSTD_CCtx_setParameter(cctx, ZSTD_c_checksumFlag, 2),
        "ZSTD_CCtx_setParameter(checksumFlag truthy)"
    );
    check_zstd(
        ZSTD_CCtx_getParameter(cctx, ZSTD_c_checksumFlag, &current_checksum),
        "ZSTD_CCtx_getParameter(checksumFlag truthy)"
    );
    if (current_checksum != 1) {
        fprintf(stderr, "truthy checksumFlag did not normalize to 1\n");
        exit(1);
    }
    check_zstd(
        ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, ZSTD_maxCLevel() + 1000),
        "ZSTD_CCtx_setParameter(compressionLevel high clamp)"
    );
    check_zstd(
        ZSTD_CCtx_getParameter(cctx, ZSTD_c_compressionLevel, &current_level),
        "ZSTD_CCtx_getParameter(compressionLevel high clamp)"
    );
    if (current_level != ZSTD_maxCLevel()) {
        fprintf(stderr, "high compression level did not clamp to max\n");
        exit(1);
    }
    check_zstd(
        ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, ZSTD_minCLevel() - 1000),
        "ZSTD_CCtx_setParameter(compressionLevel low clamp)"
    );
    check_zstd(
        ZSTD_CCtx_getParameter(cctx, ZSTD_c_compressionLevel, &current_level),
        "ZSTD_CCtx_getParameter(compressionLevel low clamp)"
    );
    if (current_level != ZSTD_minCLevel()) {
        fprintf(stderr, "low compression level did not clamp to min\n");
        exit(1);
    }
    check_zstd(
        ZSTD_CCtx_setParameter(cctx, ZSTD_c_compressionLevel, -5),
        "ZSTD_CCtx_setParameter(negative level)"
    );
    check_zstd(
        ZSTD_CCtx_getParameter(cctx, ZSTD_c_compressionLevel, &current_level),
        "ZSTD_CCtx_getParameter(negative level)"
    );
    if (current_level != -5) {
        fprintf(stderr, "negative compression level was not preserved\n");
        exit(1);
    }
    ZSTD_freeCCtx(cctx);
}

static void verify_estimate_helpers(void)
{
    ZSTD_compressionParameters const small = ZSTD_getCParams(1, 16 * 1024, 0);
    ZSTD_compressionParameters const large = ZSTD_getCParams(19, ZSTD_CONTENTSIZE_UNKNOWN, 0);
    ZSTD_CCtx_params* const params = ZSTD_createCCtxParams();
    size_t const cctx_level_one = ZSTD_estimateCCtxSize(1);
    size_t const cctx_level_nineteen = ZSTD_estimateCCtxSize(19);
    size_t const cctx_small = ZSTD_estimateCCtxSize_usingCParams(small);
    size_t const cctx_large = ZSTD_estimateCCtxSize_usingCParams(large);
    size_t const cstream_level_one = ZSTD_estimateCStreamSize(1);
    size_t const cstream_level_nineteen = ZSTD_estimateCStreamSize(19);
    size_t cctx_params_default;
    size_t cctx_params_window;
    size_t cstream_params_default;
    size_t cstream_params_window;
    int current_checksum = 0;

    if (params == NULL) {
        fprintf(stderr, "ZSTD_createCCtxParams failed\n");
        exit(1);
    }
    if (cctx_level_nineteen <= cctx_level_one) {
        fprintf(stderr, "ZSTD_estimateCCtxSize is not level-sensitive\n");
        exit(1);
    }
    if (cctx_large <= cctx_small) {
        fprintf(stderr, "ZSTD_estimateCCtxSize_usingCParams ignored cParams\n");
        exit(1);
    }
    if (cstream_level_nineteen <= cstream_level_one) {
        fprintf(stderr, "ZSTD_estimateCStreamSize is not level-sensitive\n");
        exit(1);
    }
    if (cstream_level_nineteen <= cctx_level_nineteen) {
        fprintf(stderr, "ZSTD_estimateCStreamSize did not include stream buffers\n");
        exit(1);
    }

    check_zstd(ZSTD_CCtxParams_init(params, 3), "ZSTD_CCtxParams_init(estimate)");
    cctx_params_default = ZSTD_estimateCCtxSize_usingCCtxParams(params);
    cstream_params_default = ZSTD_estimateCStreamSize_usingCCtxParams(params);
    check_zstd(cctx_params_default, "ZSTD_estimateCCtxSize_usingCCtxParams(default)");
    check_zstd(cstream_params_default, "ZSTD_estimateCStreamSize_usingCCtxParams(default)");
    check_zstd(
        ZSTD_CCtxParams_setParameter(params, ZSTD_c_windowLog, 14),
        "ZSTD_CCtxParams_setParameter(windowLog tuned)"
    );
    check_zstd(
        ZSTD_CCtxParams_setParameter(params, ZSTD_c_windowLog, 0),
        "ZSTD_CCtxParams_setParameter(windowLog default reset)"
    );
    check_zstd(
        ZSTD_CCtxParams_setParameter(params, ZSTD_c_hashLog, 20),
        "ZSTD_CCtxParams_setParameter(hashLog estimate)"
    );
    check_zstd(
        ZSTD_CCtxParams_setParameter(params, ZSTD_c_checksumFlag, 2),
        "ZSTD_CCtxParams_setParameter(checksumFlag truthy)"
    );
    check_zstd(
        ZSTD_CCtxParams_getParameter(params, ZSTD_c_checksumFlag, &current_checksum),
        "ZSTD_CCtxParams_getParameter(checksumFlag truthy)"
    );
    if (current_checksum != 1) {
        fprintf(stderr, "truthy CCtxParams checksumFlag did not normalize to 1\n");
        exit(1);
    }
    cctx_params_window = ZSTD_estimateCCtxSize_usingCCtxParams(params);
    cstream_params_window = ZSTD_estimateCStreamSize_usingCCtxParams(params);
    check_zstd(cctx_params_window, "ZSTD_estimateCCtxSize_usingCCtxParams(hashLog)");
    check_zstd(cstream_params_window, "ZSTD_estimateCStreamSize_usingCCtxParams(hashLog)");
    if (cctx_params_window <= cctx_params_default) {
        fprintf(stderr, "ZSTD_estimateCCtxSize_usingCCtxParams ignored hashLog\n");
        exit(1);
    }
    if (cstream_params_window <= cstream_params_default) {
        fprintf(stderr, "ZSTD_estimateCStreamSize_usingCCtxParams ignored hashLog\n");
        exit(1);
    }

    check_zstd(
        ZSTD_CCtxParams_setParameter(params, ZSTD_c_nbWorkers, 1),
        "ZSTD_CCtxParams_setParameter(nbWorkers estimate)"
    );
    if (!ZSTD_isError(ZSTD_estimateCCtxSize_usingCCtxParams(params))) {
        fprintf(stderr, "ZSTD_estimateCCtxSize_usingCCtxParams accepted nbWorkers >= 1\n");
        exit(1);
    }
    if (!ZSTD_isError(ZSTD_estimateCStreamSize_usingCCtxParams(params))) {
        fprintf(stderr, "ZSTD_estimateCStreamSize_usingCCtxParams accepted nbWorkers >= 1\n");
        exit(1);
    }

    ZSTD_freeCCtxParams(params);
}

int main(void)
{
    verify_negative_compression_levels();
    verify_estimate_helpers();

    const char* args[] = {
        "paramgrill",
        "-i1",
        "-s64K"
    };

    return upstream_paramgrill_main((int)(sizeof(args) / sizeof(args[0])), args);
}
