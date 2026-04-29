# Phase Name

Upstream Black-Box Release Gates

# Implement Phase ID

`impl_upstream_release_gates`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- All outputs from `impl_safe_packaging_install`, including `safe/scripts/build-artifacts.sh`, `safe/scripts/build-deb.sh`, `safe/scripts/build-original-cli-against-safe.sh`, `safe/scripts/install-safe-debs.sh`, `safe/scripts/verify-install-layout.sh`, `safe/scripts/verify-deb-profiles.sh`, `safe/scripts/run-debian-autopkgtests.sh`, `safe/scripts/run-build-variant-tests.sh`, `safe/debian/`, `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`, `safe/out/deb/default/metadata.env`, `safe/out/deb/default/packages/`, and `safe/out/deb/default/stage-root/`
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `original/libzstd-1.5.5+dfsg2/lib/zstd.h`
- `original/libzstd-1.5.5+dfsg2/lib/zdict.h`
- `original/libzstd-1.5.5+dfsg2/lib/zstd_errors.h`
- `safe/abi/original.exports.txt`
- `safe/abi/original.soname.txt`
- `safe/abi/export_map.toml`
- `safe/tests/upstream_test_matrix.toml`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/verify-header-identity.sh`
- `safe/scripts/verify-baseline-contract.sh`
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
- `safe/scripts/phase6-common.sh`
- `safe/scripts/check-cli-permissions.sh`
- `safe/scripts/run-performance-smoke.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/tests/fixtures/versions/README.md`
- `safe/tests/fixtures/versions/manifest.toml`
- `safe/tests/fixtures/versions/hello`
- `safe/tests/fixtures/versions/hello.zst`
- `safe/tests/fixtures/versions/helloworld`
- `safe/tests/fixtures/versions/helloworld.zst`
- `safe/tests/fixtures/regression/README.md`
- `safe/tests/fixtures/regression/results-memoized.csv`
- `safe/tests/fixtures/regression/results-memoized.source-sha256`
- `safe/tests/fixtures/fuzz-corpora/manifest.toml`
- `safe/tests/fixtures/fuzz-corpora/raw/hello`
- `safe/tests/fixtures/fuzz-corpora/raw/helloworld`
- `safe/tests/fixtures/fuzz-corpora/compressed/hello.zst`
- `safe/tests/fixtures/fuzz-corpora/compressed/helloworld.zst`
- `safe/tests/ported/whitebox/Makefile`
- `safe/tests/ported/whitebox/offline_regression_data.c`
- `relevant_cves.json`
- `original/libzstd-1.5.5+dfsg2/tests/Makefile`
- `original/libzstd-1.5.5+dfsg2/tests/fuzz/`
- `original/libzstd-1.5.5+dfsg2/tests/regression/`
- `original/libzstd-1.5.5+dfsg2/tests/gzip/`
- `original/libzstd-1.5.5+dfsg2/zlibWrapper/`
- `original/libzstd-1.5.5+dfsg2/doc/educational_decoder/`
- `original/libzstd-1.5.5+dfsg2/contrib/pzstd/`
- `original/libzstd-1.5.5+dfsg2/contrib/seekable_format/`

This phase consumes the Phase 4 build/package roots and the checked-in offline fixtures. Header identity inputs are explicit and must be preserved in place.

# New Outputs

- rewritten `safe/tests/upstream_test_matrix.toml`
- rewritten `safe/scripts/phase6-common.sh`
- rewritten `safe/scripts/run-upstream-tests.sh`
- rewritten `safe/scripts/run-original-playtests.sh`
- rewritten `safe/scripts/run-original-cli-tests.sh`
- rewritten `safe/scripts/run-original-gzip-tests.sh`
- rewritten `safe/scripts/run-zlibwrapper-tests.sh`
- rewritten `safe/scripts/run-educational-decoder-tests.sh`
- rewritten `safe/scripts/run-pzstd-tests.sh`
- rewritten `safe/scripts/run-seekable-tests.sh`
- rewritten `safe/scripts/run-version-compat-tests.sh`
- rewritten `safe/scripts/run-upstream-regression.sh`
- rewritten `safe/scripts/run-upstream-fuzz-tests.sh`
- rewritten `safe/scripts/run-original-examples.sh`
- rewritten `safe/scripts/check-cli-permissions.sh`
- rewritten `safe/scripts/run-performance-smoke.sh`
- rewritten `safe/scripts/run-full-suite.sh`
- rewritten `safe/tests/fixtures/versions/manifest.toml`
- rewritten `safe/tests/fixtures/regression/README.md`
- rewritten `safe/tests/fixtures/fuzz-corpora/manifest.toml`
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed Debian package outputs under `safe/out/deb/default/packages/`
- refreshed `safe/out/deb/default/stage-root/`

# File Changes

- Keep all current upstream suite families in the release gate: export parity, header identity, link compatibility, Rust tests, C API tests, build/install variants, Debian profile outputs, Debian autopkgtests, upstream `tests/Makefile` targets, playtests, CLI variant tests, CLI smoke tests, gzip compatibility, zlibWrapper, educational decoder, pzstd, examples, seekable format, version fixtures, offline regression harness, fuzz replay, and CLI permission checks.
- Keep the current fixture directories and whitebox harnesses; update them in place if gaps are found.
- Preserve zlibWrapper, educational decoder, pzstd, seekable, version, regression, fuzz, and CLI-permission coverage.
- Make black-box wrappers consume the Phase 4 install tree, helper root, and staged package metadata instead of rebuilding those artifacts on demand.

# Implementation Details

- Preserve the fixed Phase 1 rebased `owning_phase` values in `safe/tests/upstream_test_matrix.toml`; this phase may update helper-path, prerequisite, `release_gate`, wrapper-alignment metadata, and already-existing missing entries assigned to the fixed Phase 1-5 numbering only.
- Keep wrappers offline and reproducible. Reuse `safe/tests/fixtures/versions/`, `safe/tests/fixtures/regression/`, and `safe/tests/fixtures/fuzz-corpora/` instead of downloading or regenerating large corpora.
- `safe/scripts/phase6-common.sh` and every wrapper that sources it must treat `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, and `safe/out/deb/default/metadata.env` as required Phase 4 inputs. They may validate freshness and path resolution, but they must not call `build-artifacts.sh`, `build-original-cli-against-safe.sh`, or `build-deb.sh`.
- Because `safe/scripts/build-deb.sh` hashes the full `safe/scripts/` tree, this phase must explicitly rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, and `bash safe/scripts/build-deb.sh` after wrapper edits and before verifiers.
- `safe/scripts/run-zlibwrapper-tests.sh`, `safe/scripts/run-pzstd-tests.sh`, `safe/scripts/run-seekable-tests.sh`, and `safe/tests/ported/whitebox/Makefile` must keep resolving `libzstd` to the Phase 4 safe build through `safe/out/original-cli/lib/`.
- `safe/scripts/run-full-suite.sh` must grow into the single top-level release gate and include `safe/scripts/verify-header-identity.sh`, `safe/scripts/verify-baseline-contract.sh`, the preserved upstream black-box families, and the downstream image-based matrix once Phase 6 lands. It must not hide missing-artifact rebuilds behind implicit helper calls.
- Preserve the CLI permission regression checks derived from `relevant_cves.json`.

