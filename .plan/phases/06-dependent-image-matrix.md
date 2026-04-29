# Phase Name

Dependent Inventory Expansion and Reproducible Image

# Implement Phase ID

`impl_dependent_image_matrix`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- All outputs from `impl_upstream_release_gates`, including `safe/tests/upstream_test_matrix.toml`, `safe/scripts/phase6-common.sh`, the upstream wrapper scripts under `safe/scripts/`, `safe/scripts/run-full-suite.sh`, the fixture manifests under `safe/tests/fixtures/`, and the refreshed Phase 4 artifact roots under `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`, and `safe/out/deb/default/`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
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

The current inventories and existing 10 dependent probes are consumed and updated in place. The image files and dependent output roots are introduced here, then become the only downstream image and results artifacts for later phases. Do not introduce a second dependent inventory, image context, image tag discovery path, or results manifest.

# New Outputs

- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed Debian package outputs under `safe/out/deb/default/packages/`
- refreshed `safe/out/deb/default/stage-root/`
- rewritten `dependents.json`
- rewritten `safe/tests/dependents/dependent_matrix.toml`
- rewritten `safe/tests/dependents/src/rpm_probe.c`
- rewritten `safe/tests/dependents/src/zarchive_probe.c`
- rewritten `safe/tests/dependents/fixtures/rpm/hello.spec`
- rewritten `safe/tests/dependents/fixtures/rpm/hello.txt`
- rewritten `safe/tests/dependents/fixtures/zarchive/input/a.txt`
- rewritten `safe/tests/dependents/fixtures/zarchive/input/sub/b.txt`
- rewritten `safe/docker/dependents/Dockerfile`
- rewritten `safe/docker/dependents/entrypoint.sh`
- rewritten `safe/scripts/build-dependent-image.sh`
- rewritten `safe/scripts/run-dependent-matrix.sh`
- rewritten `safe/scripts/check-dependent-compile-compat.sh`
- rewritten `safe/scripts/verify-baseline-contract.sh`
- rewritten `test-original.sh`
- refreshed `safe/out/dependents/image-context/`
- rewritten `safe/out/dependents/image-context/metadata.env`
- refreshed `safe/out/dependents/compile-compat/`
- refreshed `safe/out/dependents/logs/`

# File Changes

- Freeze a 12-application dependent inventory in `dependents.json` and `safe/tests/dependents/dependent_matrix.toml`.
- Replace the inline `docker run ... bash <<EOF` body in `test-original.sh` with checked-in scripts and a checked-in image definition that installs safe `.deb` artifacts before tests run.
- Keep the current 10 applications: `apt`, `dpkg`, `rsync`, `systemd`, `libarchive`, `btrfs-progs`, `squashfs-tools`, `qemu`, `curl`, and `tiff`.
- Add exactly 2 fixed runtime consumers: `rpm` and `zarchive`.
- Materialize checked-in runtime fixtures for the 2 new dependents instead of generating their only inputs from shell heredocs.
- Remove downstream use of `SAFE_UPSTREAM_LIB`, repo bind mounts, host Cargo binds, and host Rust toolchain binds from the final image-based test flow.
- Run compile compatibility and runtime coverage inside the built image while the host only orchestrates image build/run and persists logs plus probe outputs under `safe/out/dependents/`.
- Pin one Ubuntu 24.04 image definition plus one canonical image-tag metadata file under `safe/out/dependents/image-context/`.
- Preserve inventory-consistency checks, safe-package version checks, safe-library-resolution checks, `btrfs` loop-device/mount coverage, and `systemd-journald` coverage.

# Implementation Details

- Preserve the existing 10 packages already curated in `dependents.json`.
- Add `rpm` in both inventories:
  - source package: `rpm`
  - binary package: `rpm`
  - compile probe: `safe/tests/dependents/src/rpm_probe.c`
  - runtime test: `test_rpm`
  - `compile_mode = "pkg-config-c"`
  - `pkg_config_modules = ["libzstd"]`
  - JSON evidence: Ubuntu Noble source package `rpm` Build-Depends on `libzstd-dev`.
  - Runtime path: `rpm`, `rpmbuild`, and `rpm2cpio` using the `librpmio9t64 -> libzstd1` dependency chain.
