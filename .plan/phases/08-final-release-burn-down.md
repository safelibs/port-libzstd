# Phase Name

Final Release Burn-Down

# Implement Phase ID

`impl_final_release_burn_down`

# Preexisting Inputs

- `.plan/goal.md`
- `.plan/workflow-structure.yaml`
- `workflow.yaml`
- All outputs from `impl_compat_regressions_and_fixes`, including touched safe source files, checked-in regressions under `safe/tests/`, updated scripts under `safe/scripts/`, `safe/docs/unsafe-audit.md`, metadata updates in `safe/tests/upstream_test_matrix.toml` and `safe/tests/dependents/dependent_matrix.toml`, the refreshed Phase 4 artifact roots, and the refreshed Phase 6 dependent roots
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
- `safe/out/dependents/image-context/metadata.env`
- `safe/out/dependents/compile-compat/`
- `safe/out/dependents/logs/`
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
- `safe/tests/dependents/dependent_matrix.toml`
- `dependents.json`
- `relevant_cves.json`
- `test-original.sh`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/build-dependent-image.sh`
- `safe/scripts/run-full-suite.sh`
- `safe/scripts/verify-header-identity.sh`
- `safe/scripts/verify-export-parity.sh`
- `safe/scripts/verify-link-compat.sh`
- `safe/scripts/verify-baseline-contract.sh`
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
- `safe/docs/unsafe-audit.md`
- git history from the previous implement phases

Final work consumes the canonical Phase 4 and Phase 6 artifact roots. Do not introduce new build, package, image, or results roots.

# New Outputs

- rewritten `safe/scripts/run-full-suite.sh`
- final cleanup edits across touched safe files
- rewritten `safe/abi/export_map.toml`
- rewritten `safe/tests/upstream_test_matrix.toml`
- rewritten `safe/tests/dependents/dependent_matrix.toml`
- rewritten `safe/docs/unsafe-audit.md`
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

- Finalize the aggregator so it includes the image-based downstream workflow and any new regressions from Phase 7.
- Remove temporary comments, compatibility scaffolding, or metadata that became obsolete during earlier phases.
- Leave the repository in a state where the next reviewer can see one commit per implement phase before its verifier phases.
- Keep header identity, baseline contract, upstream suites, downstream matrix, and CVE-derived CLI permission coverage in the final gate.

# Implementation Details

- `safe/scripts/run-full-suite.sh` must remain the single top-level release gate and include `safe/scripts/verify-header-identity.sh`, `safe/scripts/verify-baseline-contract.sh`, preserved upstream suite wrappers, the downstream image-based test flow, and performance smoke.
- The full suite must consume existing Phase 4 and Phase 6 artifact roots instead of rebuilding them implicitly.
- The downstream leg must preserve Phase 6 topology: compile compatibility executes inside the built image, runtime coverage executes in the same image, and runtime containers use `--privileged` with writable `/run`.
- If cleanup edits touch files hashed or staged by canonical Phase 4 or Phase 6 producers, explicitly rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, `bash safe/scripts/build-deb.sh`, and `bash safe/scripts/build-dependent-image.sh` before the final verification suite.
- `safe/scripts/run-full-suite.sh` must not hide that refresh work behind implicit helper calls.
- Final metadata cleanup must keep `safe/abi/export_map.toml`, `safe/tests/upstream_test_matrix.toml`, and `safe/tests/dependents/dependent_matrix.toml` consistent with the finished workflow.
- Final code cleanup must not weaken coverage or reintroduce upstream runtime dependencies.

# Verification Phases

- Phase ID: `script_final_release_burn_down`
  - Type: `check`
  - `bounce_target`: `impl_final_release_burn_down`
  - Purpose: run the end-to-end release gate after all fixes and ensure no shipping dependency on upstream C remains outside the approved boundary.
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
  - Commands: none; perform final release-gate and residual-risk review.
- Phase ID: `check_final_release_senior_tester`
  - Type: `check`
  - `bounce_target`: `impl_final_release_burn_down`
  - Purpose: review drop-in replaceability, linear workflow discipline, and final unsafe justification.
  - Commands: none; perform senior release, workflow, ABI, and unsafe review.

# Success Criteria

- `safe/scripts/run-full-suite.sh` is the single top-level release gate and succeeds.
- `bash test-original.sh` succeeds independently.
- The shipping safe library exports the upstream symbol set with matching SONAME and no extra public exports.
- No shipping path depends on `SAFE_UPSTREAM_LIB`, `dlopen()`, `dlsym()`, `load_upstream!`, or `upstream-phase4`.
- Any remaining C is justified ABI-boundary or legacy-format glue.
- Debian packages, upstream suite families, downstream 12-application image matrix, performance smoke, fixtures, fuzz replay, and CLI permission checks all pass against canonical safe artifacts.
- Every compatibility failure found during the work has a checked-in regression.

# Git Commit Requirement

The implementer must commit the Phase 8 work to git before yielding. That commit must exist before any verifier phase for `impl_final_release_burn_down` runs.
