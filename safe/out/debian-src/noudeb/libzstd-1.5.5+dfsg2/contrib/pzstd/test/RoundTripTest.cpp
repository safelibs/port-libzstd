/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 */
extern "C" {
#include "datagen.h"
}
#include "Options.h"
#include "test/RoundTrip.h"
#include "utils/ScopeGuard.h"

#include <algorithm>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
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

string
writeData(size_t size, double matchProba, double litProba, unsigned seed) {
  std::unique_ptr<uint8_t[]> buf(new uint8_t[size]);
  RDG_genBuffer(buf.get(), size, matchProba, litProba, seed);
  string file = tmpnam(nullptr);
  auto fd = std::fopen(file.c_str(), "wb");
  auto guard = makeScopeGuard([&] { std::fclose(fd); });
  auto bytesWritten = std::fwrite(buf.get(), 1, size, fd);
  if (bytesWritten != size) {
    std::abort();
  }
  return file;
}

template <typename Generator>
string generateInputFile(Generator& gen) {
  // Use inputs ranging from 1 Byte to 2^16 Bytes
  std::uniform_int_distribution<size_t> size{1, 1 << 16};
  std::uniform_real_distribution<> prob{0, 1};
  return writeData(size(gen), prob(gen), prob(gen), gen());
}

template <typename Generator>
Options generateOptions(Generator& gen, const string& inputFile) {
  Options options;
  options.inputFiles = {inputFile};
  options.overwrite = true;

  const unsigned maxThreads =
      readPositiveEnvOrDefault("PZSTD_MAX_THREADS", 32);
  const unsigned maxLevel =
      readPositiveEnvOrDefault("PZSTD_MAX_LEVEL", 10);
  std::uniform_int_distribution<unsigned> numThreads{1, maxThreads};
  std::uniform_int_distribution<unsigned> compressionLevel{1, maxLevel};

  options.numThreads = numThreads(gen);
  options.compressionLevel = compressionLevel(gen);

  return options;
}
}

int main() {
  std::mt19937 gen(std::random_device{}());
  const unsigned inputCount =
      readPositiveEnvOrDefault("PZSTD_ROUNDTRIP_CASES", 10000);
  const unsigned optionsPerInput =
      readPositiveEnvOrDefault("PZSTD_ROUNDTRIP_OPTIONS_PER_INPUT", 10);
  const unsigned progressInterval = std::max(1U, inputCount / 100);

  auto newlineGuard = makeScopeGuard([] { std::fprintf(stderr, "\n"); });
  for (unsigned i = 0; i < inputCount; ++i) {
    if (i % progressInterval == 0) {
      std::fprintf(stderr, "Progress: %u/%u\r", i, inputCount);
    }
    auto inputFile = generateInputFile(gen);
    auto inputGuard = makeScopeGuard([&] { std::remove(inputFile.c_str()); });
    for (unsigned j = 0; j < optionsPerInput; ++j) {
      auto options = generateOptions(gen, inputFile);
      if (!roundTrip(options)) {
        std::fprintf(stderr, "numThreads: %u\n", options.numThreads);
        std::fprintf(stderr, "level: %u\n", options.compressionLevel);
        std::fprintf(stderr, "decompress? %u\n", (unsigned)options.decompress);
        std::fprintf(stderr, "file: %s\n", inputFile.c_str());
        return 1;
      }
    }
  }
  std::fprintf(stderr, "Progress: %u/%u\r", inputCount, inputCount);
  return 0;
}