- Add `zarchive` in both inventories:
  - source package: `zarchive`
  - binary package: `zarchive-tools`
  - compile probe: `safe/tests/dependents/src/zarchive_probe.c`
  - runtime test: `test_zarchive`
  - `compile_mode = "pkg-config-c"`
  - `pkg_config_modules = ["libzstd"]`
  - JSON evidence: Ubuntu Noble source package `zarchive` Build-Depends-Arch on `libzstd-dev`.
  - Runtime path: `zarchive-tools` reaches `libzstd1` through `libzarchive0.1`.
- `safe/tests/dependents/src/rpm_probe.c` must follow the current minimal `pkg-config-c` pattern and exercise `ZSTD_compressBound`, `ZSTD_createCCtx`, `ZSTD_CCtx_setParameter(..., ZSTD_c_compressionLevel, 19)`, `ZSTD_compress2`, `ZSTD_createDCtx`, `ZSTD_decompressDCtx`, and matching free calls.
- `safe/tests/dependents/src/zarchive_probe.c` must follow the same pattern and exercise `ZSTD_CStreamInSize`, `ZSTD_CStreamOutSize`, `ZSTD_createCStream`, `ZSTD_createDStream`, `ZSTD_compressStream2`, `ZSTD_decompressStream`, and matching free calls.
- Keep checked-in RPM fixtures at `safe/tests/dependents/fixtures/rpm/hello.spec` and `safe/tests/dependents/fixtures/rpm/hello.txt` for a tiny noarch RPM whose payload compressor must be queried as `zstd`.
- Keep checked-in zarchive fixtures at `safe/tests/dependents/fixtures/zarchive/input/a.txt` and `safe/tests/dependents/fixtures/zarchive/input/sub/b.txt` for a create-then-extract round trip.
- `safe/docker/dependents/Dockerfile` must build from safe Debian artifacts already produced by Phase 4 and materialized under `safe/out/deb/default/`. It must use `FROM ubuntu:24.04`, or an equivalent `ARG` whose default is exactly `ubuntu:24.04`.
- The Dockerfile must install this exact existing downstream package set: `apt`, `apt-utils`, `btrfs-progs`, `build-essential`, `ca-certificates`, `cmake`, `curl`, `debhelper`, `devscripts`, `dh-package-notes`, `dpkg-dev`, `fakeroot`, `help2man`, `jq`, `libarchive-tools`, `liblz4-dev`, `liblzma-dev`, `libtiff-tools`, `less`, `pkgconf`, `python3`, `python3-pil`, `qemu-utils`, `rsync`, `squashfs-tools`, `systemd`, `zlib1g-dev`, and `zstd`.
- The Dockerfile must also install the exact new runtime packages `rpm`, `cpio`, `file`, and `zarchive-tools`.
- The image must bake in safe `.deb` artifacts, dependent fixtures, matrix metadata, and checked-in helper scripts so later `docker run` invocations do not require a repo mount. It must not need `cargo`, `rustc`, `HOST_CARGO_HOME`, or `HOST_RUSTUP_HOME`.
- Because `safe/scripts/build-deb.sh` hashes the full `safe/scripts/` tree and this phase rewrites scripts inside that tree, explicitly rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, and `bash safe/scripts/build-deb.sh` before image assembly.
- `safe/scripts/build-dependent-image.sh` must stage context under `safe/out/dependents/image-context/`, copy current safe `libzstd1`, `libzstd-dev`, and `zstd` `.deb` outputs plus `safe/out/deb/default/metadata.env`, `dependents.json`, `safe/tests/dependents/`, `safe/scripts/check-dependent-compile-compat.sh`, and `safe/docker/dependents/entrypoint.sh`, then run `docker build` from `safe/docker/dependents/Dockerfile`.
- `safe/scripts/build-dependent-image.sh` must write `safe/out/dependents/image-context/metadata.env` with one canonical local image tag/name and the Ubuntu 24.04 base image used for the matrix. Later phases must source that file instead of inventing alternate tags.
- `safe/docker/dependents/entrypoint.sh` must expose explicit `compile`, `runtime`, and `all` subcommands. `compile` invokes `safe/scripts/check-dependent-compile-compat.sh` inside the image. `runtime` dispatches per-application test functions.
- `safe/docker/dependents/entrypoint.sh` must keep the root filesystem writable and preserve the existing `/tmp/libzstd-dependent-tests` scratch-root pattern so runtime tests can create loopback images, mount them, and manage transient journald state under `/run`.
- `test_rpm` must assert that `/usr/bin/rpm`, `/usr/bin/rpmbuild`, and `/usr/bin/rpm2cpio` resolve `libzstd.so.1`; copy the checked-in RPM fixtures into `$TEST_ROOT/rpm/`; run `rpmbuild --define '_binary_payload w19.zstdio'`; assert `%{PAYLOADCOMPRESSOR}` is `zstd`; extract the resulting RPM with `rpm2cpio | cpio -idmu`; and compare the extracted `hello.txt` with the fixture source.
- `test_zarchive` must assert that `/usr/bin/zarchive` resolves `libzstd.so.1`; copy the checked-in zarchive fixture tree into `$TEST_ROOT/zarchive/in/`; run `zarchive "$dir/in" "$dir/archive.za"` and `zarchive "$dir/archive.za" "$dir/out"`; and `diff -ru` the input and output trees.
- `safe/scripts/run-dependent-matrix.sh` must be the only host-side executor for the downstream matrix. It must read `safe/out/dependents/image-context/metadata.env`, own image execution, in-image compile-compat execution, per-dependent runtime dispatch, and selective modes `--compile-only`, `--runtime-only`, and `--apps rpm,zarchive`.
- Runtime container invocation must use `docker run --rm --privileged`, must keep `/run` writable, and must bind-mount only host output directories for `safe/out/dependents/logs/` and `safe/out/dependents/compile-compat/`.
- `safe/scripts/check-dependent-compile-compat.sh` and `safe/scripts/verify-baseline-contract.sh` must be updated in place to expect all 12 source packages and runtime mappings `rpm -> test_rpm` and `zarchive -> test_zarchive`, without re-shifting Phase 1 ownership metadata.
- `safe/scripts/check-dependent-compile-compat.sh` must assume it is running inside the dependent image after safe packages are installed there; host-side verifiers must go through `safe/scripts/run-dependent-matrix.sh` or `test-original.sh`.
- `safe/docker/dependents/entrypoint.sh` and `test-original.sh` must preserve inventory consistency, installed safe-package version equality and `safelibs` suffix checks for `libzstd1`, `libzstd-dev`, and `zstd`, plus `assert_uses_safe_lib` checks before app-specific runtime coverage.
- `test-original.sh` must become the stable top-level wrapper that validates inventory consistency, ensures existing safe Debian artifacts exist, delegates to `safe/scripts/build-dependent-image.sh` and `safe/scripts/run-dependent-matrix.sh`, and no longer exports `SAFE_UPSTREAM_LIB` or mounts the repo/toolchain into runtime containers.

