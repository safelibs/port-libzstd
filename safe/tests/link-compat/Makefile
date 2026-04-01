SAFE_ROOT ?= $(abspath ../..)
REPO_ROOT ?= $(abspath ../../..)
ORIGINAL_ROOT := $(REPO_ROOT)/original/libzstd-1.5.5+dfsg2
BUILD_DIR ?= $(SAFE_ROOT)/target/link-compat
OBJ_DIR := $(BUILD_DIR)/obj
BIN_DIR := $(BUILD_DIR)/bin
WORK_DIR := $(BUILD_DIR)/work
RUNTIME_DIR := $(BUILD_DIR)/runtime
SHARED_LIB := $(SAFE_ROOT)/target/release/libzstd.so
RUNTIME_LIB := $(RUNTIME_DIR)/libzstd.so.1
LD_PATH := $(RUNTIME_DIR)$${LD_LIBRARY_PATH:+:$$LD_LIBRARY_PATH}

CARGO ?= cargo
CC ?= cc
WARN_FLAGS := -std=c11 -Wall -Wextra -Werror -Wno-deprecated-declarations -Wno-sign-compare -Wno-unused-function -Wno-unused-parameter -D_POSIX_C_SOURCE=200809L
ORIGINAL_CPPFLAGS := -I$(ORIGINAL_ROOT)/lib -I$(ORIGINAL_ROOT)/examples
LINK_FLAGS := -L$(SAFE_ROOT)/target/release -Wl,-rpath,$(RUNTIME_DIR) -lzstd
SAFE_CARGO_ARGS ?= --release --crate-type cdylib

OBJECTS := \
	$(OBJ_DIR)/pkg_make_ztest.o \
	$(OBJ_DIR)/cmake_ztest.o \
	$(OBJ_DIR)/zstreamtest.o \
	$(OBJ_DIR)/run_zstreamtest.o \
	$(OBJ_DIR)/poolTests.o \
	$(OBJ_DIR)/run_pooltests.o \
	$(OBJ_DIR)/streaming_memory_usage.o \
	$(OBJ_DIR)/streaming_compression_thread_pool.o \
	$(OBJ_DIR)/dict_builder_driver.o \
	$(OBJ_DIR)/sequence_api_driver.o \
	$(OBJ_DIR)/thread_pool_driver.o

BINARIES := \
	$(BIN_DIR)/pkg_make_ztest_safe \
	$(BIN_DIR)/cmake_ztest_safe \
	$(BIN_DIR)/zstreamtest_safe \
	$(BIN_DIR)/poolTests_safe \
	$(BIN_DIR)/streaming_memory_usage_safe \
	$(BIN_DIR)/streaming_compression_thread_pool_safe \
	$(BIN_DIR)/dict_builder_driver_safe \
	$(BIN_DIR)/sequence_api_driver_safe \
	$(BIN_DIR)/thread_pool_driver_safe

.PHONY: all safe-lib objects binaries run clean workdirs

all: run

workdirs:
	mkdir -p $(OBJ_DIR) $(BIN_DIR) $(WORK_DIR) $(RUNTIME_DIR)

safe-lib: | workdirs
	$(CARGO) rustc --manifest-path $(SAFE_ROOT)/Cargo.toml $(SAFE_CARGO_ARGS)
	ln -sf $(SHARED_LIB) $(RUNTIME_LIB)

objects: $(OBJECTS)

binaries: safe-lib $(BINARIES)

$(OBJ_DIR)/pkg_make_ztest.o: $(ORIGINAL_ROOT)/debian/tests/ztest/pkg-make/ztest.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -c $< -o $@

$(OBJ_DIR)/cmake_ztest.o: $(ORIGINAL_ROOT)/debian/tests/ztest/cmake/ztest.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -c $< -o $@

$(OBJ_DIR)/zstreamtest.o: $(ORIGINAL_ROOT)/tests/zstreamtest.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -Dmain=upstream_zstreamtest_main -c $< -o $@

$(OBJ_DIR)/run_zstreamtest.o: $(SAFE_ROOT)/tests/link-compat/run_zstreamtest.c | workdirs
	$(CC) $(WARN_FLAGS) -c $< -o $@

$(OBJ_DIR)/poolTests.o: $(ORIGINAL_ROOT)/tests/poolTests.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -Dmain=upstream_poolTests_main -c $< -o $@

$(OBJ_DIR)/run_pooltests.o: $(SAFE_ROOT)/tests/link-compat/run_pooltests.c | workdirs
	$(CC) $(WARN_FLAGS) -c $< -o $@

