REPO_ROOT ?= $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/../../../..)
SAFE_ROOT ?= $(REPO_ROOT)/safe
ORIGINAL_ROOT ?= $(REPO_ROOT)/original/libzstd-1.5.5+dfsg2
HELPER_LIB ?= $(SAFE_ROOT)/out/original-cli/lib
BUILD_DIR ?= $(SAFE_ROOT)/out/phase6/whitebox

SAFE_LIBDIR ?= $(shell \
  if ls "$(SAFE_ROOT)/out/install/release-default/usr/lib"/*/libzstd.so.1.5.5 >/dev/null 2>&1; then \
    dirname "$$(ls "$(SAFE_ROOT)/out/install/release-default/usr/lib"/*/libzstd.so.1.5.5 | head -n 1)"; \
  else \
    printf '%s\n' "$(SAFE_ROOT)/out/install/release-default/usr/lib"; \
  fi)

CC ?= cc

FUZZ_CPPFLAGS = \
	-I$(ORIGINAL_ROOT)/tests/fuzz \
	-I$(ORIGINAL_ROOT)/programs \
	-I$(ORIGINAL_ROOT)/contrib/seekable_format \
	-I$(HELPER_LIB) \
	-I$(HELPER_LIB)/common
FUZZ_CFLAGS ?= -O2 -g -Wall -Wextra -Wno-deprecated-declarations
FUZZ_LDLIBS = $(HELPER_LIB)/libzstd.a -Wl,-rpath,$(SAFE_LIBDIR)

REG_CPPFLAGS = \
	-I$(ORIGINAL_ROOT)/tests \
	-I$(ORIGINAL_ROOT)/tests/regression \
	-I$(ORIGINAL_ROOT)/programs \
	-I$(HELPER_LIB) \
	-I$(HELPER_LIB)/common
REG_CFLAGS ?= -O2 -g -Wall -Wextra -Wno-deprecated-declarations
REG_LDLIBS = $(HELPER_LIB)/libzstd.a -Wl,-rpath,$(SAFE_LIBDIR)

FUZZ_TARGETS = \
	block_decompress \
	block_round_trip \
	decompress_dstSize_tooSmall \
	dictionary_decompress \
	dictionary_loader \
	dictionary_round_trip \
	dictionary_stream_round_trip \
	fse_read_ncount \
	huf_decompress \
	huf_round_trip \
	raw_dictionary_round_trip \
	seekable_roundtrip \
	sequence_compression_api \
	simple_compress \
	simple_decompress \
	simple_round_trip \
	stream_decompress \
	stream_round_trip \
	zstd_frame_info

FUZZ_BINARIES = $(addprefix $(BUILD_DIR)/fuzz/,$(FUZZ_TARGETS))
FUZZ_COMMON_OBJS = \
	$(BUILD_DIR)/fuzz/fuzz_helpers.o \
	$(BUILD_DIR)/fuzz/fuzz_data_producer.o \
	$(BUILD_DIR)/fuzz/zstd_helpers.o \
	$(BUILD_DIR)/fuzz/regression_driver.o \
	$(BUILD_DIR)/fuzz/util.o \
	$(BUILD_DIR)/fuzz/xxhash.o

FUZZ_SEEKABLE_OBJS = \
	$(BUILD_DIR)/fuzz/zstdseek_compress.o \
	$(BUILD_DIR)/fuzz/zstdseek_decompress.o

REGRESSION_BINARY = $(BUILD_DIR)/regression/regression-offline
REGRESSION_OBJS = \
	$(BUILD_DIR)/regression/config.o \
	$(BUILD_DIR)/regression/method.o \
	$(BUILD_DIR)/regression/result.o \
	$(BUILD_DIR)/regression/test.o \
	$(BUILD_DIR)/regression/util.o \
	$(BUILD_DIR)/regression/offline_regression_data.o

.PHONY: all clean fuzz regression

all: fuzz regression

fuzz: $(FUZZ_BINARIES)

regression: $(REGRESSION_BINARY)

$(BUILD_DIR)/fuzz:
	mkdir -p $@

$(BUILD_DIR)/regression:
	mkdir -p $@

$(BUILD_DIR)/fuzz/fuzz_helpers.o: $(ORIGINAL_ROOT)/tests/fuzz/fuzz_helpers.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/fuzz_data_producer.o: $(ORIGINAL_ROOT)/tests/fuzz/fuzz_data_producer.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/zstd_helpers.o: $(ORIGINAL_ROOT)/tests/fuzz/zstd_helpers.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/regression_driver.o: $(ORIGINAL_ROOT)/tests/fuzz/regression_driver.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/util.o: $(ORIGINAL_ROOT)/programs/util.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/xxhash.o: $(HELPER_LIB)/common/xxhash.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/zstdseek_compress.o: $(ORIGINAL_ROOT)/contrib/seekable_format/zstdseek_compress.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/zstdseek_decompress.o: $(ORIGINAL_ROOT)/contrib/seekable_format/zstdseek_decompress.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/%.o: $(ORIGINAL_ROOT)/tests/fuzz/%.c | $(BUILD_DIR)/fuzz
	$(CC) $(FUZZ_CPPFLAGS) $(FUZZ_CFLAGS) -c $< -o $@

$(BUILD_DIR)/fuzz/seekable_roundtrip: $(FUZZ_COMMON_OBJS) $(FUZZ_SEEKABLE_OBJS) $(BUILD_DIR)/fuzz/seekable_roundtrip.o
	$(CC) $(FUZZ_CFLAGS) $^ $(FUZZ_LDLIBS) -o $@

$(BUILD_DIR)/fuzz/%: $(FUZZ_COMMON_OBJS) $(BUILD_DIR)/fuzz/%.o
	$(CC) $(FUZZ_CFLAGS) $^ $(FUZZ_LDLIBS) -o $@

$(BUILD_DIR)/regression/config.o: $(ORIGINAL_ROOT)/tests/regression/config.c | $(BUILD_DIR)/regression
	$(CC) $(REG_CPPFLAGS) $(REG_CFLAGS) -c $< -o $@

$(BUILD_DIR)/regression/method.o: $(ORIGINAL_ROOT)/tests/regression/method.c | $(BUILD_DIR)/regression
	$(CC) $(REG_CPPFLAGS) $(REG_CFLAGS) -c $< -o $@

$(BUILD_DIR)/regression/result.o: $(ORIGINAL_ROOT)/tests/regression/result.c | $(BUILD_DIR)/regression
	$(CC) $(REG_CPPFLAGS) $(REG_CFLAGS) -c $< -o $@

$(BUILD_DIR)/regression/test.o: $(ORIGINAL_ROOT)/tests/regression/test.c | $(BUILD_DIR)/regression
	$(CC) $(REG_CPPFLAGS) $(REG_CFLAGS) -c $< -o $@

$(BUILD_DIR)/regression/util.o: $(ORIGINAL_ROOT)/programs/util.c | $(BUILD_DIR)/regression
	$(CC) $(REG_CPPFLAGS) $(REG_CFLAGS) -c $< -o $@

$(BUILD_DIR)/regression/offline_regression_data.o: $(dir $(lastword $(MAKEFILE_LIST)))offline_regression_data.c | $(BUILD_DIR)/regression
	$(CC) $(REG_CPPFLAGS) $(REG_CFLAGS) -c $< -o $@

$(REGRESSION_BINARY): $(REGRESSION_OBJS)
	$(CC) $(REG_CFLAGS) $^ $(REG_LDLIBS) -o $@

clean:
	rm -rf $(BUILD_DIR)
