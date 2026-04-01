# Dependent Inventory Expansion and Reproducible Image

## Workflow Position
Phase 6 of 8 in the linear explicit-phase workflow.

## Phase Name
Dependent Inventory Expansion and Reproducible Image

## Implement Phase ID
`impl_dependent_image_matrix`

## Preexisting Inputs
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
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
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

## New Outputs
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed `safe/out/deb/default/packages/`
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

## File Changes
- Freeze a 12-application dependent inventory in `dependents.json` and `safe/tests/dependents/dependent_matrix.toml`.
- Replace the inline `docker run ... bash <<EOF` body in `test-original.sh` with checked-in scripts and a checked-in image definition that installs the safe `.deb` artifacts before tests run.
- Keep the current 10 applications and add exactly 2 more fixed runtime consumers: `rpm` and `zarchive`.
- Materialize checked-in runtime fixtures for the 2 new dependents instead of generating their only inputs from shell heredocs.
- Remove downstream use of `SAFE_UPSTREAM_LIB`, repo bind mounts, host Cargo binds, and host Rust toolchain binds from the final image-based test flow.
- Fix the downstream execution topology so compile compatibility and runtime coverage both execute inside the built image, while the host only orchestrates image build/run and persists logs plus probe outputs under `safe/out/dependents/`.
- Pin a single Ubuntu 24.04 image definition plus a single canonical image-tag metadata file under `safe/out/dependents/image-context/` so later phases reuse the same image identity instead of inventing new tags.
- Preserve the current inventory-consistency checks, safe-package version checks, and safe-library-resolution checks while moving them into the checked-in image workflow.
- Limit the Phase 6 `safe/scripts/verify-baseline-contract.sh` edit to extending the dependent-inventory and runtime-test assertions from 10 applications to 12; do not re-shift or renumber the fixed rebased `owning_phase` table or workflow numbering established in Phase 1.
- The downstream runtime tests must validate the installed safe Debian packages inside the image, not only build-tree artifacts via `LD_LIBRARY_PATH`.

## Implementation Details
- Preserve the existing 10 packages already curated in `dependents.json`.
- Add these exact 2 new dependent entries and keep them in both `dependents.json` and `safe/tests/dependents/dependent_matrix.toml`:
  - source package `rpm`, binary package `rpm`, compile probe `safe/tests/dependents/src/rpm_probe.c`, runtime test `test_rpm`, `compile_mode = "pkg-config-c"`, and `pkg_config_modules = ["libzstd"]`
  - source package `zarchive`, binary package `zarchive-tools`, compile probe `safe/tests/dependents/src/zarchive_probe.c`, runtime test `test_zarchive`, `compile_mode = "pkg-config-c"`, and `pkg_config_modules = ["libzstd"]`
