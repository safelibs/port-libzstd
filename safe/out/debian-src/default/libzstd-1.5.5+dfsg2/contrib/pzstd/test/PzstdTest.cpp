/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 */
#include "Pzstd.h"
extern "C" {
#include "datagen.h"
}
#include "test/RoundTrip.h"
#include "utils/ScopeGuard.h"

#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <gtest/gtest.h>
#include <limits>
#include <memory>
#include <random>

using namespace std;
using namespace pzstd;

namespace {
unsigned readPositiveEnvOrDefault(const char* name, unsigned fallback) {
  const char* value = std::getenv(name);
  if (value == nullptr || *value == '\0') {
    return fallback;
  }

  char* end = nullptr;
  unsigned long parsed = std::strtoul(value, &end, 10);
  if (end == value || *end != '\0' || parsed == 0 ||
      parsed > std::numeric_limits<unsigned>::max()) {
    return fallback;
  }
  return static_cast<unsigned>(parsed);
}
}

TEST(Pzstd, SmallSizes) {
  unsigned seed = std::random_device{}();
  std::fprintf(stderr, "Pzstd.SmallSizes seed: %u\n", seed);
  std::mt19937 gen(seed);
  const unsigned maxLen = readPositiveEnvOrDefault("PZSTD_SMALL_MAX_LEN", 255);
  const unsigned maxThreads =
      readPositiveEnvOrDefault("PZSTD_MAX_THREADS", 2);
  const unsigned maxLevel =
      readPositiveEnvOrDefault("PZSTD_MAX_LEVEL", 4);

  for (unsigned len = 1; len <= maxLen; ++len) {
    if (len % 16 == 0) {
      std::fprintf(stderr, "%u / 16\n", len / 16);
    }
    std::string inputFile = std::tmpnam(nullptr);
    auto guard = makeScopeGuard([&] { std::remove(inputFile.c_str()); });
    {
      static uint8_t buf[256];
      RDG_genBuffer(buf, len, 0.5, 0.0, gen());
      auto fd = std::fopen(inputFile.c_str(), "wb");
      auto written = std::fwrite(buf, 1, len, fd);
      std::fclose(fd);
      ASSERT_EQ(written, len);
    }
    for (unsigned numThreads = 1; numThreads <= maxThreads; numThreads *= 2) {
      for (unsigned level : {1u, 4u}) {
        if (level > maxLevel) {
          continue;
        }
        auto errorGuard = makeScopeGuard([&] {
          std::fprintf(stderr, "# threads: %u\n", numThreads);
          std::fprintf(stderr, "compression level: %u\n", level);
        });
        Options options;
        options.overwrite = true;
        options.inputFiles = {inputFile};
        options.numThreads = numThreads;
        options.compressionLevel = level;
        options.verbosity = 1;
        ASSERT_TRUE(roundTrip(options));
        errorGuard.dismiss();
      }
    }
  }
}

TEST(Pzstd, LargeSizes) {
  unsigned seed = std::random_device{}();
  std::fprintf(stderr, "Pzstd.LargeSizes seed: %u\n", seed);
  std::mt19937 gen(seed);
  const unsigned minShift =
      readPositiveEnvOrDefault("PZSTD_LARGE_MIN_SHIFT", 20);
  const unsigned maxShift =
      readPositiveEnvOrDefault("PZSTD_LARGE_MAX_SHIFT", 24);
  const unsigned maxThreads =
      readPositiveEnvOrDefault("PZSTD_MAX_THREADS", 16);
  const unsigned maxLevel =
      readPositiveEnvOrDefault("PZSTD_MAX_LEVEL", 4);

  for (unsigned len = 1U << minShift; len <= (1U << maxShift); len *= 2) {
    std::string inputFile = std::tmpnam(nullptr);
    auto guard = makeScopeGuard([&] { std::remove(inputFile.c_str()); });
    {
      std::unique_ptr<uint8_t[]> buf(new uint8_t[len]);
      RDG_genBuffer(buf.get(), len, 0.5, 0.0, gen());
      auto fd = std::fopen(inputFile.c_str(), "wb");
      auto written = std::fwrite(buf.get(), 1, len, fd);
      std::fclose(fd);
      ASSERT_EQ(written, len);
    }
    for (unsigned numThreads = 1; numThreads <= maxThreads; numThreads *= 4) {
      for (unsigned level : {1u, 4u}) {
        if (level > maxLevel) {
          continue;
        }
        auto errorGuard = makeScopeGuard([&] {
          std::fprintf(stderr, "# threads: %u\n", numThreads);
          std::fprintf(stderr, "compression level: %u\n", level);
        });
        Options options;
        options.overwrite = true;
        options.inputFiles = {inputFile};
        options.numThreads = std::min(numThreads, options.numThreads);
        options.compressionLevel = level;
        options.verbosity = 1;
        ASSERT_TRUE(roundTrip(options));
        errorGuard.dismiss();
      }
    }
  }
}

TEST(Pzstd, DISABLED_ExtremelyLargeSize) {
  unsigned seed = std::random_device{}();
  std::fprintf(stderr, "Pzstd.ExtremelyLargeSize seed: %u\n", seed);
  std::mt19937 gen(seed);

  std::string inputFile = std::tmpnam(nullptr);
  auto guard = makeScopeGuard([&] { std::remove(inputFile.c_str()); });

  {
    // Write 4GB + 64 MB
    constexpr size_t kLength = 1 << 26;
    std::unique_ptr<uint8_t[]> buf(new uint8_t[kLength]);
    auto fd = std::fopen(inputFile.c_str(), "wb");
    auto closeGuard = makeScopeGuard([&] { std::fclose(fd); });
    for (size_t i = 0; i < (1 << 6) + 1; ++i) {
      RDG_genBuffer(buf.get(), kLength, 0.5, 0.0, gen());
      auto written = std::fwrite(buf.get(), 1, kLength, fd);
      if (written != kLength) {
        std::fprintf(stderr, "Failed to write file, skipping test\n");
        return;
      }
    }
  }

  Options options;
  options.overwrite = true;
  options.inputFiles = {inputFile};
  options.compressionLevel = 1;
  if (options.numThreads == 0) {
    options.numThreads = 1;
  }
  ASSERT_TRUE(roundTrip(options));
}

TEST(Pzstd, ExtremelyCompressible) {
  std::string inputFile = std::tmpnam(nullptr);
  auto guard = makeScopeGuard([&] { std::remove(inputFile.c_str()); });
  {
    std::unique_ptr<uint8_t[]> buf(new uint8_t[10000]);
    std::memset(buf.get(), 'a', 10000);
    auto fd = std::fopen(inputFile.c_str(), "wb");
    auto written = std::fwrite(buf.get(), 1, 10000, fd);
    std::fclose(fd);
    ASSERT_EQ(written, 10000);
  }
  Options options;
  options.overwrite = true;
  options.inputFiles = {inputFile};
  options.numThreads = 1;
  options.compressionLevel = 1;
  ASSERT_TRUE(roundTrip(options));
}
