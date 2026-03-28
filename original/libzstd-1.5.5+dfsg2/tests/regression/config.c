/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#include "config.h"

#define PARAM_VALUES(pv) { .data = pv, .size = sizeof(pv) / sizeof((pv)[0]) }

#define FAST_LEVEL(x)                                               \
    static param_value_t const level_fast##x##_param_values[] = {   \
        { .param = ZSTD_c_compressionLevel, .value = -(x) },        \
    };                                                              \
    static config_t const level_fast##x = {                         \
        .name = "level -" #x,                                       \
        .cli_args = "--fast=" #x,                                   \
        .param_values = PARAM_VALUES(level_fast##x##_param_values), \
    };                                                              \
    static config_t const level_fast##x##_dict = {                  \
        .name = "level -" #x " with dict",                          \
        .cli_args = "--fast=" #x,                                   \
        .param_values = PARAM_VALUES(level_fast##x##_param_values), \
        .use_dictionary = 1,                                        \
    };

#define LEVEL(x)                                                    \
    static param_value_t const level_##x##_param_values[] = {       \
        { .param = ZSTD_c_compressionLevel, .value = (x) },         \
    };                                                              \
    static config_t const level_##x = {                             \
        .name = "level " #x,                                        \
        .cli_args = "-" #x,                                         \
        .param_values = PARAM_VALUES(level_##x##_param_values),     \
    };                                                              \
    static config_t const level_##x##_dict = {                      \
        .name = "level " #x " with dict",                           \
        .cli_args = "-" #x,                                         \
        .param_values = PARAM_VALUES(level_##x##_param_values),     \
        .use_dictionary = 1,                                        \
    };

#define ROW_LEVEL(x, y)
#include "levels.h"
#undef ROW_LEVEL
#undef LEVEL
#undef FAST_LEVEL

static config_t const no_pledged_src_size = {
    .name = "no source size",
    .cli_args = "",
    .param_values = PARAM_VALUES(level_0_param_values),
    .no_pledged_src_size = 1,
};

static config_t const no_pledged_src_size_with_dict = {
    .name = "no source size with dict",
    .cli_args = "",
    .param_values = PARAM_VALUES(level_0_param_values),
    .use_dictionary = 1,
    .no_pledged_src_size = 1,
};

static param_value_t const ldm_param_values[] = {
    { .param = ZSTD_c_enableLongDistanceMatching, .value = 1 },
};

static config_t const ldm = {
    .name = "long distance mode",
    .cli_args = "--long",
    .param_values = PARAM_VALUES(ldm_param_values),
};

static param_value_t const mt_param_values[] = {
    { .param = ZSTD_c_nbWorkers, .value = 2 },
};

static config_t const mt = {
    .name = "multithreaded",
    .cli_args = "-T2",
    .param_values = PARAM_VALUES(mt_param_values),
};

static param_value_t const mt_ldm_param_values[] = {
    { .param = ZSTD_c_nbWorkers, .value = 2 },
    { .param = ZSTD_c_enableLongDistanceMatching, .value = 1 },
};

static config_t const mt_ldm = {
    .name = "multithreaded long distance mode",
    .cli_args = "-T2 --long",
    .param_values = PARAM_VALUES(mt_ldm_param_values),
};

static param_value_t const small_wlog_param_values[] = {
    { .param = ZSTD_c_windowLog, .value = 10 },
};

static config_t const small_wlog = {
    .name = "small window log",
    .cli_args = "--zstd=wlog=10",
    .param_values = PARAM_VALUES(small_wlog_param_values),
};

static param_value_t const small_hlog_param_values[] = {
    { .param = ZSTD_c_hashLog, .value = 6 },
    { .param = ZSTD_c_strategy, .value = (int)ZSTD_btopt },
};

static config_t const small_hlog = {
    .name = "small hash log",
    .cli_args = "--zstd=hlog=6,strat=7",
    .param_values = PARAM_VALUES(small_hlog_param_values),
};

static param_value_t const small_clog_param_values[] = {
    { .param = ZSTD_c_chainLog, .value = 6 },
    { .param = ZSTD_c_strategy, .value = (int)ZSTD_btopt },
};

static config_t const small_clog = {
    .name = "small chain log",
    .cli_args = "--zstd=clog=6,strat=7",
    .param_values = PARAM_VALUES(small_clog_param_values),
};

static param_value_t const explicit_params_param_values[] = {
    { .param = ZSTD_c_checksumFlag, .value = 1 },
    { .param = ZSTD_c_contentSizeFlag, .value = 0 },
    { .param = ZSTD_c_dictIDFlag, .value = 0 },
    { .param = ZSTD_c_strategy, .value = (int)ZSTD_greedy },
    { .param = ZSTD_c_windowLog, .value = 18 },
    { .param = ZSTD_c_hashLog, .value = 21 },
    { .param = ZSTD_c_chainLog, .value = 21 },
    { .param = ZSTD_c_targetLength, .value = 100 },
};

static config_t const explicit_params = {
    .name = "explicit params",
    .cli_args = "--no-check --no-dictID --zstd=strategy=3,wlog=18,hlog=21,clog=21,tlen=100",
    .param_values = PARAM_VALUES(explicit_params_param_values),
};

static config_t const* g_configs[] = {
#define FAST_LEVEL(x) &level_fast##x, &level_fast##x##_dict,
#define LEVEL(x) &level_##x, &level_##x##_dict,
#define ROW_LEVEL(x, y)
#include "levels.h"
#undef ROW_LEVEL
#undef LEVEL
#undef FAST_LEVEL
    &no_pledged_src_size,
    &no_pledged_src_size_with_dict,
    &ldm,
    &mt,
    &mt_ldm,
    &small_wlog,
    &small_hlog,
    &small_clog,
    &explicit_params,
    NULL,
};

config_t const* const* configs = g_configs;

int config_skip_data(config_t const* config, data_t const* data)
{
    return config->use_dictionary && !data_has_dict(data);
}

int config_get_level(config_t const* config)
{
    size_t i;
    for (i = 0; i < config->param_values.size; ++i) {
        if (config->param_values.data[i].param == ZSTD_c_compressionLevel) {
            return config->param_values.data[i].value;
        }
    }
    return CONFIG_NO_LEVEL;
}
