/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#include "method.h"

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

#include <zstd.h>

#define MIN(x, y) ((x) < (y) ? (x) : (y))

static char const* g_zstdcli = NULL;

void method_set_zstdcli(char const* zstdcli)
{
    g_zstdcli = zstdcli;
}

#define container_of(ptr, type, member) \
    ((type*)(ptr == NULL ? NULL : (char*)(ptr)-offsetof(type, member)))

typedef struct {
    method_state_t base;
    data_buffers_t inputs;
    data_buffer_t dictionary;
    data_buffer_t compressed;
    data_buffer_t decompressed;
} buffer_state_t;

static size_t buffers_max_size(data_buffers_t buffers)
{
    size_t max = 0;
    size_t i;
    for (i = 0; i < buffers.size; ++i) {
        if (buffers.buffers[i].size > max) {
            max = buffers.buffers[i].size;
        }
    }
    return max;
}

static method_state_t* buffer_state_create(data_t const* data)
{
    buffer_state_t* state = (buffer_state_t*)calloc(1, sizeof(buffer_state_t));
    size_t max_size;
    if (state == NULL) {
        return NULL;
    }
    state->base.data = data;
    state->inputs = data_buffers_get(data);
    state->dictionary = data_buffer_get_dict(data);
    max_size = buffers_max_size(state->inputs);
    state->compressed = data_buffer_create(ZSTD_compressBound(max_size));
    state->decompressed = data_buffer_create(max_size == 0 ? 1 : max_size);
    return &state->base;
}

static void buffer_state_destroy(method_state_t* base)
{
    buffer_state_t* state;
    if (base == NULL) {
        return;
    }
    state = container_of(base, buffer_state_t, base);
    data_buffers_free(state->inputs);
    data_buffer_free(state->dictionary);
    data_buffer_free(state->compressed);
    data_buffer_free(state->decompressed);
    free(state);
}

static int buffer_state_bad(buffer_state_t const* state, config_t const* config)
{
    if (state == NULL) {
        fprintf(stderr, "buffer_state_t is NULL\n");
        return 1;
    }
    if (state->inputs.size == 0 || state->compressed.data == NULL ||
        state->decompressed.data == NULL) {
        fprintf(stderr, "buffer state allocation failure\n");
        return 1;
    }
    if (config->use_dictionary && state->dictionary.data == NULL) {
        fprintf(stderr, "dictionary loading failed\n");
        return 1;
    }
    return 0;
}

static int config_is_level_only(config_t const* config)
{
    size_t i;
    if (config->param_values.size == 0) {
        return 0;
    }
    for (i = 0; i < config->param_values.size; ++i) {
        if (config->param_values.data[i].param != ZSTD_c_compressionLevel) {
            return 0;
        }
    }
    return 1;
}

static method_state_t* method_state_create(data_t const* data)
{
    method_state_t* state = (method_state_t*)malloc(sizeof(method_state_t));
    if (state == NULL) {
        return NULL;
    }
    state->data = data;
    return state;
}

static void method_state_destroy(method_state_t* state)
{
    free(state);
}

static int prepare_cctx(ZSTD_CCtx* cctx, buffer_state_t* state,
                        config_t const* config, size_t srcSize)
{
    size_t code;
    size_t i;
    code = ZSTD_CCtx_reset(cctx, ZSTD_reset_session_and_parameters);
    if (ZSTD_isError(code)) {
        return 1;
    }
    for (i = 0; i < config->param_values.size; ++i) {
        param_value_t const pv = config->param_values.data[i];
        code = ZSTD_CCtx_setParameter(cctx, pv.param, pv.value);
        if (ZSTD_isError(code)) {
            return 1;
        }
    }
    if (config->use_dictionary) {
        code = ZSTD_CCtx_loadDictionary(cctx, state->dictionary.data, state->dictionary.size);
        if (ZSTD_isError(code)) {
            return 1;
        }
    }
    if (!config->no_pledged_src_size) {
        code = ZSTD_CCtx_setPledgedSrcSize(cctx, srcSize);
        if (ZSTD_isError(code)) {
            return 1;
        }
    }
    return 0;
}