$(OBJ_DIR)/streaming_memory_usage.o: $(ORIGINAL_ROOT)/examples/streaming_memory_usage.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -c $< -o $@

$(OBJ_DIR)/streaming_compression_thread_pool.o: $(ORIGINAL_ROOT)/examples/streaming_compression_thread_pool.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -DZSTD_STATIC_LINKING_ONLY -c $< -o $@

$(OBJ_DIR)/dict_builder_driver.o: $(SAFE_ROOT)/tests/capi/dict_builder_driver.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -c $< -o $@

$(OBJ_DIR)/sequence_api_driver.o: $(SAFE_ROOT)/tests/capi/sequence_api_driver.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -c $< -o $@

$(OBJ_DIR)/thread_pool_driver.o: $(SAFE_ROOT)/tests/capi/thread_pool_driver.c | workdirs
	$(CC) $(WARN_FLAGS) $(ORIGINAL_CPPFLAGS) -c $< -o $@

$(BIN_DIR)/pkg_make_ztest_safe: $(OBJ_DIR)/pkg_make_ztest.o safe-lib | workdirs
	$(CC) $< -o $@ $(LINK_FLAGS)

$(BIN_DIR)/cmake_ztest_safe: $(OBJ_DIR)/cmake_ztest.o safe-lib | workdirs
	$(CC) $< -o $@ $(LINK_FLAGS)

$(BIN_DIR)/zstreamtest_safe: $(OBJ_DIR)/zstreamtest.o $(OBJ_DIR)/run_zstreamtest.o safe-lib | workdirs
	$(CC) $(OBJ_DIR)/zstreamtest.o $(OBJ_DIR)/run_zstreamtest.o -o $@ $(LINK_FLAGS)

$(BIN_DIR)/poolTests_safe: $(OBJ_DIR)/poolTests.o $(OBJ_DIR)/run_pooltests.o safe-lib | workdirs
	$(CC) $(OBJ_DIR)/poolTests.o $(OBJ_DIR)/run_pooltests.o -o $@ $(LINK_FLAGS) -pthread

$(BIN_DIR)/streaming_memory_usage_safe: $(OBJ_DIR)/streaming_memory_usage.o safe-lib | workdirs
	$(CC) $< -o $@ $(LINK_FLAGS)

$(BIN_DIR)/streaming_compression_thread_pool_safe: $(OBJ_DIR)/streaming_compression_thread_pool.o safe-lib | workdirs
	$(CC) $< -o $@ $(LINK_FLAGS) -pthread

$(BIN_DIR)/dict_builder_driver_safe: $(OBJ_DIR)/dict_builder_driver.o safe-lib | workdirs
	$(CC) $< -o $@ $(LINK_FLAGS)

$(BIN_DIR)/sequence_api_driver_safe: $(OBJ_DIR)/sequence_api_driver.o safe-lib | workdirs
	$(CC) $< -o $@ $(LINK_FLAGS)

$(BIN_DIR)/thread_pool_driver_safe: $(OBJ_DIR)/thread_pool_driver.o safe-lib | workdirs
	$(CC) $< -o $@ $(LINK_FLAGS)

$(WORK_DIR)/input-one.txt $(WORK_DIR)/input-two.txt: | workdirs
	python3 -c 'from pathlib import Path; import random; work = Path("$(WORK_DIR)"); work.mkdir(parents=True, exist_ok=True); alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_"; random.seed(0x11112222); (work / "input-one.txt").write_bytes(bytes(alphabet[random.randrange(len(alphabet))] for _ in range(144 * 1024 + 9))); random.seed(0x33334444); (work / "input-two.txt").write_bytes(bytes(alphabet[random.randrange(len(alphabet))] for _ in range(112 * 1024 + 21)))'

run: binaries $(WORK_DIR)/input-one.txt $(WORK_DIR)/input-two.txt
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/pkg_make_ztest_safe
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/cmake_ztest_safe
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/zstreamtest_safe
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/poolTests_safe
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/dict_builder_driver_safe
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/sequence_api_driver_safe
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/thread_pool_driver_safe
	LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/streaming_memory_usage_safe
	cd $(WORK_DIR) && LD_LIBRARY_PATH=$(LD_PATH) $(BIN_DIR)/streaming_compression_thread_pool_safe 2 3 input-one.txt input-two.txt

clean:
	rm -rf $(BUILD_DIR)
