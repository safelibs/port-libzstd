# Dependent Regressions and Compatibility Fixes

## Workflow Position
Phase 7 of 8 in the linear explicit-phase workflow.

## Phase Name
Dependent Regressions and Compatibility Fixes

## Implement Phase ID
`impl_compat_regressions_and_fixes`

## Preexisting Inputs
- `safe/Cargo.toml`
- `dependents.json`
- `safe/tests/upstream_test_matrix.toml`
- `safe/tests/dependents/dependent_matrix.toml`
- `safe/tests/dependents/src/rpm_probe.c`
- `safe/tests/dependents/src/zarchive_probe.c`
- `safe/tests/dependents/fixtures/rpm/hello.spec`
- `safe/tests/dependents/fixtures/rpm/hello.txt`
- `safe/tests/dependents/fixtures/zarchive/input/a.txt`
- `safe/tests/dependents/fixtures/zarchive/input/sub/b.txt`
- `safe/docker/dependents/Dockerfile`
- `safe/docker/dependents/entrypoint.sh`
- `safe/scripts/build-artifacts.sh`
- `safe/scripts/build-original-cli-against-safe.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/build-dependent-image.sh`
- `safe/scripts/run-dependent-matrix.sh`
- `safe/scripts/check-dependent-compile-compat.sh`
- `safe/scripts/verify-baseline-contract.sh`
- `test-original.sh`
- `safe/out/dependents/image-context/metadata.env`
- `safe/out/install/release-default/`
- `safe/out/original-cli/lib/`
- `safe/out/debian-src/default/libzstd-1.5.5+dfsg2/`
- `safe/out/deb/default/metadata.env`
- `safe/out/deb/default/packages/`
- `safe/out/deb/default/stage-root/`
- `safe/out/dependents/image-context/`
- `safe/out/dependents/compile-compat/`
- `safe/out/dependents/logs/`
- `safe/src/`
- `safe/tests/rust/`
- `safe/tests/capi/`
- `safe/tests/dependents/`
- `safe/scripts/`
- `safe/docs/unsafe-audit.md`

## New Outputs
- rewritten library/source files in the specific safe modules implicated by downstream failures
- new or rewritten checked-in regressions under the closest existing harness:
  - `safe/tests/rust/`
  - `safe/tests/capi/`
  - `safe/tests/dependents/src/`
  - `safe/tests/dependents/`
  - `safe/scripts/`
- rewritten `safe/docs/unsafe-audit.md`
- rewritten `safe/tests/upstream_test_matrix.toml` and `safe/tests/dependents/dependent_matrix.toml` if coverage metadata changes
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
- Add a reproducer before fixing every newly discovered compatibility issue.
- Keep each reproducer as close as possible to the failing surface rather than creating a disconnected ad hoc test location.
- If coverage metadata changes, keep the fixed Phase 1 rebased `owning_phase` values in `safe/tests/upstream_test_matrix.toml` and update only status, prerequisite, helper-path, `release_gate`, or similarly non-renumbering metadata in place.
- Update the downstream image/runtime harness only when the failure is in the harness, not when the failure is in the library behavior.

## Implementation Details
- Triage failures from the existing Phase 6 logs and artifacts under `safe/out/dependents/` first; do not create a second dependent-results manifest and do not broaden the app set beyond the fixed 12-application inventory.
- Route failures to the narrowest appropriate harness:
  - ABI or pointer-semantics bugs go to `safe/tests/capi/`
  - algorithmic or Rust API bugs go to `safe/tests/rust/`
  - packaging or CLI integration bugs go to `safe/scripts/` or Debian tests
  - app-specific failures go to `safe/tests/dependents/`
- If this phase edits any file hashed or staged by the canonical Phase 4 or Phase 6 producers, it must explicitly rerun the canonical refresh chain in order before downstream verifiers: `bash safe/scripts/build-artifacts.sh --release`, `bash safe/scripts/build-original-cli-against-safe.sh`, `bash safe/scripts/build-deb.sh`, and `bash safe/scripts/build-dependent-image.sh`. Reuse the existing Phase 4 and Phase 6 paths; do not create alternate output roots, an alternate image tag flow, or an extra results manifest.
- If a harness fix requires refreshing the staged downstream image context, update `safe/out/dependents/` in place through the existing scripts; do not reintroduce inline `docker run ... bash <<EOF` logic, repo mounts, host toolchain binds, or a host-side compile-compat path outside the image.
- Update `safe/docs/unsafe-audit.md` whenever a fix changes where `unsafe` is used or why it remains.
- Re-run affected upstream suites after each fix so dependent fixes do not silently regress the broader black-box surface.

## Verification Phases
- `script_compat_regressions_and_fixes` | type: `script` | `bounce_target: impl_compat_regressions_and_fixes` | purpose: run the full 12-application runtime matrix, execute the new regressions, and validate the fixes.
- `check_compat_regressions_software_tester` | type: `check` | `bounce_target: impl_compat_regressions_and_fixes` | purpose: review that every discovered bug gained a reproducer before being fixed.
- `check_compat_regressions_senior_tester` | type: `check` | `bounce_target: impl_compat_regressions_and_fixes` | purpose: review the appropriateness of each fix, the minimality of the unsafe surface, and any cross-suite regression risk.

## Verification Commands
- `bash safe/scripts/build-artifacts.sh --release`
- `bash safe/scripts/build-original-cli-against-safe.sh`
- `bash safe/scripts/build-deb.sh`
- `bash safe/scripts/build-dependent-image.sh`
- `bash safe/scripts/run-dependent-matrix.sh --compile-only`
- `bash safe/scripts/run-dependent-matrix.sh --runtime-only`
- `bash test-original.sh`
- run the exact new regression tests added in this phase
- rerun whichever upstream suites exercise the touched surface before yielding

## Success Criteria
- Every newly discovered compatibility failure gets a checked-in reproducer before the fix, placed in the closest existing harness rather than in a parallel ad hoc location.
- Any required Phase 4 or Phase 6 artifact refresh happens explicitly in place through the canonical build, packaging, and image scripts, and the downstream matrix continues to use the existing 12-app inventory, image roots, and output roots without alternate roots or extra manifests.
- The full runtime matrix, `test-original.sh`, the new regressions, and the touched upstream suites all pass after the fixes land, with `safe/docs/unsafe-audit.md` updated for any unsafe-surface changes.

## Git Commit Requirement
The implementer must commit all work for this phase to git before yielding to the verifier phases.
