# Link-Compatibility Harness

This harness proves that objects compiled against the original upstream headers
can link against the safe `libzstd.so` without recompiling those sources
against any safe-port headers.

The `Makefile` keeps compilation and linking separate:

- `*.o` files are compiled with include paths rooted in
  `original/libzstd-1.5.5+dfsg2`.
- final binaries are linked against `safe/target/release/libzstd.so`.
- the run target executes Debian `ztest` clients, upstream example objects, and
  phase-4 advanced API drivers built from retained object files.

Typical usage:

```sh
make -C safe/tests/link-compat run
```