- The `dependents.json` entries for `rpm` and `zarchive` must record the Ubuntu Noble build/runtime evidence from the source plan: `rpm` Build-Depends on `libzstd-dev` and reaches `libzstd1` through `librpmio9t64`, while `zarchive` Build-Depends-Arch on `libzstd-dev` and `zarchive-tools` reaches `libzstd1` through `libzarchive0.1`.
- `safe/tests/dependents/src/rpm_probe.c` must follow the current minimal `pkg-config-c` pattern and reference `ZSTD_compressBound`, `ZSTD_createCCtx`, `ZSTD_CCtx_setParameter(..., ZSTD_c_compressionLevel, 19)`, `ZSTD_compress2`, `ZSTD_createDCtx`, `ZSTD_decompressDCtx`, and the matching free calls.
- `safe/tests/dependents/src/zarchive_probe.c` must follow the same pattern but exercise `ZSTD_CStreamInSize`, `ZSTD_CStreamOutSize`, `ZSTD_createCStream`, `ZSTD_createDStream`, `ZSTD_compressStream2`, `ZSTD_decompressStream`, and the matching free calls.
- Add checked-in runtime fixtures `safe/tests/dependents/fixtures/rpm/hello.spec`, `safe/tests/dependents/fixtures/rpm/hello.txt`, `safe/tests/dependents/fixtures/zarchive/input/a.txt`, and `safe/tests/dependents/fixtures/zarchive/input/sub/b.txt`.
- `safe/docker/dependents/Dockerfile` must build the reproducible downstream image from the safe Debian artifacts already produced by Phase 4 and materialized under `safe/out/deb/default/`. It must use Ubuntu 24.04 as the fixed base, either as `FROM ubuntu:24.04` or as an equivalent `ARG` whose default is exactly `ubuntu:24.04`. It must install the existing downstream package set from `test-original.sh`, plus `rpm`, `cpio`, `file`, and `zarchive-tools`, and it must not require `cargo`, `rustc`, `HOST_CARGO_HOME`, or `HOST_RUSTUP_HOME`.
- Because `safe/scripts/build-deb.sh` hashes the full `safe/scripts/` tree and this phase rewrites scripts inside that tree, this phase must explicitly rerun the canonical Phase 4 refresh sequence before image assembly: `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, and `bash safe/scripts/build-deb.sh`.
- `safe/scripts/build-dependent-image.sh` must stage an image context under `safe/out/dependents/` that copies in the current safe `.deb` outputs plus `safe/out/deb/default/metadata.env`, `dependents.json`, `safe/tests/dependents/`, `safe/scripts/check-dependent-compile-compat.sh`, and `safe/docker/dependents/entrypoint.sh`, then `docker build` from `safe/docker/dependents/Dockerfile`. It must write `safe/out/dependents/image-context/metadata.env` with the single canonical local image tag or name and the base image default used for the matrix.
- `safe/docker/dependents/entrypoint.sh` must define the full runtime matrix formerly embedded in `test-original.sh`, including the new `test_rpm` and `test_zarchive` functions. It must expose explicit subcommands for `compile`, `runtime`, and `all`, where `compile` invokes `safe/scripts/check-dependent-compile-compat.sh` inside the image and `runtime` dispatches the per-application test functions. It must keep the container root filesystem writable and preserve the existing `/tmp/libzstd-dependent-tests` scratch-root pattern so the `btrfs` and `systemd-journald` runtime paths remain covered.
- `safe/docker/dependents/entrypoint.sh` must implement `test_rpm` exactly: assert that `/usr/bin/rpm`, `/usr/bin/rpmbuild`, and `/usr/bin/rpm2cpio` resolve `libzstd.so.1`; copy the checked-in RPM fixtures into `$TEST_ROOT/rpm/`; run `rpmbuild --define '_binary_payload w19.zstdio'`; assert `%{PAYLOADCOMPRESSOR}` is `zstd`; extract the resulting RPM with `rpm2cpio | cpio -idmu`; and compare the extracted `hello.txt` with the fixture source.
- `safe/docker/dependents/entrypoint.sh` must implement `test_zarchive` exactly: assert that `/usr/bin/zarchive` resolves `libzstd.so.1`; copy the checked-in zarchive fixture tree into `$TEST_ROOT/zarchive/in/`; run `zarchive "$dir/in" "$dir/archive.za"` and `zarchive "$dir/archive.za" "$dir/out"`; and `diff -ru` the input and output trees.
- `safe/scripts/run-dependent-matrix.sh` must be the only host-side executor for the downstream matrix. It must read `safe/out/dependents/image-context/metadata.env`, own image execution, in-image compile-compat execution, per-dependent runtime dispatch, and selective modes such as `--compile-only`, `--runtime-only`, and `--apps rpm,zarchive`. Its runtime container invocation must use `docker run --rm --privileged`, keep `/run` writable, and bind-mount only `safe/out/dependents/logs/` and `safe/out/dependents/compile-compat/`.
- `safe/scripts/check-dependent-compile-compat.sh` is an in-image helper only; host-side verifiers must go through `safe/scripts/run-dependent-matrix.sh` or `test-original.sh`.
- `safe/scripts/check-dependent-compile-compat.sh` must be updated in place to expect all 12 source packages and the new runtime mappings `rpm -> test_rpm` and `zarchive -> test_zarchive`.
- `safe/scripts/verify-baseline-contract.sh` may be updated in Phase 6 only to extend the dependent-inventory and runtime-test assertions from 10 applications to 12 applications, including `rpm -> test_rpm` and `zarchive -> test_zarchive`; it must keep the fixed Phase 1 rebased `owning_phase` checks unchanged.
- `safe/docker/dependents/entrypoint.sh` and `test-original.sh` must preserve the current inventory-consistency check, the installed safe-package version equality and `safelibs` suffix checks for `libzstd1`, `libzstd-dev`, and `zstd`, and the `assert_uses_safe_lib` checks before any app-specific runtime coverage runs.
- `test-original.sh` must become the stable top-level wrapper that validates inventory consistency, ensures the existing safe Debian artifacts exist, delegates to `safe/scripts/build-dependent-image.sh` and `safe/scripts/run-dependent-matrix.sh`, and no longer exports `SAFE_UPSTREAM_LIB` or mounts the repo or toolchain into runtime containers.
- Phase 6 defines `safe/out/dependents/image-context/`, `safe/out/dependents/compile-compat/`, and `safe/out/dependents/logs/` as the only canonical downstream-image roots, with `safe/out/dependents/image-context/metadata.env` as the only canonical downstream-image metadata file.

## Verification Phases
- `script_dependent_image_matrix` | type: `script` | `bounce_target: impl_dependent_image_matrix` | purpose: build the checked-in image, install safe Debian packages in it, compile dependent probes, and exercise the current runtime matrix.
- `check_dependent_image_matrix_software_tester` | type: `check` | `bounce_target: impl_dependent_image_matrix` | purpose: review dependent selection, compile-probe quality, and runtime-test determinism.
- `check_dependent_image_matrix_senior_tester` | type: `check` | `bounce_target: impl_dependent_image_matrix` | purpose: review that the workflow now uses a reproducible checked-in image instead of an inline container bootstrap.

## Verification Commands
- `bash safe/scripts/build-artifacts.sh --release`
- `bash safe/scripts/build-original-cli-against-safe.sh`
- `bash safe/scripts/build-deb.sh`
- `bash safe/scripts/build-dependent-image.sh`
- `bash safe/scripts/run-dependent-matrix.sh --compile-only`
- `bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps btrfs-progs,systemd`
- `bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps rpm,zarchive`
- `bash safe/scripts/verify-baseline-contract.sh`
- `bash test-original.sh`

## Success Criteria
- The downstream matrix is frozen to 12 applications, with `rpm` and `zarchive` added in both dependent inventories and backed by checked-in compile probes plus runtime fixtures.
- The inline container bootstrap is gone: the checked-in Dockerfile, entrypoint, image-build script, matrix runner, and `test-original.sh` all reuse the canonical Phase 4 `.deb` outputs and the single `safe/out/dependents/image-context/metadata.env` image identity.
- Compile compatibility runs inside the image, runtime coverage runs with the preserved `--privileged` plus writable-`/run` contract, and the downstream runtime tests validate installed safe Debian packages inside the image rather than only build-tree artifacts via `LD_LIBRARY_PATH`.
- The listed image-matrix verification commands pass without `SAFE_UPSTREAM_LIB`, repo mounts, host toolchain mounts, alternate image roots, or alternate bootstrap logic.

## Git Commit Requirement
The implementer must commit all work for this phase to git before yielding to the verifier phases.
