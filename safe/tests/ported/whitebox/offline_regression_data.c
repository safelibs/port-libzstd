/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#include "data.h"

#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "util.h"

static data_t silesia = {
    .name = "silesia",
    .type = data_type_dir,
};

static data_t silesia_tar = {
    .name = "silesia.tar",
    .type = data_type_file,
};

static data_t github = {
    .name = "github",
    .type = data_type_dir,
    .dict = {
        .url = "offline-dict",
    },
};

static data_t github_tar = {
    .name = "github.tar",
    .type = data_type_file,
    .dict = {
        .url = "offline-dict",
    },
};

static data_t* g_data[] = {
    &silesia,
    &silesia_tar,
    &github,
    &github_tar,
    NULL,
};

data_t const* const* data = (data_t const* const*)g_data;

static char* g_data_dir = NULL;

int data_has_dict(data_t const* datum) {
    return datum->dict.url != NULL;
}

data_buffer_t data_buffer_create(size_t const capacity) {
    data_buffer_t buffer = {};

    buffer.data = (uint8_t*)malloc(capacity);
    if (buffer.data == NULL)
        return buffer;
    buffer.capacity = capacity;
    return buffer;
}

data_buffer_t data_buffer_read(char const* filename) {
    data_buffer_t buffer = {};
    uint64_t const size = UTIL_getFileSize(filename);

    if (size == UTIL_FILESIZE_UNKNOWN) {
        fprintf(stderr, "unknown size for %s\n", filename);
        return buffer;
    }

    buffer.data = (uint8_t*)malloc(size == 0 ? 1 : size);
    if (buffer.data == NULL) {
        fprintf(stderr, "malloc failed\n");
        return buffer;
    }
    buffer.capacity = size;

    FILE* file = fopen(filename, "rb");
    if (file == NULL) {
        fprintf(stderr, "failed to open %s: %s\n", filename, strerror(errno));
        free(buffer.data);
        memset(&buffer, 0, sizeof(buffer));
        return buffer;
    }

    buffer.size = fread(buffer.data, 1, buffer.capacity, file);
    fclose(file);
    if (buffer.size != buffer.capacity) {
        fprintf(stderr, "read %zu != %zu\n", buffer.size, buffer.capacity);
        free(buffer.data);
        memset(&buffer, 0, sizeof(buffer));
    }
    return buffer;
}

data_buffer_t data_buffer_get_data(data_t const* datum) {
    data_buffer_t const empty = {};
    if (datum->type != data_type_file)
        return empty;
    return data_buffer_read(datum->data.path);
}

data_buffer_t data_buffer_get_dict(data_t const* datum) {
    data_buffer_t const empty = {};
    if (!data_has_dict(datum))
        return empty;
    return data_buffer_read(datum->dict.path);
}

int data_buffer_compare(data_buffer_t lhs, data_buffer_t rhs) {
    size_t const size = lhs.size < rhs.size ? lhs.size : rhs.size;
    int const cmp = memcmp(lhs.data, rhs.data, size);
    if (cmp != 0)
        return cmp;
    if (lhs.size < rhs.size)
        return -1;
    if (lhs.size == rhs.size)
        return 0;
    return 1;
}

void data_buffer_free(data_buffer_t buffer) {
    free(buffer.data);
}

static FileNamesTable* data_filenames_get(data_t const* datum) {
    char const* const path = datum->data.path;
    return UTIL_createExpandedFNT(&path, 1, 0);
}

data_buffers_t data_buffers_get(data_t const* datum) {
    data_buffers_t buffers = { .size = 0 };
    FileNamesTable* const filenames = data_filenames_get(datum);
    size_t i;

    if (filenames == NULL)
        return buffers;
    if (filenames->tableSize == 0) {
        UTIL_freeFileNamesTable(filenames);
        return buffers;
    }

    data_buffer_t* const entries =
        (data_buffer_t*)calloc(filenames->tableSize, sizeof(*entries));
    if (entries == NULL) {
        UTIL_freeFileNamesTable(filenames);
        return buffers;
    }

    buffers.buffers = entries;
    buffers.size = filenames->tableSize;
    for (i = 0; i < filenames->tableSize; ++i) {
        entries[i] = data_buffer_read(filenames->fileNames[i]);
        if (entries[i].data == NULL) {
            data_buffers_free(buffers);
            UTIL_freeFileNamesTable(filenames);
            buffers.buffers = NULL;
            buffers.size = 0;
            return buffers;
        }
    }

    UTIL_freeFileNamesTable(filenames);
    return buffers;
}

