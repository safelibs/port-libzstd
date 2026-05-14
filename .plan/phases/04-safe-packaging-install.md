# Phase Name

Packaging, Install Layout, and Drop-In Artifact Flow

# Implement Phase ID

`impl_safe_packaging_install`

# Preexisting Inputs

- `safe/build.rs`
- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
- `safe/src/ffi/compress.rs`
- `safe/src/ffi/advanced.rs`
- `safe/src/compress/cctx_params.rs`
- `safe/src/compress/cdict.rs`
- `safe/src/compress/sequence_api.rs`
- `safe/src/compress/static_ctx.rs`
- `safe/src/threading/job_queue.rs`
- `safe/src/threading/pool.rs`
- `safe/src/threading/zstdmt.rs`
- `safe/src/dict_builder/cover.rs`
- `safe/src/dict_builder/divsufsort.rs`
- `safe/src/dict_builder/fastcover.rs`
- `safe/src/dict_builder/zdict.rs`
- `safe/tests/capi/dict_builder_driver.c`
- `safe/tests/capi/sequence_api_driver.c`
- `safe/tests/capi/thread_pool_driver.c`
- `safe/tests/link-compat/Makefile`
- `safe/scripts/run-advanced-mt-tests.sh`
- `safe/scripts/verify-link-compat.sh`
- `safe/docs/unsafe-audit.md`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/install-safe-debs.sh`
- `safe/scripts/verify-install-layout.sh`
- `safe/scripts/verify-deb-profiles.sh`
- `safe/scripts/run-debian-autopkgtests.sh`
- `safe/scripts/run-build-variant-tests.sh`
- `safe/debian/changelog`
- `safe/debian/clean`
- `safe/debian/control`
- `safe/debian/compat`
- `safe/debian/copyright`
- `safe/debian/rules`
- `safe/debian/source/format`
- `safe/debian/libzstd-dev.examples`
- `safe/debian/libzstd-dev.install`
- `safe/debian/libzstd1.install`
- `safe/debian/zstd.install`
- `safe/debian/zstd.docs`
- `safe/debian/zstd.manpages`
- `safe/debian/tests/README.md`
- `safe/debian/tests/control`
- `safe/debian/tests/python/check_build/*`
- `safe/debian/tests/requirements/install.txt`
- `safe/debian/tests/requirements/tests.txt`
- `safe/debian/tests/ztest/programs.toml`
- `safe/debian/tests/ztest/cmake/*`
- `safe/debian/tests/ztest/pkg-make/*`
- `original/libzstd-1.5.5+dfsg2/programs/*`
- `original/libzstd-1.5.5+dfsg2/zlibWrapper/*`
- `original/libzstd-1.5.5+dfsg2/examples/*`
- `original/libzstd-1.5.5+dfsg2/doc/educational_decoder/*`
- `original/libzstd-1.5.5+dfsg2/contrib/pzstd/*`
- `original/libzstd-1.5.5+dfsg2/CHANGELOG`
- `original/libzstd-1.5.5+dfsg2/CODE_OF_CONDUCT.md`
- `original/libzstd-1.5.5+dfsg2/CONTRIBUTING.md`
- `original/libzstd-1.5.5+dfsg2/COPYING`
- `original/libzstd-1.5.5+dfsg2/LICENSE`
- `original/libzstd-1.5.5+dfsg2/README.md`
- `original/libzstd-1.5.5+dfsg2/TESTING.md`

# New Outputs

- rewritten `safe/scripts/build-artifacts.sh`
- rewritten `safe/scripts/build-deb.sh`
- rewritten `safe/scripts/build-original-cli-against-safe.sh`
- rewritten `safe/scripts/install-safe-debs.sh`
- rewritten `safe/scripts/verify-install-layout.sh`
- rewritten `safe/scripts/verify-deb-profiles.sh`
- rewritten `safe/scripts/run-debian-autopkgtests.sh`
- rewritten `safe/scripts/run-build-variant-tests.sh`
- rewritten `safe/debian/changelog`
- rewritten `safe/debian/clean`
- rewritten `safe/debian/control`
- rewritten `safe/debian/copyright`
- rewritten `safe/debian/rules`
- rewritten `safe/debian/libzstd-dev.examples`
- rewritten `safe/debian/libzstd-dev.install`
- rewritten `safe/debian/libzstd1.install`
- rewritten `safe/debian/zstd.install`
- rewritten `safe/debian/zstd.docs`
- rewritten `safe/debian/zstd.manpages`
- rewritten `safe/debian/tests/README.md`
- rewritten `safe/debian/tests/control`
- rewritten `safe/debian/tests/python/check_build/*`
- rewritten `safe/debian/tests/requirements/install.txt`
- rewritten `safe/debian/tests/requirements/tests.txt`
- rewritten `safe/debian/tests/ztest/programs.toml`
- rewritten `safe/debian/tests/ztest/cmake/*`
- rewritten `safe/debian/tests/ztest/pkg-make/*`
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed `safe/out/deb/default/packages/*.deb`
- refreshed when SAFE_ENABLE_UDEB=1 `safe/out/deb/default/packages/*.udeb`
- refreshed `safe/out/deb/default/stage-root/`

# File Changes

- Keep the existing package names and install layout.
- Ensure build signatures and reuse logic still work after library internals move fully into Rust.
- Ensure CLI and helper-tree build steps link against safe artifacts only.
- Make Phase 4 establish the only canonical producer scripts and artifact roots for the install tree, helper root, staged-source tree, and package outputs that later phases may refresh only in place through the same scripts and paths.

# Implementation Details

- `safe/scripts/build-artifacts.sh` must continue to emit the default, `mt`, and `nomt` variants, with the default shared object multithreaded and the default static archive single-threaded, matching upstream contract.
- `safe/scripts/build-original-cli-against-safe.sh` may continue to use upstream CLI/program sources, but the library and headers that those sources see must come from the safe artifact tree, not from upstream `libzstd`.
- The Phase 4 script verifier must run the three canonical producer scripts in order: `build-artifacts.sh --release`, `build-original-cli-against-safe.sh`, and `build-deb.sh`.
- `safe/out/original-cli/lib/` must be left populated with safe headers plus the helper `libzstd.so*` and `libzstd.a` indirection files that preserved upstream wrappers expect.
- `safe/scripts/build-deb.sh` must keep staging `programs/`, `zlibWrapper/`, `examples/`, `contrib/pzstd/`, `doc/educational_decoder/`, and the upstream top-level doc files because checked-in Debian metadata already references those paths.
- `safe/debian/libzstd-dev.examples` must continue to ship `examples/*`; `safe/debian/zstd.docs` must continue to ship `CHANGELOG` and `*.md`; `safe/debian/zstd.manpages` must continue to install `usr/share/man/man1/*`; and `safe/debian/rules` must keep `execute_after_dh_installman` generating `zstdmt.1` and `pzstd.1` from safe-built binaries.
- `safe/debian/tests/control`, `safe/debian/tests/python/check_build/*`, `safe/debian/tests/requirements/*`, and `safe/debian/tests/ztest/*` must stay fully safe-rooted and checked in.
- Phase 4 establishes `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`, and `safe/out/deb/default/` as the only canonical safe build/package roots.

# Verification Phases

- Phase ID: `script_safe_packaging_install`
- Type: `check`
- `bounce_target`: `impl_safe_packaging_install`
- Purpose: verify build variants, install layout, Debian profiles, and Debian autopkgtests using only safe-built library artifacts.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - `bash safe/scripts/build-artifacts.sh --release`
  - `bash safe/scripts/build-original-cli-against-safe.sh`
  - `bash safe/scripts/build-deb.sh`
  - `bash safe/scripts/run-build-variant-tests.sh`
  - `bash safe/scripts/verify-install-layout.sh`
  - `bash safe/scripts/verify-install-layout.sh --debian`
  - `bash safe/scripts/verify-deb-profiles.sh`
  - `bash safe/scripts/run-debian-autopkgtests.sh`

- Phase ID: `check_safe_packaging_software_tester`
- Type: `check`
- `bounce_target`: `impl_safe_packaging_install`
- Purpose: review package contents, install layout, Debian test coverage, and cache/rebuild behavior.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

- Phase ID: `check_safe_packaging_senior_tester`
- Type: `check`
- `bounce_target`: `impl_safe_packaging_install`
- Purpose: review Ubuntu/Debian drop-in compatibility and confirm that the package outputs do not smuggle the old helper-library model back in.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

# Success Criteria

- Canonical Phase 4 roots exist and are produced through the canonical scripts.
- Packages preserve upstream names, docs, examples, manpages, pkg-config metadata, CMake metadata, and autopkgtests.
- Build variants, install layout, Debian profile outputs, and autopkgtests pass against safe artifacts.
- No package path reintroduces an upstream helper-library dependency.

# Git Commit Requirement

The implementer must commit all Phase 4 work to git before yielding. That commit must exist before `script_safe_packaging_install`, `check_safe_packaging_software_tester`, and `check_safe_packaging_senior_tester` run.