static int prepare_dctx(ZSTD_DCtx* dctx, buffer_state_t* state, config_t const* config)
{
    size_t code = ZSTD_DCtx_reset(dctx, ZSTD_reset_session_and_parameters);
    if (ZSTD_isError(code)) {
        return 1;
    }
    if (config->use_dictionary) {
        code = ZSTD_DCtx_loadDictionary(dctx, state->dictionary.data, state->dictionary.size);
        if (ZSTD_isError(code)) {
            return 1;
        }
    }
    return 0;
}

static result_t round_trip_compress2(method_state_t* base, config_t const* config,
                                     size_t subtract)
{
    buffer_state_t* state = container_of(base, buffer_state_t, base);
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    result_data_t data = { 0 };
    result_t result;
    size_t i;

    if (buffer_state_bad(state, config)) {
        return result_error(result_error_system_error);
    }
    if (cctx == NULL || dctx == NULL) {
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        return result_error(result_error_system_error);
    }

    for (i = 0; i < state->inputs.size; ++i) {
        data_buffer_t const input = state->inputs.buffers[i];
        size_t const capacity = ZSTD_compressBound(input.size) > subtract
            ? ZSTD_compressBound(input.size) - subtract
            : 0;
        size_t cSize;
        size_t dSize;

        if (prepare_cctx(cctx, state, config, input.size)) {
            result = result_error(result_error_compression_error);
            goto out;
        }
        cSize = ZSTD_compress2(cctx, state->compressed.data, capacity, input.data, input.size);
        if (ZSTD_isError(cSize)) {
            if (subtract != 0) {
                result = result_error(result_error_skip);
            } else {
                result = result_error(result_error_compression_error);
            }
            goto out;
        }
        if (prepare_dctx(dctx, state, config)) {
            result = result_error(result_error_decompression_error);
            goto out;
        }
        dSize = ZSTD_decompressDCtx(dctx, state->decompressed.data,
                                    state->decompressed.capacity,
                                    state->compressed.data, cSize);
        if (ZSTD_isError(dSize)) {
            result = result_error(result_error_decompression_error);
            goto out;
        }
        state->decompressed.size = dSize;
        if (data_buffer_compare(input, state->decompressed) != 0) {
            result = result_error(result_error_round_trip_error);
            goto out;
        }
        data.total_size += cSize;
    }

    result = result_data(data);
out:
    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    return result;
}

static result_t simple_compress(method_state_t* base, config_t const* config)
{
    buffer_state_t* state = container_of(base, buffer_state_t, base);
    data_buffer_t input;
    int level;
    size_t cSize;
    size_t dSize;
    result_data_t data;

    if (buffer_state_bad(state, config)) {
        return result_error(result_error_system_error);
    }
    if (base->data->type != data_type_file) {
        return result_error(result_error_skip);
    }
    if (config->use_dictionary || config->no_pledged_src_size || !config_is_level_only(config)) {
        return result_error(result_error_skip);
    }

    level = config_get_level(config);
    if (level == CONFIG_NO_LEVEL) {
        return result_error(result_error_skip);
    }

    input = state->inputs.buffers[0];
    cSize = ZSTD_compress(state->compressed.data, state->compressed.capacity,
                          input.data, input.size, level);
    if (ZSTD_isError(cSize)) {
        return result_error(result_error_compression_error);
    }
    dSize = ZSTD_decompress(state->decompressed.data, state->decompressed.capacity,
                            state->compressed.data, cSize);
    if (ZSTD_isError(dSize)) {
        return result_error(result_error_decompression_error);
    }
    state->decompressed.size = dSize;
    if (data_buffer_compare(input, state->decompressed) != 0) {
        return result_error(result_error_round_trip_error);
    }
    data.total_size = cSize;
    return result_data(data);
}

