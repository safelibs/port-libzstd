/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#ifndef CONFIG_H
#define CONFIG_H

#include <stddef.h>

#include <zstd.h>

#include "data.h"

typedef struct {
    ZSTD_cParameter param;
    int value;
} param_value_t;

typedef struct {
    size_t size;
    param_value_t const* data;
} param_values_t;

typedef struct {
    const char* name;
    char const* cli_args;
    param_values_t param_values;
    int use_dictionary;
    int no_pledged_src_size;
} config_t;

int config_skip_data(config_t const* config, data_t const* data);

#define CONFIG_NO_LEVEL (-1000000)
int config_get_level(config_t const* config);

extern config_t const* const* configs;

#endif