# Verification Phases

- Phase ID: `script_upstream_release_gates`
  - Type: `check`
  - `bounce_target`: `impl_upstream_release_gates`
  - Purpose: run preserved upstream black-box families against installed safe artifacts and confirm wrappers remain accurate and offline-friendly.
  - Commands:
    - `bash safe/scripts/build-artifacts.sh --release`
    - `bash safe/scripts/build-original-cli-against-safe.sh`
    - `bash safe/scripts/build-deb.sh`
    - `bash safe/scripts/verify-header-identity.sh`
    - `bash safe/scripts/verify-baseline-contract.sh`
    - `bash safe/scripts/run-upstream-tests.sh`
    - `bash safe/scripts/run-original-playtests.sh`
    - `bash safe/scripts/run-original-cli-tests.sh`
    - `bash safe/scripts/run-original-gzip-tests.sh`
    - `bash safe/scripts/run-zlibwrapper-tests.sh`
    - `bash safe/scripts/run-educational-decoder-tests.sh`
    - `bash safe/scripts/run-pzstd-tests.sh`
    - `bash safe/scripts/run-seekable-tests.sh`
    - `bash safe/scripts/run-version-compat-tests.sh`
    - `bash safe/scripts/run-upstream-regression.sh`
    - `bash safe/scripts/run-upstream-fuzz-tests.sh`
    - `bash safe/scripts/run-original-examples.sh`
    - `bash safe/scripts/check-cli-permissions.sh`
    - `bash safe/scripts/run-performance-smoke.sh`
- Phase ID: `check_upstream_release_gates_software_tester`
  - Type: `check`
  - `bounce_target`: `impl_upstream_release_gates`
  - Purpose: review coverage completeness, fixture reuse, header identity, and expected-failure handling.
  - Commands: none; perform release-gate, fixture, and evidence review.
- Phase ID: `check_upstream_release_gates_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_upstream_release_gates`
  - Purpose: review that black-box suites exercise the safe install tree instead of an upstream build artifact.
  - Commands: none; perform senior wrapper, artifact-root, and release-gate review.

# Success Criteria

- Header identity is explicitly checked against the existing safe and upstream header files.
- All preserved upstream release-gate families remain wired to canonical Phase 4 artifacts.
- Offline fixture directories are consumed in place.
- `safe/scripts/run-full-suite.sh` includes the preserved upstream gate families without implicit Phase 4 rebuilds.
- All listed verifier commands pass or any failure is fixed before yielding.

# Git Commit Requirement

The implementer must commit the Phase 5 work to git before yielding. That commit must exist before any verifier phase for `impl_upstream_release_gates` runs.
