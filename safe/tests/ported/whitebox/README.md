# White-Box Ports

This directory contains the phase-6 ports for upstream suites that used to
compile upstream `lib/*.c` sources directly.

- `Makefile` builds the ported fuzz drivers and the offline regression harness
  against the safe install tree created by `safe/scripts/build-artifacts.sh`
  and `safe/scripts/build-original-cli-against-safe.sh`.
- `offline_regression_data.c` removes the network-backed regression cache
  downloader and treats the cache directory as an already-populated offline
  fixture tree.
