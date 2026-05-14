# Phase Name

Final Release Burn-Down

# Implement Phase ID

`impl_final_release_burn_down`

# Preexisting Inputs

- `safe/src/**/*`
- `safe/tests/rust/*.rs`
- `safe/tests/capi/*.c`
- `safe/tests/dependents/src/*.c`
- `safe/tests/dependents/regressions/*`
- `safe/scripts/*`
- `safe/docs/unsafe-audit.md`
- `safe/tests/upstream_test_matrix.toml`
- `safe/tests/dependents/dependent_matrix.toml`
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/*.deb`
- `safe/out/deb/default/packages/*.udeb`
- `safe/out/deb/default/stage-root/`
- `safe/out/dependents/image-context/`
- `safe/out/dependents/compile-compat/`
- `safe/out/dependents/logs/`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/build-dependent-image.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/abi/export_map.toml`
- `git history from the previous implement phases`

# New Outputs

- rewritten `safe/scripts/run-full-suite.sh`
- final cleanup edits across any touched safe files `safe/**/*`
- rewritten `safe/abi/export_map.toml`
- rewritten `safe/tests/upstream_test_matrix.toml`
- rewritten `safe/tests/dependents/dependent_matrix.toml`
- rewritten `safe/docs/unsafe-audit.md`
- refreshed `safe/out/install/release-default/`
- refreshed `safe/out/original-cli/lib/`
- refreshed `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- refreshed `safe/out/deb/default/metadata.env`
- refreshed `safe/out/deb/default/packages/*.deb`
- refreshed when SAFE_ENABLE_UDEB=1 `safe/out/deb/default/packages/*.udeb`
- refreshed `safe/out/deb/default/stage-root/`
- refreshed `safe/out/dependents/image-context/`
- refreshed `safe/out/dependents/compile-compat/`
- refreshed `safe/out/dependents/logs/`

# File Changes

- Finalize the aggregator so it includes the image-based downstream workflow and any new regressions from Phase 7.
- Remove temporary comments, compatibility scaffolding, or metadata that became obsolete during earlier phases.
- Leave the repository in a state where the next reviewer can see one commit per implement phase before its verifier phases.

# Implementation Details

- `safe/scripts/run-full-suite.sh` must remain the single top-level release gate and include `safe/scripts/verify-header-identity.sh`, `safe/scripts/verify-baseline-contract.sh`, the preserved upstream suite wrappers, downstream image-based test flow, and performance smoke, all consuming existing Phase 4 and Phase 6 artifact roots instead of rebuilding them.
- Its downstream leg must keep the Phase 6 topology unchanged: compile compatibility executes inside the built image, and runtime coverage executes in the same image with the preserved `--privileged` plus writable-`/run` container contract.
- If this phase makes cleanup edits that touch files hashed or staged by the canonical Phase 4 or Phase 6 producers, explicitly rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, `bash safe/scripts/build-deb.sh`, and `bash safe/scripts/build-dependent-image.sh` before the final verification suite.
- `safe/scripts/run-full-suite.sh` itself must not hide refresh work behind implicit helper calls.
- Final metadata cleanup must keep `safe/abi/export_map.toml`, `safe/tests/upstream_test_matrix.toml`, and `safe/tests/dependents/dependent_matrix.toml` consistent with the finished workflow.
- Final code cleanup must not weaken coverage or reintroduce upstream runtime dependencies.

# Verification Phases

- Phase ID: `script_final_release_burn_down`
- Type: `check`
- `bounce_target`: `impl_final_release_burn_down`
- Purpose: run the end-to-end release gate after all fixes and ensure no shipping dependency on upstream C remains outside the approved boundary.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
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

- Phase ID: `check_final_release_software_tester`
- Type: `check`
- `bounce_target`: `impl_final_release_burn_down`
- Purpose: review residual risks, testing completeness, and final regression coverage.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

- Phase ID: `check_final_release_senior_tester`
- Type: `check`
- `bounce_target`: `impl_final_release_burn_down`
- Purpose: review drop-in replaceability, linear workflow discipline, and final unsafe justification.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

# Success Criteria

- The shipping library exports the full upstream symbol set with the same SONAME and no extra public exports.
- No shipping path depends on `SAFE_UPSTREAM_LIB`, `dlopen()`, `dlsym()`, `load_upstream!`, or `upstream-phase4`.
- The only remaining C in the final library build is justified ABI-boundary or legacy-format glue.
- All preserved upstream suites, the downstream 12-application image matrix, `bash safe/scripts/run-full-suite.sh`, and `bash test-original.sh` pass.
- Artifact roots under `safe/out/install/release-default/`, `safe/out/original-cli/lib/`, `safe/out/deb/default/`, and `safe/out/dependents/` remain the only release-gate state and are refreshed only through canonical scripts.

# Git Commit Requirement

The implementer must commit all Phase 8 work to git before yielding. That commit must exist before `script_final_release_burn_down`, `check_final_release_software_tester`, and `check_final_release_senior_tester` run.
