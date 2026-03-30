# Upstream Black-Box Release Gates

## Phase Name
Upstream Black-Box Release Gates

## Implement Phase ID
`impl_upstream_release_gates`

## Preexisting Inputs
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
- `safe/debian/copyright`
- `safe/debian/rules`
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
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
- `safe/tests/upstream_test_matrix.toml`
- `safe/scripts/verify-header-identity.sh`
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
- `safe/scripts/verify-baseline-contract.sh`
- `safe/scripts/check-cli-permissions.sh`
- `safe/scripts/run-performance-smoke.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/tests/fixtures/versions/`
- `safe/tests/fixtures/regression/`
- `safe/tests/fixtures/fuzz-corpora/`
- `safe/tests/ported/whitebox/Makefile`
- `safe/tests/ported/whitebox/offline_regression_data.c`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `relevant_cves.json`
- `original/libzstd-1.5.5+dfsg2/tests/Makefile`
- `original/libzstd-1.5.5+dfsg2/tests/fuzz/`
- `original/libzstd-1.5.5+dfsg2/tests/regression/`
- `original/libzstd-1.5.5+dfsg2/tests/gzip/`
- `original/libzstd-1.5.5+dfsg2/zlibWrapper/`
- `original/libzstd-1.5.5+dfsg2/doc/educational_decoder/`
- `original/libzstd-1.5.5+dfsg2/contrib/pzstd/`
- `original/libzstd-1.5.5+dfsg2/contrib/seekable_format/`

## New Outputs
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
- refreshed `safe/out/deb/default/packages/`
- refreshed `safe/out/deb/default/stage-root/`

## File Changes
- Keep all current upstream suite families in the release gate.
- Keep the current fixture directories and whitebox harnesses; update them in place if gaps are found.
- Preserve the explicit zlibWrapper, educational decoder, pzstd, seekable, version, regression, fuzz, and CLI-permission coverage.
- Make the upstream black-box wrappers consume the Phase 4 install tree, helper root, and staged package metadata instead of rebuilding those artifacts on demand.
- Preserve the upstream header identity contract for `safe/include/zstd.h`, `safe/include/zdict.h`, and `safe/include/zstd_errors.h` while the black-box wrappers stay rooted on the Phase 4 safe artifact tree.

## Implementation Details
- Preserve the fixed Phase 1 rebased `owning_phase` values in `safe/tests/upstream_test_matrix.toml`; this phase may update only helper-path, prerequisite, `release_gate`, and other wrapper-alignment metadata in place, plus any already-existing suite entries that were missing from the matrix and must be assigned to the already-fixed Phase 1-5 numbering.
- Keep the wrappers offline and reproducible. Reuse existing fixture directories under `safe/tests/fixtures/` instead of downloading or regenerating large corpora.
- `safe/scripts/phase6-common.sh` and every wrapper that sources it must treat `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, and `safe/out/deb/default/metadata.env` as required Phase 4 inputs. They may validate freshness and path resolution, but they must not call `build-artifacts.sh`, `build-original-cli-against-safe.sh`, or `build-deb.sh`.
- Because `safe/scripts/build-deb.sh` currently hashes the full `safe/scripts/` tree, this phase must explicitly rerun the canonical Phase 4 refresh sequence after its wrapper edits and before its verifiers: `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, and `bash safe/scripts/build-deb.sh`.
- `safe/scripts/run-zlibwrapper-tests.sh`, `safe/scripts/run-pzstd-tests.sh`, `safe/scripts/run-seekable-tests.sh`, and `safe/tests/ported/whitebox/Makefile` currently rely on a helper library root built from the safe artifacts plus selected upstream sources; that helper root must keep resolving `libzstd` to the Phase 4 safe build.
- `safe/scripts/run-full-suite.sh` must grow into the single top-level release gate and include `safe/scripts/verify-header-identity.sh`, `safe/scripts/verify-baseline-contract.sh`, the preserved upstream black-box families maintained in this phase, and the downstream image-based matrix once Phase 6 lands. It must not hide missing-artifact rebuilds behind implicit helper calls.
- Preserve the CLI permission regression checks derived from `relevant_cves.json`.

## Verification Phases
- `script_upstream_release_gates` | type: `script` | `bounce_target: impl_upstream_release_gates` | purpose: run the preserved upstream black-box families against the installed safe artifacts and confirm that the current wrappers remain accurate and offline-friendly.
- `check_upstream_release_gates_software_tester` | type: `check` | `bounce_target: impl_upstream_release_gates` | purpose: review coverage completeness, fixture reuse, and any expected-failure handling.
- `check_upstream_release_gates_senior_tester` | type: `check` | `bounce_target: impl_upstream_release_gates` | purpose: review that the black-box suites really exercise the safe install tree instead of an upstream build artifact.

## Verification Commands
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

## Success Criteria
- Every preserved upstream black-box family continues to run against the Phase 4 safe artifact roots and wrapper scripts no longer hide any implicit rebuild or alternate-bootstrap path.
- Fixture directories, whitebox data, header identity, CLI-permission checks, and performance smoke stay checked in and offline-friendly while `safe/tests/upstream_test_matrix.toml` keeps the fixed rebased phase ownership.
- The listed black-box and release-gate verification commands pass after the explicit Phase 4 refresh sequence is rerun in place.

## Git Commit Requirement
The implementer must commit all work for this phase to git before yielding to the verifier phases.