void data_buffers_free(data_buffers_t buffers) {
    size_t i;
    for (i = 0; i < buffers.size; ++i) {
        free(((data_buffer_t*)buffers.buffers)[i].data);
    }
    free((data_buffer_t*)buffers.buffers);
}

static int ensure_directory_exists(char const* dir) {
    char* const copy = strdup(dir);
    char* pos = copy;
    int ret = 0;

    if (copy == NULL)
        return ENOMEM;

    do {
        char const save = *pos;
        if (*pos == '/' || *pos == '\0') {
            *pos = '\0';
            if (*copy != '\0' && mkdir(copy, S_IRWXU) != 0 && errno != EEXIST) {
                ret = errno;
                *pos = save;
                break;
            }
        }
        *pos = save;
        if (*pos == '\0')
            break;
        ++pos;
    } while (1);

    free(copy);
    return ret;
}

static char* cat3(char const* lhs, char const* mid, char const* rhs) {
    size_t const lhs_len = strlen(lhs);
    size_t const mid_len = strlen(mid);
    size_t const rhs_len = rhs == NULL ? 0 : strlen(rhs);
    size_t const total = lhs_len + mid_len + rhs_len + 1;
    char* const out = (char*)malloc(total);

    if (out == NULL)
        return NULL;
    memcpy(out, lhs, lhs_len);
    memcpy(out + lhs_len, mid, mid_len);
    if (rhs != NULL)
        memcpy(out + lhs_len + mid_len, rhs, rhs_len);
    out[total - 1] = '\0';
    return out;
}

static char* cat2(char const* lhs, char const* rhs) {
    return cat3(lhs, rhs, NULL);
}

static int data_create_paths(data_t* const* entries, char const* dir) {
    for (; *entries != NULL; ++entries) {
        data_t* const datum = *entries;
        datum->data.path = cat3(dir, "/", datum->name);
        if (datum->data.path == NULL)
            return ENOMEM;
        if (data_has_dict(datum)) {
            datum->dict.path = cat2(datum->data.path, ".dict");
            if (datum->dict.path == NULL)
                return ENOMEM;
        }
    }
    return 0;
}

static void data_free_paths(data_t* const* entries) {
    for (; *entries != NULL; ++entries) {
        data_t* const datum = *entries;
        free((void*)datum->data.path);
        free((void*)datum->dict.path);
        datum->data.path = NULL;
        datum->dict.path = NULL;
    }
}

static int verify_cache(data_t const* const* entries) {
    for (; *entries != NULL; ++entries) {
        data_t const* const datum = *entries;
        if (datum->type == data_type_dir) {
            if (!UTIL_isDirectory(datum->data.path)) {
                fprintf(stderr, "missing regression directory fixture: %s\n", datum->data.path);
                return ENOENT;
            }
        } else if (!UTIL_isRegularFile(datum->data.path)) {
            fprintf(stderr, "missing regression file fixture: %s\n", datum->data.path);
            return ENOENT;
        }
        if (data_has_dict(datum) && !UTIL_isRegularFile(datum->dict.path)) {
            fprintf(stderr, "missing regression dictionary fixture: %s\n", datum->dict.path);
            return ENOENT;
        }
    }
    return 0;
}

int data_init(char const* dir) {
    int err;

    if (dir == NULL)
        return EINVAL;

    err = ensure_directory_exists(dir);
    if (err != 0)
        return err;

    g_data_dir = strdup(dir);
    if (g_data_dir == NULL)
        return ENOMEM;

    err = data_create_paths(g_data, dir);
    if (err != 0)
        return err;

    err = verify_cache(data);
    if (err == 0) {
        fprintf(stderr, "using offline regression cache at %s\n", g_data_dir);
    }
    return err;
}

void data_finish(void) {
    data_free_paths(g_data);
    free(g_data_dir);
    g_data_dir = NULL;
}
