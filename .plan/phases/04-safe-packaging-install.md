# Packaging, Install Layout, and Drop-In Artifact Flow

## Phase Name
Packaging, Install Layout, and Drop-In Artifact Flow

## Implement Phase ID
`impl_safe_packaging_install`

## Preexisting Inputs
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
- `safe/debian/tests/python/check_build/`
- `safe/debian/tests/requirements/install.txt`
- `safe/debian/tests/requirements/tests.txt`
- `safe/debian/tests/ztest/programs.toml`
- `safe/debian/tests/ztest/cmake/`
- `safe/debian/tests/ztest/pkg-make/`
- `original/libzstd-1.5.5+dfsg2/programs/`
- `original/libzstd-1.5.5+dfsg2/zlibWrapper/`
- `original/libzstd-1.5.5+dfsg2/examples/`
- `original/libzstd-1.5.5+dfsg2/doc/educational_decoder/`
- `original/libzstd-1.5.5+dfsg2/contrib/pzstd/`
- `original/libzstd-1.5.5+dfsg2/CHANGELOG`
- `original/libzstd-1.5.5+dfsg2/CODE_OF_CONDUCT.md`
- `original/libzstd-1.5.5+dfsg2/CONTRIBUTING.md`
- `original/libzstd-1.5.5+dfsg2/COPYING`
- `original/libzstd-1.5.5+dfsg2/LICENSE`
- `original/libzstd-1.5.5+dfsg2/README.md`
- `original/libzstd-1.5.5+dfsg2/TESTING.md`

## New Outputs
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
- rewritten `safe/debian/tests/python/check_build/`
- rewritten `safe/debian/tests/requirements/install.txt`
- rewritten `safe/debian/tests/requirements/tests.txt`
- rewritten `safe/debian/tests/ztest/programs.toml`
- rewritten `safe/debian/tests/ztest/cmake/`
- rewritten `safe/debian/tests/ztest/pkg-make/`
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed `safe/out/deb/default/packages/`
- refreshed `safe/out/deb/default/stage-root/`

## File Changes
- Keep the existing package names and install layout.
- Ensure build signatures and reuse logic still work after library internals move fully into Rust.
- Ensure CLI and helper-tree build steps link against safe artifacts only.
- Make Phase 4 establish the only canonical producer scripts and artifact roots for the install tree, helper root, staged-source tree, and package outputs that later phases may refresh only in place through the same scripts and paths.

## Implementation Details
- `safe/scripts/build-artifacts.sh` must continue to emit the default, `mt`, and `nomt` variants, with the default shared object multithreaded and the default static archive single-threaded, matching upstream contract.
- `safe/scripts/build-original-cli-against-safe.sh` may continue to use upstream CLI/program sources, but the library and headers that those sources see must come from the safe artifact tree, not from upstream `libzstd`.
- The Phase 4 script verifier must run the three canonical producer scripts in order, `build-artifacts.sh --release`, `build-original-cli-against-safe.sh`, and `build-deb.sh`, so all four canonical roots exist before the layout, profile, and autopkgtest verifiers run.
- `safe/out/original-cli/lib/` must be left populated with the safe headers plus the helper `libzstd.so*` and `libzstd.a` indirection files that the preserved upstream wrappers expect; Phase 4 does not leave that helper root implicit or verifier-optional.
- `safe/scripts/build-deb.sh` must keep staging `programs/`, `zlibWrapper/`, `examples/`, `contrib/pzstd/`, `doc/educational_decoder/`, and the upstream top-level doc files because the checked-in Debian metadata already references those paths.
- `safe/debian/libzstd-dev.examples` must continue to ship `examples/*`; `safe/debian/zstd.docs` must continue to ship `CHANGELOG` and `*.md`; `safe/debian/zstd.manpages` must continue to install `usr/share/man/man1/*`; and `safe/debian/rules` must keep `execute_after_dh_installman` generating `zstdmt.1` and `pzstd.1` from the safe-built binaries.
- `safe/debian/tests/control`, `safe/debian/tests/python/check_build/*`, `safe/debian/tests/requirements/*`, and `safe/debian/tests/ztest/*` must stay fully safe-rooted and checked in. `safe/scripts/run-debian-autopkgtests.sh` must keep using the safe-staged Debian tree, verify that `debian/tests/` no longer points back into `../original`, and keep the three upstream autopkgtest identities from `original/libzstd-1.5.5+dfsg2/debian/tests/control`.
- Phase 4 establishes `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`, and `safe/out/deb/default/` as the only canonical safe build/package roots. Later phases may refresh those same roots only by explicitly rerunning `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, and `bash safe/scripts/build-deb.sh` before their verifiers when they edit inputs covered by those producers.

## Verification Phases
- `script_safe_packaging_install` | type: `script` | `bounce_target: impl_safe_packaging_install` | purpose: verify build variants, install layout, Debian profiles, and Debian autopkgtests using only safe-built library artifacts.
- `check_safe_packaging_software_tester` | type: `check` | `bounce_target: impl_safe_packaging_install` | purpose: review package contents, install layout, Debian test coverage, and cache/rebuild behavior.
- `check_safe_packaging_senior_tester` | type: `check` | `bounce_target: impl_safe_packaging_install` | purpose: review Ubuntu/Debian drop-in compatibility and confirm that the package outputs do not smuggle the old helper-library model back in.

## Verification Commands
- `bash safe/scripts/build-artifacts.sh --release`
- `bash safe/scripts/build-original-cli-against-safe.sh`
- `bash safe/scripts/build-deb.sh`
- `bash safe/scripts/run-build-variant-tests.sh`
- `bash safe/scripts/verify-install-layout.sh`
- `bash safe/scripts/verify-install-layout.sh --debian`
- `bash safe/scripts/verify-deb-profiles.sh`
- `bash safe/scripts/run-debian-autopkgtests.sh`

## Success Criteria
- Package names, install layout, docs, manpages, examples, pkg-config metadata, CMake metadata, and autopkgtests remain drop-in compatible with the Ubuntu/Debian contract already modeled in `safe/debian/`.
- The canonical Phase 4 producer scripts are the only supported way to refresh `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`, and `safe/out/deb/default/`.
- The listed build, layout, profile, and autopkgtest verification commands pass against safe-built artifacts only.

## Git Commit Requirement
The implementer must commit all work for this phase to git before yielding to the verifier phases.
