# Phase Name

Dependent Inventory Expansion and Reproducible Image

# Implement Phase ID

`impl_dependent_image_matrix`

# Preexisting Inputs

- `safe/tests/upstream_test_matrix.toml`
- `safe/scripts/phase6-common.sh`
- `safe/scripts/run-upstream-tests.sh`
- `safe/scripts/run-original-playtests.sh`
- `safe/scripts/run-original-cli-tests.sh`
- `safe/scripts/run-original-gzip-tests.sh`
- `safe/scripts/run-zlibwrapper-tests.sh`
- `safe/scripts/run-educational-decoder-tests.sh`
- `safe/scripts/run-pzstd-tests.sh`
- `safe/scripts/run-seekable-tests.sh`
- `safe/scripts/run-version-compat-tests.sh`
- `safe/scripts/run-upstream-regression.sh`
- `safe/scripts/run-upstream-fuzz-tests.sh`
- `safe/scripts/run-original-examples.sh`
- `safe/scripts/check-cli-permissions.sh`
- `safe/scripts/run-performance-smoke.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/tests/fixtures/versions/manifest.toml`
- `safe/tests/fixtures/regression/README.md`
- `safe/tests/fixtures/fuzz-corpora/manifest.toml`
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/*.deb`
- `safe/out/deb/default/packages/*.udeb`
- `safe/out/deb/default/stage-root/`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `dependents.json`
- `safe/tests/dependents/dependent_matrix.toml`
- `safe/tests/dependents/src/apt_probe.c`
- `safe/tests/dependents/src/btrfs-progs_probe.c`
- `safe/tests/dependents/src/curl_probe.c`
- `safe/tests/dependents/src/dpkg_probe.c`
- `safe/tests/dependents/src/libarchive_probe.c`
- `safe/tests/dependents/src/qemu_probe.c`
- `safe/tests/dependents/src/rsync_probe.c`
- `safe/tests/dependents/src/squashfs-tools_probe.c`
- `safe/tests/dependents/src/systemd_probe.c`
- `safe/tests/dependents/src/tiff_probe.c`
- `safe/scripts/check-dependent-compile-compat.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `test-original.sh`

# New Outputs

- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed `safe/out/deb/default/packages/*.deb`
- refreshed when SAFE_ENABLE_UDEB=1 `safe/out/deb/default/packages/*.udeb`
- refreshed `safe/out/deb/default/stage-root/`
- rewritten `dependents.json`
- rewritten `safe/tests/dependents/dependent_matrix.toml`
- new `safe/tests/dependents/src/rpm_probe.c`
- new `safe/tests/dependents/src/zarchive_probe.c`
- new `safe/tests/dependents/fixtures/rpm/hello.spec`
- new `safe/tests/dependents/fixtures/rpm/hello.txt`
- new `safe/tests/dependents/fixtures/zarchive/input/a.txt`
- new `safe/tests/dependents/fixtures/zarchive/input/sub/b.txt`
- new `safe/docker/dependents/Dockerfile`
- new `safe/docker/dependents/entrypoint.sh`
- new `safe/scripts/build-dependent-image.sh`
- new `safe/scripts/run-dependent-matrix.sh`
- rewritten `safe/scripts/check-dependent-compile-compat.sh`
- rewritten `safe/scripts/verify-baseline-contract.sh`
- rewritten `test-original.sh`
- refreshed `safe/out/dependents/image-context/`
- new `safe/out/dependents/image-context/metadata.env`
- refreshed `safe/out/dependents/compile-compat/`
- refreshed `safe/out/dependents/logs/`

# File Changes

- Freeze a 12-application dependent inventory in `dependents.json` and `safe/tests/dependents/dependent_matrix.toml`.
- Replace the inline `docker run ... bash <<EOF` body in `test-original.sh` with checked-in scripts and a checked-in image definition that installs the safe `.deb` artifacts before tests run.
- Keep the current 10 applications and add exactly 2 more fixed runtime consumers: `rpm` and `zarchive`.
- Materialize checked-in runtime fixtures for the 2 new dependents instead of generating their only inputs from shell heredocs.
- Remove downstream use of `SAFE_UPSTREAM_LIB`, repo bind mounts, host Cargo binds, and host Rust toolchain binds from the final image-based test flow.
- Fix the downstream execution topology so compile compatibility and runtime coverage both execute inside the built image, while the host only orchestrates image build/run and persists logs plus probe outputs under `safe/out/dependents/`.
- Pin a single Ubuntu 24.04 image definition plus a single canonical image-tag metadata file under `safe/out/dependents/image-context/` so later phases reuse the same image identity.
- Preserve the current inventory-consistency checks, safe-package version checks, and safe-library-resolution checks while moving them into the checked-in image workflow.

# Implementation Details

- Preserve the existing 10 packages already curated in `dependents.json`.
- Add source package `rpm`, binary package `rpm`, compile probe `safe/tests/dependents/src/rpm_probe.c`, runtime test `test_rpm`, `compile_mode = "pkg-config-c"`, and `pkg_config_modules = ["libzstd"]`. The JSON entry must record that Ubuntu Noble source package `rpm` Build-Depends on `libzstd-dev`, and that the runtime path under test is `rpm`/`rpmbuild`/`rpm2cpio` using the `librpmio9t64 -> libzstd1` dependency chain.
- Add source package `zarchive`, binary package `zarchive-tools`, compile probe `safe/tests/dependents/src/zarchive_probe.c`, runtime test `test_zarchive`, `compile_mode = "pkg-config-c"`, and `pkg_config_modules = ["libzstd"]`. The JSON entry must record that Ubuntu Noble source package `zarchive` Build-Depends-Arch on `libzstd-dev`, and that `zarchive-tools` reaches `libzstd1` through `libzarchive0.1`.
- `safe/tests/dependents/src/rpm_probe.c` must follow the current minimal `pkg-config-c` pattern and reference `ZSTD_compressBound`, `ZSTD_createCCtx`, `ZSTD_CCtx_setParameter(..., ZSTD_c_compressionLevel, 19)`, `ZSTD_compress2`, `ZSTD_createDCtx`, `ZSTD_decompressDCtx`, and matching free calls.
- `safe/tests/dependents/src/zarchive_probe.c` must follow the same pattern but exercise `ZSTD_CStreamInSize`, `ZSTD_CStreamOutSize`, `ZSTD_createCStream`, `ZSTD_createDStream`, `ZSTD_compressStream2`, `ZSTD_decompressStream`, and matching free calls.
- Add checked-in runtime fixtures `safe/tests/dependents/fixtures/rpm/hello.spec` and `safe/tests/dependents/fixtures/rpm/hello.txt` for a tiny noarch RPM whose payload compressor must be queried as `zstd`.
- Add checked-in runtime fixtures `safe/tests/dependents/fixtures/zarchive/input/a.txt` and `safe/tests/dependents/fixtures/zarchive/input/sub/b.txt` for a create-then-extract round trip.
- `safe/docker/dependents/Dockerfile` must build from safe Debian artifacts already produced under `safe/out/deb/default/`. It must use Ubuntu 24.04 as the fixed base: `FROM ubuntu:24.04`, or an `ARG` whose default is exactly `ubuntu:24.04`.
- `safe/docker/dependents/Dockerfile` must install exactly the current downstream package set from `test-original.sh`: `apt`, `apt-utils`, `btrfs-progs`, `build-essential`, `ca-certificates`, `cmake`, `curl`, `debhelper`, `devscripts`, `dh-package-notes`, `dpkg-dev`, `fakeroot`, `help2man`, `jq`, `libarchive-tools`, `liblz4-dev`, `liblzma-dev`, `libtiff-tools`, `less`, `pkgconf`, `python3`, `python3-pil`, `qemu-utils`, `rsync`, `squashfs-tools`, `systemd`, `zlib1g-dev`, and `zstd`, plus new runtime packages `rpm`, `cpio`, `file`, and `zarchive-tools`.
- The image must bake in dependent fixtures, matrix metadata, and checked-in helper scripts so later `docker run` invocations do not require a repo mount. It must not need `cargo`, `rustc`, `HOST_CARGO_HOME`, or `HOST_RUSTUP_HOME`.
- Because `safe/scripts/build-deb.sh` hashes the full `safe/scripts/` tree and this phase rewrites scripts there, rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, and `bash safe/scripts/build-deb.sh` before image assembly.
- `safe/scripts/build-dependent-image.sh` must stage `safe/out/dependents/image-context/`, copy current safe `libzstd1`, `libzstd-dev`, and `zstd` `.deb` outputs plus `safe/out/deb/default/metadata.env`, `dependents.json`, `safe/tests/dependents/`, `safe/scripts/check-dependent-compile-compat.sh`, and `safe/docker/dependents/entrypoint.sh`, then `docker build` from `safe/docker/dependents/Dockerfile`.
- `safe/scripts/build-dependent-image.sh` must write `safe/out/dependents/image-context/metadata.env` with the single canonical local image tag/name and base image default, and later phases must source that file instead of inventing alternate tags.
- `safe/docker/dependents/entrypoint.sh` must expose `compile`, `runtime`, and `all` subcommands; `compile` invokes `safe/scripts/check-dependent-compile-compat.sh` inside the image and `runtime` dispatches per-application test functions.
- `safe/docker/dependents/entrypoint.sh` must keep the container root filesystem writable, use `/tmp/libzstd-dependent-tests`, preserve loopback image/mount coverage for `btrfs`, and preserve transient journald state under `/run`.
- `test_rpm` must assert that `/usr/bin/rpm`, `/usr/bin/rpmbuild`, and `/usr/bin/rpm2cpio` resolve `libzstd.so.1`; copy the checked-in RPM fixtures into `$TEST_ROOT/rpm/`; run `rpmbuild --define '_binary_payload w19.zstdio'`; assert `%{PAYLOADCOMPRESSOR}` is `zstd`; extract the RPM with `rpm2cpio | cpio -idmu`; and compare the extracted `hello.txt` with the fixture source.
- `test_zarchive` must assert that `/usr/bin/zarchive` resolves `libzstd.so.1`; copy the checked-in zarchive fixture tree into `$TEST_ROOT/zarchive/in/`; run `zarchive "$dir/in" "$dir/archive.za"` and `zarchive "$dir/archive.za" "$dir/out"`; and `diff -ru` the input and output trees.
- `safe/scripts/run-dependent-matrix.sh` must be the only host-side downstream executor. It must read image identity from `safe/out/dependents/image-context/metadata.env`, own image execution, in-image compile-compat execution, per-dependent runtime dispatch, and selective modes such as `--compile-only`, `--runtime-only`, and `--apps rpm,zarchive`.
- `safe/scripts/run-dependent-matrix.sh` must use `docker run --rm --privileged`, keep `/run` writable, and bind-mount only host output directories for `safe/out/dependents/logs/` and `safe/out/dependents/compile-compat/`.
- `safe/scripts/check-dependent-compile-compat.sh` and `safe/scripts/verify-baseline-contract.sh` must expect all 12 source packages and the new runtime mappings `rpm -> test_rpm` and `zarchive -> test_zarchive`. The compile-compat script must assume it is running inside the dependent image after safe packages are installed.
- `safe/docker/dependents/entrypoint.sh` and `test-original.sh` must preserve inventory-consistency checks, installed safe-package version equality and `safelibs` suffix checks for `libzstd1`, `libzstd-dev`, and `zstd`, and `assert_uses_safe_lib` checks before app-specific runtime coverage runs.
- `test-original.sh` must become the stable top-level wrapper that validates inventory consistency, ensures existing safe Debian artifacts exist, delegates to `safe/scripts/build-dependent-image.sh` and `safe/scripts/run-dependent-matrix.sh`, and no longer exports `SAFE_UPSTREAM_LIB` or mounts the repo/toolchain into runtime containers.

# Verification Phases

- Phase ID: `script_dependent_image_matrix`
- Type: `check`
- `bounce_target`: `impl_dependent_image_matrix`
- Purpose: build the checked-in image, install safe Debian packages in it, compile dependent probes, and exercise the current runtime matrix.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - `bash safe/scripts/build-artifacts.sh --release`
  - `bash safe/scripts/build-original-cli-against-safe.sh`
  - `bash safe/scripts/build-deb.sh`
  - `bash safe/scripts/build-dependent-image.sh`
  - `bash safe/scripts/run-dependent-matrix.sh --compile-only`
  - `bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps btrfs-progs,systemd`
  - `bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps rpm,zarchive`
  - `bash safe/scripts/verify-baseline-contract.sh`
  - `bash test-original.sh`

- Phase ID: `check_dependent_image_matrix_software_tester`
- Type: `check`
- `bounce_target`: `impl_dependent_image_matrix`
- Purpose: review dependent selection, compile-probe quality, and runtime-test determinism.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

- Phase ID: `check_dependent_image_matrix_senior_tester`
- Type: `check`
- `bounce_target`: `impl_dependent_image_matrix`
- Purpose: review that the workflow now uses a reproducible checked-in image instead of an inline container bootstrap.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

# Success Criteria

- The dependent inventory is exactly 12 applications, preserving the original 10 and adding `rpm` and `zarchive`.
- The checked-in image workflow builds from current safe Debian packages and Ubuntu 24.04.
- Compile compatibility and runtime tests run inside the image, with privileged writable-`/run` runtime containers and no repo, Cargo, Rustup, or upstream-library mounts.
- `safe/out/dependents/image-context/metadata.env` is the single downstream image identity source.
- `test_rpm` and `test_zarchive` are implemented with checked-in probes and fixtures and pass through `run-dependent-matrix.sh`.

# Git Commit Requirement

The implementer must commit all Phase 6 work to git before yielding. That commit must exist before `script_dependent_image_matrix`, `check_dependent_image_matrix_software_tester`, and `check_dependent_image_matrix_senior_tester` run.
