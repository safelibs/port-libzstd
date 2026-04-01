# Final Release Burn-Down

## Workflow Position
Phase 8 of 8 in the linear explicit-phase workflow.

## Phase Name
Final Release Burn-Down

## Implement Phase ID
`impl_final_release_burn_down`

## Preexisting Inputs
- `safe/Cargo.toml`
- `dependents.json`
- `safe/src/`
- `safe/tests/rust/`
- `safe/tests/capi/`
- `safe/tests/dependents/src/`
- `safe/tests/dependents/`
- `safe/tests/fixtures/versions/`
- `safe/tests/fixtures/regression/`
- `safe/tests/fixtures/fuzz-corpora/`
- `safe/tests/ported/whitebox/Makefile`
- `safe/tests/ported/whitebox/offline_regression_data.c`
- `safe/scripts/`
- `safe/scripts/phase6-common.sh`
- `safe/docs/unsafe-audit.md`
- `safe/tests/upstream_test_matrix.toml`
- `safe/tests/dependents/dependent_matrix.toml`
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
- `safe/out/dependents/image-context/`
- `safe/out/dependents/compile-compat/`
- `safe/out/dependents/logs/`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/build-dependent-image.sh`
- `safe/scripts/verify-header-identity.sh`
- `safe/scripts/verify-export-parity.sh`
- `safe/scripts/verify-link-compat.sh`
- `safe/scripts/run-capi-decompression.sh`
- `safe/scripts/run-capi-roundtrip.sh`
- `safe/scripts/run-advanced-mt-tests.sh`
- `safe/scripts/run-build-variant-tests.sh`
- `safe/scripts/verify-install-layout.sh`
- `safe/scripts/verify-deb-profiles.sh`
- `safe/scripts/run-debian-autopkgtests.sh`
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
- `safe/scripts/check-cli-permissions.sh`
- `safe/scripts/run-performance-smoke.sh`
- `safe/scripts/run-dependent-matrix.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/include/zstd.h`
- `safe/include/zdict.h`
- `safe/include/zstd_errors.h`
- `safe/abi/export_map.toml`
- `relevant_cves.json`
- `test-original.sh`
- `.git/`

## New Outputs
- rewritten `safe/scripts/run-full-suite.sh`
- final cleanup edits across any touched safe files
- rewritten `safe/abi/export_map.toml`
- rewritten `safe/tests/upstream_test_matrix.toml`
- rewritten `safe/tests/dependents/dependent_matrix.toml`
- rewritten `safe/docs/unsafe-audit.md`
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed `safe/out/deb/default/packages/`
- refreshed `safe/out/deb/default/stage-root/`
- refreshed `safe/out/dependents/image-context/`
- refreshed `safe/out/dependents/compile-compat/`
- refreshed `safe/out/dependents/logs/`

## File Changes
- Finalize the aggregator so it includes the image-based downstream workflow and any new regressions from Phase 7.
- Remove temporary comments, compatibility scaffolding, or metadata that became obsolete during earlier phases.
- Leave the repository in a state where the next reviewer can see one commit per implement phase before its verifier phases.

## Implementation Details
- `safe/scripts/run-full-suite.sh` must remain the single top-level release gate and must include `safe/scripts/verify-header-identity.sh`, `safe/scripts/verify-baseline-contract.sh`, the preserved upstream suite wrappers, the downstream image-based test flow, and performance smoke, all consuming the existing Phase 4 and Phase 6 artifact roots instead of rebuilding them. Its downstream leg must keep the Phase 6 topology unchanged: compile compatibility executes inside the built image, and runtime coverage executes in the same image with the preserved `--privileged` plus writable-`/run` container contract.
- If this phase makes any cleanup edit that touches files hashed or staged by the canonical Phase 4 or Phase 6 producers, it must explicitly rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, `bash safe/scripts/build-deb.sh`, and `bash safe/scripts/build-dependent-image.sh` before the final verification suite. `safe/scripts/run-full-suite.sh` itself must not hide that refresh work behind implicit helper calls.
- Final metadata cleanup must keep `safe/abi/export_map.toml`, `safe/tests/upstream_test_matrix.toml`, and `safe/tests/dependents/dependent_matrix.toml` consistent with the finished workflow.
- Final code cleanup must not weaken coverage or reintroduce upstream runtime dependencies.

## Verification Phases
- `script_final_release_burn_down` | type: `script` | `bounce_target: impl_final_release_burn_down` | purpose: run the end-to-end release gate after all fixes and ensure no shipping dependency on upstream C remains outside the approved boundary.
- `check_final_release_software_tester` | type: `check` | `bounce_target: impl_final_release_burn_down` | purpose: review residual risks, testing completeness, and final regression coverage.
- `check_final_release_senior_tester` | type: `check` | `bounce_target: impl_final_release_burn_down` | purpose: review drop-in replaceability, linear workflow discipline, and final unsafe justification.

## Verification Commands
- `bash safe/scripts/build-artifacts.sh --release`
- `bash safe/scripts/build-original-cli-against-safe.sh`
- `bash safe/scripts/build-deb.sh`
- `bash safe/scripts/build-dependent-image.sh`
- `cargo test --manifest-path safe/Cargo.toml --release --all-targets`
- `bash safe/scripts/verify-header-identity.sh`
- `bash safe/scripts/verify-export-parity.sh`
- `bash safe/scripts/verify-link-compat.sh`
- `bash safe/scripts/run-capi-decompression.sh`
- `bash safe/scripts/run-capi-roundtrip.sh`
- `bash safe/scripts/run-advanced-mt-tests.sh`
- `bash safe/scripts/run-build-variant-tests.sh`
- `bash safe/scripts/verify-install-layout.sh`
- `bash safe/scripts/verify-install-layout.sh --debian`
- `bash safe/scripts/verify-deb-profiles.sh`
- `bash safe/scripts/run-debian-autopkgtests.sh`
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
- `bash safe/scripts/check-cli-permissions.sh`
- `bash safe/scripts/run-performance-smoke.sh`
- `bash safe/scripts/run-dependent-matrix.sh --compile-only`
- `bash safe/scripts/run-dependent-matrix.sh --runtime-only`
- `bash safe/scripts/verify-baseline-contract.sh`
- `bash test-original.sh`
- `bash safe/scripts/run-full-suite.sh`
- `rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe`

## Success Criteria
- `safe/scripts/run-full-suite.sh` is the single top-level release gate and it exercises header identity, baseline contract, all preserved upstream suites, the image-based downstream matrix, and performance smoke using the existing canonical artifact roots.
- Final cleanup leaves export metadata, upstream and dependent matrix metadata, the unsafe audit, and git history consistent with the finished 8-phase linear workflow, with one implement commit visible before each verifier set.
- The complete final verification suite passes, including `bash safe/scripts/run-full-suite.sh`, `bash test-original.sh`, and the forbidden-pattern grep proving no prohibited upstream runtime dependency remains.

## Git Commit Requirement
The implementer must commit all work for this phase to git before yielding to the verifier phases.
