# Phase Name

Dependent Regressions and Compatibility Fixes

# Implement Phase ID

`impl_compat_regressions_and_fixes`

# Preexisting Inputs

- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/*.deb`
- `safe/out/deb/default/packages/*.udeb`
- `safe/out/deb/default/stage-root/`
- `dependents.json`
- `safe/tests/dependents/dependent_matrix.toml`
- `safe/tests/dependents/src/rpm_probe.c`
- `safe/tests/dependents/src/zarchive_probe.c`
- `safe/tests/dependents/fixtures/rpm/hello.spec`
- `safe/tests/dependents/fixtures/rpm/hello.txt`
- `safe/tests/dependents/fixtures/zarchive/input/a.txt`
- `safe/tests/dependents/fixtures/zarchive/input/sub/b.txt`
- `safe/docker/dependents/Dockerfile`
- `safe/docker/dependents/entrypoint.sh`
- `safe/scripts/build-dependent-image.sh`
- `safe/scripts/run-dependent-matrix.sh`
- `safe/scripts/check-dependent-compile-compat.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `test-original.sh`
- `safe/out/dependents/image-context/`
- `safe/out/dependents/image-context/metadata.env`
- `safe/out/dependents/compile-compat/`
- `safe/out/dependents/logs/`
- `safe/src/**/*`
- `safe/tests/rust/*`
- `safe/tests/capi/*`
- `safe/tests/dependents/*`
- `safe/scripts/*`
- `safe/docs/unsafe-audit.md`

# New Outputs

- rewritten library/source files in the specific safe modules implicated by downstream failures `safe/src/**/*`
- new or rewritten checked-in Rust regressions `safe/tests/rust/*.rs`
- new or rewritten checked-in C API regressions `safe/tests/capi/*.c`
- new or rewritten dependent compile probes `safe/tests/dependents/src/*.c`
- new or rewritten dependent regressions `safe/tests/dependents/regressions/*`
- new or rewritten harness scripts `safe/scripts/*`
- rewritten `safe/docs/unsafe-audit.md`
- rewritten if coverage metadata changes `safe/tests/upstream_test_matrix.toml`
- rewritten if coverage metadata changes `safe/tests/dependents/dependent_matrix.toml`
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

- Add a reproducer before fixing every newly discovered compatibility issue.
- Keep each reproducer as close as possible to the failing surface rather than creating a disconnected ad hoc test location.
- Update the downstream image/runtime harness only when the failure is in the harness, not when the failure is in library behavior.

# Implementation Details

- Triage failures from existing Phase 6 logs and artifacts under `safe/out/dependents/` first; do not create a second dependent-results manifest and do not broaden the app set beyond the fixed 12-application inventory.
- Route ABI or pointer-semantics bugs to `safe/tests/capi/`.
- Route algorithmic or Rust API bugs to `safe/tests/rust/`.
- Route packaging or CLI integration bugs to `safe/scripts/` or Debian tests.
- Route app-specific failures to `safe/tests/dependents/`.
- If this phase edits any file hashed or staged by the canonical Phase 4 or Phase 6 producers, rerun `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, `bash safe/scripts/build-deb.sh`, and `bash safe/scripts/build-dependent-image.sh` before downstream verifiers.
- Reuse the existing Phase 4 and Phase 6 paths; do not create alternate output roots, alternate image tag flow, or an extra results manifest.
- Do not reintroduce inline `docker run ... bash <<EOF` logic, repo mounts, host toolchain binds, or a host-side compile-compat path outside the image.
- Update `safe/docs/unsafe-audit.md` whenever a fix changes where `unsafe` is used or why it remains.
- Re-run affected upstream suites after each fix so dependent fixes do not silently regress the broader black-box surface.

# Verification Phases

- Phase ID: `script_compat_regressions_and_fixes`
- Type: `check`
- `bounce_target`: `impl_compat_regressions_and_fixes`
- Purpose: run the full 12-application runtime matrix, execute the new regressions, and validate the fixes.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - `bash safe/scripts/build-artifacts.sh --release`
  - `bash safe/scripts/build-original-cli-against-safe.sh`
  - `bash safe/scripts/build-deb.sh`
  - `bash safe/scripts/build-dependent-image.sh`
  - `bash safe/scripts/run-dependent-matrix.sh --compile-only`
  - `bash safe/scripts/run-dependent-matrix.sh --runtime-only`
  - `bash test-original.sh`
  - `run the exact new regression tests added in this phase`
  - `rerun whichever upstream suites exercise the touched surface before yielding`

- Phase ID: `check_compat_regressions_software_tester`
- Type: `check`
- `bounce_target`: `impl_compat_regressions_and_fixes`
- Purpose: review that every discovered bug gained a reproducer before being fixed.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

- Phase ID: `check_compat_regressions_senior_tester`
- Type: `check`
- `bounce_target`: `impl_compat_regressions_and_fixes`
- Purpose: review the appropriateness of each fix, the minimality of the unsafe surface, and any cross-suite regression risk.
- Required Preexisting Inputs: the implement phase's preexisting inputs plus the new outputs listed above.
- Commands:
  - no fixed shell commands; inspect the changed files, artifacts, script verifier output, and git history for the required commit

# Success Criteria

- Every newly discovered compatibility issue has a checked-in reproducer before its fix.
- The full 12-application downstream runtime matrix and compile pass succeed through the Phase 6 image workflow.
- Affected upstream suites are rerun for each touched surface.
- No alternate artifact root, image tag flow, results manifest, or host-side compile-compat path is introduced.

# Git Commit Requirement

The implementer must commit all Phase 7 work to git before yielding. That commit must exist before `script_compat_regressions_and_fixes`, `check_compat_regressions_software_tester`, and `check_compat_regressions_senior_tester` run.