# Verification Phases

- Phase ID: `script_dependent_image_matrix`
  - Type: `check`
  - `bounce_target`: `impl_dependent_image_matrix`
  - Purpose: build the checked-in image, install safe Debian packages in it, compile dependent probes, and exercise the current runtime matrix.
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
  - Purpose: review dependent selection, compile-probe quality, exact package installation, and runtime-test determinism.
  - Commands: none; perform dependent inventory, fixture, package-list, runtime-command, and evidence review.
- Phase ID: `check_dependent_image_matrix_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_dependent_image_matrix`
  - Purpose: review that the workflow now uses a reproducible checked-in image instead of an inline container bootstrap.
  - Commands: none; perform senior downstream topology, artifact-root, and container-contract review.

# Success Criteria

- The dependent matrix contains exactly the original 10 dependents plus `rpm` and `zarchive`.
- The downstream workflow uses one checked-in Ubuntu 24.04 image definition and one canonical image metadata file.
- Compile and runtime coverage execute inside the built image, with preserved `--privileged` and writable `/run` runtime contract.
- Runtime containers do not mount the repo, Cargo, Rustup, or upstream `libzstd`.
- `test_rpm` and `test_zarchive` implement the exact runtime commands described above.
- `test-original.sh` delegates to the checked-in image workflow and succeeds.
- All listed verifier commands pass or any failure is fixed before yielding.

# Git Commit Requirement

The implementer must commit the Phase 6 work to git before yielding. That commit must exist before any verifier phase for `impl_dependent_image_matrix` runs.
