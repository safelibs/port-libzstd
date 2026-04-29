# Phase Name

Dependent Regressions and Compatibility Fixes

# Implement Phase ID

`impl_compat_regressions_and_fixes`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- All outputs from `impl_dependent_image_matrix`, including `dependents.json`, `safe/tests/dependents/dependent_matrix.toml`, `safe/tests/dependents/src/rpm_probe.c`, `safe/tests/dependents/src/zarchive_probe.c`, `safe/tests/dependents/fixtures/rpm/`, `safe/tests/dependents/fixtures/zarchive/`, `safe/docker/dependents/`, `safe/scripts/build-dependent-image.sh`, `safe/scripts/run-dependent-matrix.sh`, `safe/scripts/check-dependent-compile-compat.sh`, `safe/scripts/verify-baseline-contract.sh`, `test-original.sh`, `safe/out/dependents/image-context/metadata.env`, `safe/out/dependents/compile-compat/`, and `safe/out/dependents/logs/`
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
- `safe/out/dependents/image-context/metadata.env`
- `safe/out/dependents/compile-compat/`
- `safe/out/dependents/logs/`
- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
- `safe/tests/dependents/dependent_matrix.toml`
- `dependents.json`
- `test-original.sh`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `safe/src/decompress/dctx.rs`
- `safe/src/decompress/ddict.rs`
- `safe/src/decompress/dstream.rs`
- `safe/src/decompress/frame.rs`
- `safe/src/decompress/huf.rs`
- `safe/src/decompress/fse.rs`
- `safe/src/compress/block.rs`
- `safe/src/compress/cctx.rs`
- `safe/src/compress/cstream.rs`
- `safe/src/compress/params.rs`
- `safe/src/compress/frame.rs`
- `safe/src/compress/literals.rs`
- `safe/src/compress/ldm.rs`
- `safe/src/compress/match_state.rs`
- `safe/src/compress/sequences.rs`
- `safe/src/compress/cctx_params.rs`
- `safe/src/compress/cdict.rs`
- `safe/src/compress/sequence_api.rs`
- `safe/src/compress/static_ctx.rs`
- `safe/src/ffi/compress.rs`
- `safe/src/ffi/decompress.rs`
- `safe/src/ffi/advanced.rs`
- `safe/src/threading/job_queue.rs`
- `safe/src/threading/pool.rs`
- `safe/src/threading/zstdmt.rs`
- `safe/src/dict_builder/cover.rs`
- `safe/src/dict_builder/divsufsort.rs`
- `safe/src/dict_builder/fastcover.rs`
- `safe/src/dict_builder/zdict.rs`
- `safe/tests/rust/compress.rs`
- `safe/tests/rust/decompress.rs`
- `safe/tests/capi/decompress_smoke.c`
- `safe/tests/capi/frame_probe.c`
- `safe/tests/capi/legacy_decode.c`
- `safe/tests/capi/roundtrip_smoke.c`
- `safe/tests/capi/bigdict_driver.c`
- `safe/tests/capi/invalid_dictionaries_driver.c`
- `safe/tests/capi/zstream_driver.c`
- `safe/tests/capi/paramgrill_driver.c`
- `safe/tests/capi/external_matchfinder_driver.c`
- `safe/tests/capi/dict_builder_driver.c`
- `safe/tests/capi/sequence_api_driver.c`
- `safe/tests/capi/thread_pool_driver.c`
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
- `safe/tests/dependents/src/rpm_probe.c`
- `safe/tests/dependents/src/zarchive_probe.c`
- `safe/docker/dependents/Dockerfile`
- `safe/docker/dependents/entrypoint.sh`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/build-dependent-image.sh`
- `safe/scripts/run-dependent-matrix.sh`
- `safe/scripts/check-dependent-compile-compat.sh`
- `safe/scripts/run-capi-decompression.sh`
- `safe/scripts/run-capi-roundtrip.sh`
- `safe/scripts/run-advanced-mt-tests.sh`
- `safe/scripts/run-upstream-tests.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/docs/unsafe-audit.md`

Existing Phase 6 logs and artifacts are the first triage inputs. Do not create a second downstream results manifest or broaden the 12-application inventory.

# New Outputs

- rewritten library/source files in the specific safe modules implicated by downstream failures
- new or rewritten checked-in regressions under the closest existing harness:
  - `safe/tests/rust/`
  - `safe/tests/capi/`
  - `safe/tests/dependents/src/`
  - `safe/tests/dependents/regressions/`
  - `safe/scripts/`
- rewritten `safe/docs/unsafe-audit.md`
- rewritten `safe/tests/upstream_test_matrix.toml` if coverage metadata changes
- rewritten `safe/tests/dependents/dependent_matrix.toml` if coverage metadata changes
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed Debian package outputs under `safe/out/deb/default/packages/`
- refreshed `safe/out/deb/default/stage-root/`
- refreshed `safe/out/dependents/image-context/`
- refreshed `safe/out/dependents/compile-compat/`
- refreshed `safe/out/dependents/logs/`

# File Changes

- Add a reproducer before fixing every newly discovered compatibility issue.
- Keep each reproducer as close as possible to the failing surface rather than creating a disconnected ad hoc test location.
- Update the downstream image/runtime harness only when the failure is in the harness, not when the failure is in library behavior.
- Preserve the fixed 12-application inventory from Phase 6.

# Implementation Details

- Triage failures from existing Phase 6 logs and artifacts under `safe/out/dependents/` first.
- Route ABI or pointer-semantics bugs to `safe/tests/capi/`.
- Route algorithmic or Rust API bugs to `safe/tests/rust/`.
- Route packaging or CLI integration bugs to `safe/scripts/` or Debian tests.
- Route app-specific failures to `safe/tests/dependents/`.
- If this phase edits any file hashed or staged by canonical Phase 4 or Phase 6 producers, explicitly rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, `bash safe/scripts/build-deb.sh`, and `bash safe/scripts/build-dependent-image.sh` before downstream verifiers.
- Reuse existing Phase 4 and Phase 6 paths. Do not create alternate output roots, an alternate image tag flow, or an extra results manifest.
- If a harness fix refreshes the staged downstream image context, update `safe/out/dependents/` in place through existing scripts. Do not reintroduce inline `docker run ... bash <<EOF`, repo mounts, host toolchain binds, or a host-side compile-compat path outside the image.
- Update `safe/docs/unsafe-audit.md` whenever a fix changes where `unsafe` is used or why it remains.
- Re-run affected upstream suites after each fix so dependent fixes do not silently regress the broader black-box surface.

# Verification Phases

- Phase ID: `script_compat_regressions_and_fixes`
  - Type: `check`
  - `bounce_target`: `impl_compat_regressions_and_fixes`
  - Purpose: run the full 12-application runtime matrix, execute the new regressions, and validate the fixes.
  - Commands:
    - `bash safe/scripts/build-artifacts.sh --release`
    - `bash safe/scripts/build-original-cli-against-safe.sh`
    - `bash safe/scripts/build-deb.sh`
    - `bash safe/scripts/build-dependent-image.sh`
    - `bash safe/scripts/run-dependent-matrix.sh --compile-only`
    - `bash safe/scripts/run-dependent-matrix.sh --runtime-only`
    - `bash test-original.sh`
    - run the exact new regression tests added in this phase
    - rerun whichever upstream suites exercise the touched surface before yielding
- Phase ID: `check_compat_regressions_software_tester`
  - Type: `check`
  - `bounce_target`: `impl_compat_regressions_and_fixes`
  - Purpose: review that every discovered bug gained a reproducer before being fixed.
  - Commands: none; perform regression-first and evidence review.
- Phase ID: `check_compat_regressions_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_compat_regressions_and_fixes`
  - Purpose: review the appropriateness of each fix, the minimality of the unsafe surface, and any cross-suite regression risk.
  - Commands: none; perform senior fix, unsafe, and cross-suite risk review.

# Success Criteria

- Every discovered compatibility failure has a checked-in reproducer before its fix.
- Fixes are placed in the narrowest appropriate safe-side module or harness.
- The full 12-application compile/runtime matrix passes after any required artifact refresh.
- Related upstream suites are rerun for touched surfaces.
- No alternate downstream image, tag flow, artifact root, or results manifest is introduced.

# Git Commit Requirement

The implementer must commit the Phase 7 work to git before yielding. That commit must exist before any verifier phase for `impl_compat_regressions_and_fixes` runs.
