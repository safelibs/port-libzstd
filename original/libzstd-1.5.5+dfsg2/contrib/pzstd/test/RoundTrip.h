/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 */
#pragma once

#include "Options.h"
#include "Pzstd.h"
#include "utils/ScopeGuard.h"

#include <cstdio>
#include <cstdlib>
#include <string>
#include <cstdint>
#include <memory>
#include <vector>
#include <sys/wait.h>
#include <unistd.h>

namespace pzstd {

inline bool check(std::string source, std::string decompressed);

inline int runProgram(const std::string& binary,
                      const std::vector<std::string>& args) {
  std::vector<char*> argv;
  argv.reserve(args.size() + 2);
  argv.push_back(const_cast<char*>(binary.c_str()));
  for (const auto& arg : args) {
    argv.push_back(const_cast<char*>(arg.c_str()));
  }
  argv.push_back(nullptr);

  pid_t pid = fork();
  if (pid < 0) {
    return -1;
  }
  if (pid == 0) {
    execvp(binary.c_str(), argv.data());
    _exit(127);
  }

  int status = 0;
  if (waitpid(pid, &status, 0) < 0) {
    return -1;
  }
  if (!WIFEXITED(status)) {
    return -1;
  }
  return WEXITSTATUS(status);
}

inline bool roundTripWithExternalBin(const std::string& binary,
                                     const Options& options,
                                     const std::string& source,
                                     const std::string& compressedFile,
                                     const std::string& decompressedFile) {
  unsigned threads = options.numThreads == 0 ? 1 : options.numThreads;
  const char* styleEnv = std::getenv("PZSTD_ROUNDTRIP_STYLE");
  const bool useZstdStyle = styleEnv != nullptr && std::string(styleEnv) == "zstd";
  std::vector<std::string> compressArgs = {"-q", "-f"};
  if (useZstdStyle) {
    compressArgs.push_back("-T" + std::to_string(threads));
  } else {
    compressArgs.push_back("-p");
    compressArgs.push_back(std::to_string(threads));
  }
  compressArgs.push_back("-" + std::to_string(options.compressionLevel));
  compressArgs.push_back("-o");
  compressArgs.push_back(compressedFile);
  compressArgs.push_back(source);
  if (runProgram(binary, compressArgs) != 0) {
    return false;
  }

  std::vector<std::string> decompressArgs = {
      "-q",
      "-f",
      "-d",
      "-o",
      decompressedFile,
      compressedFile,
  };
  if (runProgram(binary, decompressArgs) != 0) {
    return false;
  }
  return check(source, decompressedFile);
}

inline bool check(std::string source, std::string decompressed) {
  std::unique_ptr<std::uint8_t[]> sBuf(new std::uint8_t[1024]);
  std::unique_ptr<std::uint8_t[]> dBuf(new std::uint8_t[1024]);

  auto sFd = std::fopen(source.c_str(), "rb");
  auto dFd = std::fopen(decompressed.c_str(), "rb");
  auto guard = makeScopeGuard([&] {
    std::fclose(sFd);
    std::fclose(dFd);
  });

  size_t sRead, dRead;

  do {
    sRead = std::fread(sBuf.get(), 1, 1024, sFd);
    dRead = std::fread(dBuf.get(), 1, 1024, dFd);
    if (std::ferror(sFd) || std::ferror(dFd)) {
      return false;
    }
    if (sRead != dRead) {
      return false;
    }

    for (size_t i = 0; i < sRead; ++i) {
      if (sBuf.get()[i] != dBuf.get()[i]) {
        return false;
      }
    }
  } while (sRead == 1024);
  if (!std::feof(sFd) || !std::feof(dFd)) {
    return false;
  }
  return true;
}

inline bool roundTrip(Options& options) {
  if (options.inputFiles.size() != 1) {
    return false;
  }
  std::string source = options.inputFiles.front();
  std::string compressedFile = std::tmpnam(nullptr);
  std::string decompressedFile = std::tmpnam(nullptr);
  auto guard = makeScopeGuard([&] {
    std::remove(compressedFile.c_str());
    std::remove(decompressedFile.c_str());
  });

  const char* externalBin = std::getenv("PZSTD_ROUNDTRIP_BIN");
  if (externalBin != nullptr && *externalBin != '\0') {
    return roundTripWithExternalBin(
        externalBin, options, source, compressedFile, decompressedFile);
  }

  {
    options.outputFile = compressedFile;
    options.decompress = false;
    if (pzstdMain(options) != 0) {
      return false;
    }
  }
  {
    options.decompress = true;
    options.inputFiles.front() = compressedFile;
    options.outputFile = decompressedFile;
    if (pzstdMain(options) != 0) {
      return false;
    }
  }
  return check(source, decompressedFile);
}
}
