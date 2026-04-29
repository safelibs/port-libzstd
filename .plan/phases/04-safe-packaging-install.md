# Phase Name

Packaging, Install Layout, and Drop-In Artifact Flow

# Implement Phase ID

`impl_safe_packaging_install`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- All outputs from `impl_safe_advanced_abi_completion`, including `safe/build.rs`, `safe/abi/export_map.toml`, `safe/tests/upstream_test_matrix.toml`, `safe/src/ffi/`, `safe/src/compress/`, `safe/src/threading/`, `safe/src/dict_builder/`, `safe/tests/capi/`, `safe/tests/link-compat/`, `safe/scripts/run-advanced-mt-tests.sh`, `safe/scripts/verify-link-compat.sh`, and `safe/docs/unsafe-audit.md`
- `safe/Cargo.toml`
- `safe/build.rs`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `safe/abi/original.exports.txt`
- `safe/abi/original.soname.txt`
- `safe/abi/export_map.toml`
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
- `safe/debian/tests/python/check_build/__init__.py`
- `safe/debian/tests/python/check_build/__main__.py`
- `safe/debian/tests/python/check_build/defs.py`
- `safe/debian/tests/python/check_build/parse.py`
- `safe/debian/tests/python/check_build/process.py`
- `safe/debian/tests/python/check_build/util.py`
- `safe/debian/tests/requirements/install.txt`
- `safe/debian/tests/requirements/tests.txt`
- `safe/debian/tests/ztest/programs.toml`
- `safe/debian/tests/ztest/cmake/CMakeLists.txt`
- `safe/debian/tests/ztest/cmake/ztest.c`
- `safe/debian/tests/ztest/pkg-make/Makefile`
- `safe/debian/tests/ztest/pkg-make/ztest.c`
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

Packaging work consumes the safe-only library produced by earlier phases plus the existing Debian metadata and upstream CLI/doc/example source trees. Update those artifacts in place.

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
- rewritten `safe/debian/tests/python/check_build/__init__.py`
- rewritten `safe/debian/tests/python/check_build/__main__.py`
- rewritten `safe/debian/tests/python/check_build/defs.py`
- rewritten `safe/debian/tests/python/check_build/parse.py`
- rewritten `safe/debian/tests/python/check_build/process.py`
- rewritten `safe/debian/tests/python/check_build/util.py`
- rewritten `safe/debian/tests/requirements/install.txt`
- rewritten `safe/debian/tests/requirements/tests.txt`
- rewritten `safe/debian/tests/ztest/programs.toml`
- rewritten `safe/debian/tests/ztest/cmake/CMakeLists.txt`
- rewritten `safe/debian/tests/ztest/cmake/ztest.c`
- rewritten `safe/debian/tests/ztest/pkg-make/Makefile`
- rewritten `safe/debian/tests/ztest/pkg-make/ztest.c`
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed Debian package outputs under `safe/out/deb/default/packages/`
- refreshed `safe/out/deb/default/stage-root/`

# File Changes

- Keep the existing package names and install layout for `libzstd1`, `libzstd-dev`, `zstd`, and `libzstd1-udeb`.
- Ensure build signatures and reuse logic still work after library internals move fully into Rust.
- Ensure CLI and helper-tree build steps link against safe artifacts only.
- Establish the only canonical producer scripts and artifact roots for the install tree, helper root, staged-source tree, and package outputs that later phases may refresh only in place.

# Implementation Details

- `safe/scripts/build-artifacts.sh` must continue to emit the default, `mt`, and `nomt` variants, with the default shared object multithreaded and the default static archive single-threaded, matching upstream contract.
- `safe/scripts/build-original-cli-against-safe.sh` may continue to use upstream CLI/program sources, but the library and headers those sources see must come from the safe artifact tree, not upstream `libzstd`.
- The script verifier must run the three canonical producers in order: `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, and `bash safe/scripts/build-deb.sh`.
- `safe/out/original-cli/lib/` must be left populated with safe headers plus the helper `libzstd.so*` and `libzstd.a` indirection files expected by preserved upstream wrappers.
- `safe/scripts/build-deb.sh` must keep staging `programs/`, `zlibWrapper/`, `examples/`, `contrib/pzstd/`, `doc/educational_decoder/`, and upstream top-level docs because checked-in Debian metadata references those paths.
- `safe/debian/libzstd-dev.examples` must continue to ship `examples/*`; `safe/debian/zstd.docs` must continue to ship `CHANGELOG` and `*.md`; `safe/debian/zstd.manpages` must continue to install `usr/share/man/man1/*`; `safe/debian/rules` must keep `execute_after_dh_installman` generating `zstdmt.1` and `pzstd.1` from safe-built binaries.
- `safe/debian/tests/control`, `safe/debian/tests/python/check_build/*`, `safe/debian/tests/requirements/*`, and `safe/debian/tests/ztest/*` must stay fully safe-rooted and checked in.
- `safe/scripts/run-debian-autopkgtests.sh` must use the safe-staged Debian tree, verify that `debian/tests/` no longer points back into `../original`, and keep the three upstream autopkgtest identities from the original Debian tests.
- This phase defines `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`, and `safe/out/deb/default/` as the only canonical safe build/package roots for later phases.

# Verification Phases

- Phase ID: `script_safe_packaging_install`
  - Type: `check`
  - `bounce_target`: `impl_safe_packaging_install`
  - Purpose: verify build variants, install layout, Debian profiles, and Debian autopkgtests using only safe-built library artifacts.
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
  - Commands: none; perform packaging, artifact, and evidence review.
- Phase ID: `check_safe_packaging_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_safe_packaging_install`
  - Purpose: review Ubuntu/Debian drop-in compatibility and confirm package outputs do not smuggle the old helper-library model back in.
  - Commands: none; perform senior package, ABI, and artifact-root review.

# Success Criteria

- The existing Ubuntu/Debian package identities and install layout are preserved.
- Canonical Phase 4 artifact roots are populated by the canonical producer scripts.
- CLI, examples, pzstd, zlibWrapper, docs, manpages, and autopkgtests are safe-rooted.
- Later phases can consume and refresh the Phase 4 roots only in place.
- All listed verifier commands pass or any failure is fixed before yielding.

# Git Commit Requirement

The implementer must commit the Phase 4 work to git before yielding. That commit must exist before any verifier phase for `impl_safe_packaging_install` runs.