static result_t compress_cctx_compress(method_state_t* base, config_t const* config)
{
    buffer_state_t* state = container_of(base, buffer_state_t, base);
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    result_data_t data = { 0 };
    result_t result;
    int level;
    size_t i;

    if (buffer_state_bad(state, config)) {
        return result_error(result_error_system_error);
    }
    if (!config_is_level_only(config)) {
        return result_error(result_error_skip);
    }

    level = config_get_level(config);
    if (level == CONFIG_NO_LEVEL) {
        return result_error(result_error_skip);
    }
    if (cctx == NULL || dctx == NULL) {
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        return result_error(result_error_system_error);
    }

    for (i = 0; i < state->inputs.size; ++i) {
        data_buffer_t const input = state->inputs.buffers[i];
        size_t cSize;
        size_t dSize;

        if (config->use_dictionary) {
            cSize = ZSTD_compress_usingDict(cctx,
                                            state->compressed.data,
                                            state->compressed.capacity,
                                            input.data,
                                            input.size,
                                            state->dictionary.data,
                                            state->dictionary.size,
                                            level);
        } else {
            cSize = ZSTD_compressCCtx(cctx,
                                      state->compressed.data,
                                      state->compressed.capacity,
                                      input.data,
                                      input.size,
                                      level);
        }
        if (ZSTD_isError(cSize)) {
            result = result_error(result_error_compression_error);
            goto out;
        }
        if (prepare_dctx(dctx, state, config)) {
            result = result_error(result_error_decompression_error);
            goto out;
        }
        dSize = ZSTD_decompressDCtx(dctx,
                                    state->decompressed.data,
                                    state->decompressed.capacity,
                                    state->compressed.data,
                                    cSize);
        if (ZSTD_isError(dSize)) {
            result = result_error(result_error_decompression_error);
            goto out;
        }
        state->decompressed.size = dSize;
        if (data_buffer_compare(input, state->decompressed) != 0) {
            result = result_error(result_error_round_trip_error);
            goto out;
        }
        data.total_size += cSize;
    }

    result = result_data(data);
out:
    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    return result;
}

static result_t advanced_one_pass_compress(method_state_t* base, config_t const* config)
{
    return round_trip_compress2(base, config, 0);
}

static result_t advanced_one_pass_compress_small_output(method_state_t* base, config_t const* config)
{
    return round_trip_compress2(base, config, 1);
}

static result_t advanced_streaming_compress(method_state_t* base, config_t const* config)
{
    buffer_state_t* state = container_of(base, buffer_state_t, base);
    ZSTD_CCtx* cctx = ZSTD_createCCtx();
    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    result_data_t data = { 0 };
    result_t result;
    size_t i;

    if (buffer_state_bad(state, config)) {
        return result_error(result_error_system_error);
    }
    if (cctx == NULL || dctx == NULL) {
        ZSTD_freeCCtx(cctx);
        ZSTD_freeDCtx(dctx);
        return result_error(result_error_system_error);
    }

    for (i = 0; i < state->inputs.size; ++i) {
        data_buffer_t input = state->inputs.buffers[i];
        size_t cSize = 0;
        size_t ret = 1;

        if (prepare_cctx(cctx, state, config, input.size)) {
            result = result_error(result_error_compression_error);
            goto out;
        }
        while (input.size > 0) {
            ZSTD_inBuffer in = { input.data, MIN(input.size, (size_t)4096), 0 };
            input.data += in.size;
            input.size -= in.size;
            while (in.pos < in.size) {
                ZSTD_outBuffer out = { state->compressed.data + cSize,
                                       state->compressed.capacity - cSize, 0 };
                ret = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_continue);
                if (ZSTD_isError(ret)) {
                    result = result_error(result_error_compression_error);
                    goto out;
                }
                cSize += out.pos;
            }
        }
        do {
            ZSTD_inBuffer in = { NULL, 0, 0 };
            ZSTD_outBuffer out = { state->compressed.data + cSize,
                                   state->compressed.capacity - cSize, 0 };
            ret = ZSTD_compressStream2(cctx, &out, &in, ZSTD_e_end);
            if (ZSTD_isError(ret)) {
                result = result_error(result_error_compression_error);
                goto out;
            }
            cSize += out.pos;
        } while (ret != 0);
        if (prepare_dctx(dctx, state, config)) {
            result = result_error(result_error_decompression_error);
            goto out;
        }
        {
            size_t const dSize = ZSTD_decompressDCtx(dctx, state->decompressed.data,
                                                     state->decompressed.capacity,
                                                     state->compressed.data, cSize);
            if (ZSTD_isError(dSize)) {
                result = result_error(result_error_decompression_error);
                goto out;
            }
            state->decompressed.size = dSize;
        }
        if (data_buffer_compare(state->inputs.buffers[i], state->decompressed) != 0) {
            result = result_error(result_error_round_trip_error);
            goto out;
        }
        data.total_size += cSize;
    }

    result = result_data(data);
out:
    ZSTD_freeCCtx(cctx);
    ZSTD_freeDCtx(dctx);
    return result;
}

static result_t cli_compress(method_state_t* state, config_t const* config)
{
    char cmd[1024];
    size_t cmd_size;
    FILE* zstd;
    char out[4096];
    size_t total_size = 0;
    if (config->cli_args == NULL) {
        return result_error(result_error_skip);
    }
    if (state->data->type == data_type_dir && config->no_pledged_src_size) {
        return result_error(result_error_skip);
    }
    if (g_zstdcli == NULL) {
        return result_error(result_error_system_error);
    }
    cmd_size = snprintf(cmd, sizeof(cmd), "'%s' -cqr %s %s%s%s %s '%s'",
                        g_zstdcli, config->cli_args,
                        config->use_dictionary ? "-D '" : "",
                        config->use_dictionary ? state->data->dict.path : "",
                        config->use_dictionary ? "'" : "",
                        config->no_pledged_src_size ? "<" : "",
                        state->data->data.path);
    if (cmd_size >= sizeof(cmd)) {
        return result_error(result_error_system_error);
    }
    zstd = popen(cmd, "r");
    if (zstd == NULL) {
        return result_error(result_error_system_error);
    }
    while (1) {
        size_t const size = fread(out, 1, sizeof(out), zstd);
        total_size += size;
        if (size != sizeof(out)) {
            break;
        }
    }
    if (ferror(zstd) || pclose(zstd) != 0) {
        return result_error(result_error_compression_error);
    }
    return result_data((result_data_t){ total_size });
}

method_t const simple = {
    .name = "compress simple",
    .create = buffer_state_create,
    .compress = simple_compress,
    .destroy = buffer_state_destroy,
};

method_t const compress_cctx = {
    .name = "compress cctx",
    .create = buffer_state_create,
    .compress = compress_cctx_compress,
    .destroy = buffer_state_destroy,
};

method_t const cli = {
    .name = "zstdcli",
    .create = method_state_create,
    .compress = cli_compress,
    .destroy = method_state_destroy,
};

method_t const advanced_one_pass = {
    .name = "advanced one pass",
    .create = buffer_state_create,
    .compress = advanced_one_pass_compress,
    .destroy = buffer_state_destroy,
};

method_t const advanced_one_pass_small_out = {
    .name = "advanced one pass small out",
    .create = buffer_state_create,
    .compress = advanced_one_pass_compress_small_output,
    .destroy = buffer_state_destroy,
};

method_t const advanced_streaming = {
    .name = "advanced streaming",
    .create = buffer_state_create,
    .compress = advanced_streaming_compress,
    .destroy = buffer_state_destroy,
};

static method_t const* g_methods[] = {
    &simple,
    &compress_cctx,
    &cli,
    &advanced_one_pass,
    &advanced_one_pass_small_out,
    &advanced_streaming,
    NULL,
};

method_t const* const* methods = g_methods;
