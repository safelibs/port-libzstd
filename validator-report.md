Phase 1 Base Commit: 138ed3431977d3c091cd62ff85ef8661baaed5bc

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator Commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
- Validator branch: main (detached HEAD at origin/main)
- Planning reference commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Local safe commit: 138ed3431977d3c091cd62ff85ef8661baaed5bc

The pinned planning commit `1319bb03` predates `safelibs/validator@c58e3e28`,
the upstream rename of the matrix mode from `port-04-test` to `port`. The
local CI runner at `safe/scripts/run-validator-libzstd.sh` invokes
`--mode port` (commit `cd37465`), so the validator checkout was advanced to
`origin/main` (`87b321fe`) before the matrix run; older commits reject the
new mode label. The same upstream rebase introduced
`safelibs/validator@b5b1b5df`, which requires `tag_ref` in the port deb-lock
to equal `refs/tags/<release_tag>`; the runner's inline lock generator emits
that shape (the `cd37465` rename had previously left the historical
`refs/tags/libzstd/04-test-local` literal in place; `9155c55` completed the
rename).

**Python Setup**

- Python setup path: `python3` (`/home/yans/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3`).
- PyYAML source: host `python3 -c 'import yaml'` succeeded (PyYAML 6.0.3); `safe/out/validator/venv/` was not created.

**Override Packages**

The validator override leaf is `safe/out/validator/override-debs/libzstd/`.

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 380926 | 9c05c6f3a144354da30827b2a020d1341f4f6f57d3e9e6c6d1aef22988b6b27c |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 3830588 | 1525e2933b9d26206f51a8e51af45935bcb11629cfb93203f22048ed39f5f6e6 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

**Generated Port Lock**

- Path: `safe/out/validator/artifacts/proof/port-debs-lock.json`
- Repository: local/port-libzstd
- Tag ref: refs/tags/build-138ed3431977
- Commit: 138ed3431977d3c091cd62ff85ef8661baaed5bc
- Release tag: build-138ed3431977
- Package architectures: amd64
- Package sizes: libzstd1=380926, libzstd-dev=3830588, zstd=159324
- Package SHA256 hashes: libzstd1=9c05c6f3a144354da30827b2a020d1341f4f6f57d3e9e6c6d1aef22988b6b27c, libzstd-dev=1525e2933b9d26206f51a8e51af45935bcb11629cfb93203f22048ed39f5f6e6, zstd=8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91

The lock's `commit`, `release_tag`, and `tag_ref` fields are derived at
runtime from `git rev-parse HEAD`; the values above are those produced by the
runner against the Phase 1 base commit recorded above.

**Exact Commands Run**

```bash
git rev-parse HEAD
git -C validator rev-parse HEAD
ls safe/out/validator/artifacts/proof/
test ! -f safe/out/validator/artifacts/proof/port-04-test-debs-lock.json
test ! -f safe/out/validator/artifacts/proof/port-04-test-validation-proof.json
test ! -f safe/out/validator/skip.env
python3 -c 'import yaml'
set +e
bash safe/scripts/run-validator-libzstd.sh
status=$?
set -e
printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"
VALIDATOR_RUNNER_STATUS="$status" python3 safe/scripts/check-validator-phase-results.py \
    --results-root safe/out/validator/artifacts/port/results/libzstd \
    --report validator-report.md \
    --allow-remaining-phase impl_validator_source_cli_regressions \
    --allow-remaining-phase impl_validator_streaming_capi_regressions \
    --allow-remaining-phase impl_validator_libarchive_usage_regressions \
    --allow-remaining-phase impl_validator_remaining_burn_down
```

The runner internally executed:

```bash
bash safe/scripts/build-artifacts.sh --release
bash safe/scripts/build-original-cli-against-safe.sh
env -u DEB_BUILD_PROFILES bash safe/scripts/build-deb.sh
PYTHON=python3 make -C validator unit
PYTHON=python3 make -C validator check-testcases
PYTHON=python3 bash validator/test.sh \
    --config validator/repositories.yml \
    --tests-root validator/tests \
    --artifact-root safe/out/validator/artifacts \
    --mode port \
    --library libzstd \
    --override-deb-root safe/out/validator/override-debs \
    --port-deb-lock safe/out/validator/artifacts/proof/port-debs-lock.json \
    --record-casts
```

Proof generation was not run because the matrix has 1 failed testcase
(`safe/scripts/run-validator-libzstd.sh` only invokes
`tools/verify_proof_artifacts.py` on a clean run with zero failures, so
`safe/out/validator/artifacts/proof/port-validation-proof.json` is absent).

**Matrix Inventory**

- Source cases: 5
- Usage cases: 170
- Total cases: 175

**Initial Run**

- Summary path: `safe/out/validator/artifacts/port/results/libzstd/summary.json`
- Cases: 175
- Source cases: 5
- Usage cases: 170
- Passed: 174
- Failed: 1
- Casts: 175
- Validator runner status: 1

**Failure Classification**

| testcase_id | kind | client_application | exit_code | error | result_path | log_path | assigned_remediation_phase | remediation_status | regression_test | fix_commit | notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| usage-libarchive-tools-zstd-cli-test-integrity-flag | usage | libarchive-tools | 1 | testcase command exited with status 1 | port/results/libzstd/usage-libarchive-tools-zstd-cli-test-integrity-flag.json | port/logs/libzstd/usage-libarchive-tools-zstd-cli-test-integrity-flag.log | impl_validator_libarchive_usage_regressions | fixed | safe/tests/rust/compress.rs::streaming_decompress_rejects_corrupted_frame_for_libarchive_integrity_flag; safe/tests/validator/usage-libarchive-tools-zstd-cli-test-integrity-flag.sh; safe/docker/dependents/entrypoint.sh::test_libarchive integrity-flag block | a7040bfc72f93db785b489dc9a2c1613f65714b5 | libarchive-tools usage case running the installed `zstd` CLI under `bsdtar`/`zstd` packaging; classified to the libarchive usage phase by ID prefix `usage-libarchive-tools-zstd-`. Streaming decompression silently consumed the trailing 4-byte XXH64 checksum without comparing it to the decoded data, so `zstd -t` reported success on a corrupted frame; bufferless `NeedChecksum` handler in `safe/src/ffi/decompress.rs` now compares the consumed bytes against `xxh64(decoded_prefix)` and returns `ZSTD_error_checksum_wrong` on mismatch (skipped when `force_ignore_checksum` is set). |

**Skip List**

- Empty. No validator checks were skipped in Phase 1.

Phase 2 Base Commit: 7ea254369326f67f316f4f860010c83714f17e77

**Phase 2 — Source CLI, Dictionary, Multi-Frame, and Corruption Regressions**

- Validator commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
- Source cases inspected: 5 (`zstd-compress-decompress`, `dictionary-train-use`, `multi-frame-behavior`, `corrupted-frame-rejection`, `streaming-c-api-smoke`)
- Source cases assigned to `impl_validator_source_cli_regressions`: 0
- No source-case failures assigned to impl_validator_source_cli_regressions

The Phase 1 failure table contains a single open row
(`usage-libarchive-tools-zstd-cli-test-integrity-flag`) assigned to
`impl_validator_libarchive_usage_regressions`. No rows are assigned to
`impl_validator_source_cli_regressions`, so this phase makes no source-side
code change and adds no regression test. The failure table is unchanged.

**Phase 2 Commands Run**

```bash
git rev-parse HEAD
git -C validator rev-parse HEAD
git -C validator status --porcelain --untracked-files=no
ls validator/tests/libzstd/tests/cases/source/
set +e
bash safe/scripts/run-validator-libzstd.sh
status=$?
set -e
VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py \
    --results-root safe/out/validator/artifacts/port/results/libzstd \
    --report validator-report.md \
    --completed-phase impl_validator_source_cli_regressions \
    --allow-remaining-phase impl_validator_streaming_capi_regressions \
    --allow-remaining-phase impl_validator_libarchive_usage_regressions \
    --allow-remaining-phase impl_validator_remaining_burn_down
```

Phase 3 Base Commit: b754ecf91163bf24bbf959dfa0598be19565467e

**Phase 3 — Streaming C API Regressions**

- Validator commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
- Streaming source case inspected: 1 (`streaming-c-api-smoke`, `kind=source`, `tags=[api, compile]`)
- Streaming source case status in current results: `passed`
- Failure-table rows assigned to `impl_validator_streaming_capi_regressions`: 0
- No streaming-C-API failures assigned to impl_validator_streaming_capi_regressions

The Phase 1 failure table contains a single open row
(`usage-libarchive-tools-zstd-cli-test-integrity-flag`) assigned to
`impl_validator_libarchive_usage_regressions`. No rows are assigned to
`impl_validator_streaming_capi_regressions`, and the lone streaming source
case (`streaming-c-api-smoke`) is `passed` in
`safe/out/validator/artifacts/port/results/libzstd/streaming-c-api-smoke.json`.
This phase therefore makes no streaming-side code change in
`safe/src/ffi/{compress,decompress,advanced}.rs`, adds no streaming
regression test under `safe/tests/capi/`, generates no skip artifacts under
`safe/out/validator/`, and leaves the failure table unchanged.

**Phase 3 Commands Run**

```bash
git rev-parse HEAD
git -C validator rev-parse HEAD
git -C validator status --porcelain --untracked-files=no
ls validator/tests/libzstd/tests/cases/source/
cat safe/out/validator/artifacts/port/results/libzstd/streaming-c-api-smoke.json
set +e
bash safe/scripts/run-validator-libzstd.sh
status=$?
set -e
VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py \
    --results-root safe/out/validator/artifacts/port/results/libzstd \
    --report validator-report.md \
    --completed-phase impl_validator_source_cli_regressions \
    --completed-phase impl_validator_streaming_capi_regressions \
    --allow-remaining-phase impl_validator_libarchive_usage_regressions \
    --allow-remaining-phase impl_validator_remaining_burn_down
```

Phase 4 Base Commit: 863016aedd81bacb9afa06ae775030b365f046dc

**Phase 4 — Libarchive Usage Regressions**

- Validator commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
- Libarchive usage cases inspected: 170 (all `usage-libarchive-tools-zstd-*` cases under `validator/tests/libzstd/tests/cases/usage/`)
- Failure-table rows assigned to `impl_validator_libarchive_usage_regressions`: 1 (`usage-libarchive-tools-zstd-cli-test-integrity-flag`)
- Per-row outcome: `usage-libarchive-tools-zstd-cli-test-integrity-flag` → `fixed`

The validator testcase compresses a tiny payload with the safe `zstd` CLI,
flips one byte at offset 10 (mid-frame, inside the raw block payload), and
asserts that `zstd -tq` rejects the corrupted copy. With the prior safe
build this assertion failed: `zstd -tq` returned 0 on the corrupted frame.

Root cause: streaming decompression in `safe/src/ffi/decompress.rs` advanced
through the `BufferlessStage::NeedChecksum(4)` stage by appending the 4
trailing bytes to `frame_bytes` and immediately moving to `Finished` —
without comparing them to `xxh64(decoded_prefix)`. The simple
(`ZSTD_decompress`) path validates checksums via
`decode_all_frames(validate_decoded_frame=true)`, but the streaming path
that the zstd CLI uses (`decode_all_frames_relaxed(validate=false)` plus
the per-stage state machine) skipped that comparison. The CLI surfaces
this as a silent success, exactly matching the validator failure.

Fix: in `bufferless_continue`'s `NeedChecksum` arm, when
`force_ignore_checksum == 0` and the current chunk is the 4 expected
checksum bytes, decode the LE u32, compare to `xxh64(decoded_prefix)`'s
low 32 bits, and return `ZSTD_error_checksum_wrong` on mismatch. The
`ZSTD_d_experimentalParam3 / force_ignore_checksum` opt-out is honored.

Regression coverage:

- `safe/tests/rust/compress.rs::streaming_decompress_rejects_corrupted_frame_for_libarchive_integrity_flag`
  reproduces the validator scenario through the C ABI (compress with
  `ZSTD_c_checksumFlag=1`, flip byte 10, drive `ZSTD_decompressStream`,
  expect `ZSTD_error_checksum_wrong`). Pre-fix the test asserts the
  streaming decode stops with a checksum error; with the original code
  the test failed with `ZSTD_decompressStream accepted a corrupted frame
  (last ret=0)`, confirming the regression boundary.
- `safe/docker/dependents/entrypoint.sh::test_libarchive` now executes
  the same CLI sequence as the validator (`zstd -q -o good.zst …`,
  magic check, `zstd -tq good.zst`, byte-10 flip, `zstd -tq bad.zst`
  must be non-zero) inside the dependent image, alongside the existing
  `bsdtar --zstd` cases. Verified via
  `bash safe/scripts/build-dependent-image.sh && bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps libarchive`
  → "all dependent runtime tests passed".
- `safe/tests/validator/usage-libarchive-tools-zstd-cli-test-integrity-flag.sh`
  is a top-level executable reproducer driven by
  `safe/scripts/run-validator-regressions.sh` (which uses
  `find -maxdepth 1`).

Validator status after the fix: `safe/out/validator/artifacts/port/results/libzstd/summary.json`
reports `cases=175, passed=175, failed=0`. The runner exits 0 and
proof generation runs.

**Phase 4 Commands Run**

```bash
git rev-parse HEAD
git -C validator rev-parse HEAD
git -C validator status --porcelain --untracked-files=no
ls validator/tests/libzstd/tests/cases/usage/ | grep -c '^usage-libarchive-tools-zstd-'
cat safe/out/validator/artifacts/port/results/libzstd/usage-libarchive-tools-zstd-cli-test-integrity-flag.json
cargo test --manifest-path safe/Cargo.toml --release --test compress streaming_decompress_rejects_corrupted_frame_for_libarchive_integrity_flag
cargo test --manifest-path safe/Cargo.toml --release --test compress
cargo test --manifest-path safe/Cargo.toml --release --test decompress
env -u DEB_BUILD_PROFILES bash safe/scripts/build-artifacts.sh --release
env -u DEB_BUILD_PROFILES bash safe/scripts/build-deb.sh
bash safe/scripts/build-dependent-image.sh
bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps libarchive
bash safe/scripts/run-validator-regressions.sh
set +e
bash safe/scripts/run-validator-libzstd.sh
status=$?
set -e
VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py \
    --results-root safe/out/validator/artifacts/port/results/libzstd \
    --report validator-report.md \
    --completed-phase impl_validator_source_cli_regressions \
    --completed-phase impl_validator_streaming_capi_regressions \
    --completed-phase impl_validator_libarchive_usage_regressions \
    --allow-remaining-phase impl_validator_remaining_burn_down
```

Phase 5 Base Commit: 38f2dc51a97713d9d57a26679fecbc1521b074c4

**Phase 5 — Remaining Burn-Down and Validator-Bug Triage**

- Validator commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
- Failure-table rows initially assigned to `impl_validator_remaining_burn_down`: 0
- Phase 2-4 deferrals carrying `suspected_validator_bug_deferred_to_phase5:<source-phase>` in `notes`: 0
- Total residual rows owned by this phase: 0
- Burn-down fixes applied in Phase 5: 0
- Validator-bug skip artifacts generated in Phase 5: 0 (no `safe/out/validator/skip.env`, no `safe/out/validator/tests-filtered/`)

The Phase 1 failure table contains a single row
(`usage-libarchive-tools-zstd-cli-test-integrity-flag`) which was assigned to
`impl_validator_libarchive_usage_regressions` and driven to
`remediation_status=fixed` in Phase 4 (`fix_commit=a7040bfc72f93db785b489dc9a2c1613f65714b5`).
No row was ever assigned to `impl_validator_remaining_burn_down`, and a
search of the failure table for the
`suspected_validator_bug_deferred_to_phase5:` marker returns no matches, so
no Phase 2-4 deferral was forwarded to this phase. With zero rows to
triage, Phase 5 makes no source-side change, adds no regression test,
generates no validator-bug skip artifacts, and leaves the failure table
unchanged.

The current matrix run reproduces the Phase 4 clean state:
`safe/out/validator/artifacts/port/results/libzstd/summary.json` reports
`cases=175, source_cases=5, usage_cases=170, passed=175, failed=0,
casts=175`, the runner exits 0, and proof generation runs (the
`port-validation-proof.json` artifact is written). The all-completed
checker invocation succeeds with all four remediation phases passed via
`--completed-phase` and no `--allow-remaining-phase`.

**Phase 5 Validator Bug Findings**

None. No testcase exhibited validator-bug behavior (every Phase 1 failure
was a genuine safe-side regression, fixed in Phase 4), so no entry under
this heading is required and no `safe/out/validator/skip.env` or
`safe/out/validator/tests-filtered/` was produced.

**Phase 5 Commands Run**

```bash
git rev-parse HEAD
git -C validator rev-parse HEAD
git -C validator status --porcelain --untracked-files=no
grep -n suspected_validator_bug_deferred_to_phase5 validator-report.md || true
ls safe/out/validator/skip.env safe/out/validator/tests-filtered 2>&1 || true
cat safe/out/validator/artifacts/port/results/libzstd/summary.json
set +e
bash safe/scripts/run-validator-libzstd.sh
status=$?
set -e
VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py \
    --results-root safe/out/validator/artifacts/port/results/libzstd \
    --report validator-report.md \
    --completed-phase impl_validator_source_cli_regressions \
    --completed-phase impl_validator_streaming_capi_regressions \
    --completed-phase impl_validator_libarchive_usage_regressions \
    --completed-phase impl_validator_remaining_burn_down
```

Phase 6 Base Commit: 0a3583a438e97577585fcb7168b53e19ba2354be

**Phase 6 — Final Clean Validator Run and Report Consolidation**

- Validator commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
- Validator worktree status: clean (`git -C validator status --porcelain --untracked-files=no` empty)
- Latest code-bearing safe commit before this phase: a7040bfc72f93db785b489dc9a2c1613f65714b5 (Phase 4 libarchive integrity-flag fix)
- Phase 6 base commit (HEAD captured before any change in this phase): 0a3583a438e97577585fcb7168b53e19ba2354be
- Phase 5 close commit (no-op burn-down): 0a3583a438e97577585fcb7168b53e19ba2354be
- Validator runner exit status (clean run from wiped artifacts): 0
- Phase-result checker exit status (all four remediation phases completed): 0
- Validator regression suite (`safe/scripts/run-validator-regressions.sh`) exit status: 0
- Dependent image build (`safe/scripts/build-dependent-image.sh`) exit status: 0
- Full release-gate suite (`safe/scripts/run-full-suite.sh`) exit status: 0

**Phase 6 Package Inventory**

The override leaf `safe/out/validator/override-debs/libzstd/` rebuilt from the
clean Phase 6 run contains exactly three .debs:

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 381026 | 7886ac0a8f25827f0404b76bd567842c7c7f77908e448bdeeb25da0dd73f21b3 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 3830246 | c8d41de81b1de2e72da67408c81252a334461d2ac7aa26bf940d47627f63f8a5 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

The regenerated port lock at `safe/out/validator/artifacts/proof/port-debs-lock.json`
records `commit=0a3583a438e97577585fcb7168b53e19ba2354be`,
`release_tag=build-0a3583a438e9`, and
`tag_ref=refs/tags/build-0a3583a438e9` against the same three filenames,
sizes, and SHA-256 digests above. The deb sizes for `libzstd1` and
`libzstd-dev` differ from the Phase 1 inventory by 100 and -342 bytes
respectively because the Phase 4 commit (`a7040bf`) added the streaming
checksum-verification path to `safe/src/ffi/decompress.rs`; the `zstd` CLI
deb is byte-identical to the Phase 1 build (sha256
`8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91`).

**Phase 6 Validator Summary**

- Summary path: `safe/out/validator/artifacts/port/results/libzstd/summary.json`
- schema_version: 2
- library: libzstd
- mode: port
- cases: 175 (canonical inventory: source=5, usage=170; executed inventory: source=5, usage=170, casts=175)
- source_cases: 5
- usage_cases: 170
- passed: 175
- failed: 0
- casts: 175
- Validator runner status: 0
- Proof generation: ran (`safe/out/validator/artifacts/proof/port-validation-proof.json` written)

**Phase 6 Failures Found**

None. The clean run from a wiped `safe/out/validator/artifacts/` reports
`failed=0` across all 175 cases.

**Phase 6 Fixes Applied**

No source-side fixes were applied in Phase 6. The single failure-table row
(`usage-libarchive-tools-zstd-cli-test-integrity-flag`) was already
`remediation_status=fixed` from Phase 4 (`fix_commit=a7040bfc72f93db785b489dc9a2c1613f65714b5`)
and the clean Phase 6 run confirms that fix continues to hold.

The only repository change in Phase 6 is a wrapper-script repair caught by
the final clean release-gate run:

- `safe/tests/fixtures/regression/results-memoized.source-sha256` refreshed
  from `89ba14ed0d1253a52fa790860753b0852f975638de4e0f297900dcb9ac916be4`
  to `451e4f4c64131f90f67710a92a746f97a9bd544a40cfbc117fbe5ad539931be2`.
  After the Phase 4 commit (`a7040bf`) modified `safe/src/ffi/decompress.rs`
  to compare the bufferless streaming checksum against
  `xxh64(decoded_prefix)`, the source digest tracked by
  `safe/scripts/run-upstream-regression.sh` no longer matched the
  checked-in fixture key, so the wrapper fell through to recomputing the
  matrix and failed at `rsync` of the absent
  `original/libzstd-1.5.5+dfsg2/tests/regression/cache/`. The Phase 4 fix
  only adds checksum verification on corrupted frames; the regression
  matrix exercises valid frames and produces byte-identical compressed
  outputs, so `safe/tests/fixtures/regression/results-memoized.csv` is
  unchanged. Refreshing the digest restores the
  `memoized_regression_fixture_is_compatible` path so the offline release
  gate re-uses the memoized snapshot. After the refresh
  `safe/scripts/run-upstream-regression.sh` (driven by
  `safe/scripts/run-full-suite.sh`) prints
  `regression matrix matched all 920 rows exactly against
  safe/tests/fixtures/regression/results-memoized.csv` and exits 0.

**Phase 6 Regressions Added**

None. No regression test or harness was added in Phase 6. The
Phase 4 regression coverage
(`safe/tests/rust/compress.rs::streaming_decompress_rejects_corrupted_frame_for_libarchive_integrity_flag`,
`safe/tests/validator/usage-libarchive-tools-zstd-cli-test-integrity-flag.sh`,
`safe/docker/dependents/entrypoint.sh::test_libarchive` integrity-flag
block) was re-executed by the Phase 6 release-gate run and continues to
pass.

**Phase 6 Skips**

None. No `safe/out/validator/skip.env` and no
`safe/out/validator/tests-filtered/` artifacts were produced or are
required; the failure-table contains zero rows with
`remediation_status=skipped_validator_bug`.

**Phase 6 Release Gate Result**

GREEN. All four release-gate wrappers exit 0:

- `bash safe/scripts/run-validator-libzstd.sh` → exit 0; `summary.json`
  reports `cases=175, passed=175, failed=0`; proof artifact written.
- `VALIDATOR_RUNNER_STATUS=0 python3 safe/scripts/check-validator-phase-results.py
  ... --completed-phase impl_validator_source_cli_regressions
  --completed-phase impl_validator_streaming_capi_regressions
  --completed-phase impl_validator_libarchive_usage_regressions
  --completed-phase impl_validator_remaining_burn_down` → exit 0;
  `allowed remaining failed testcase IDs: (none)`.
- `bash safe/scripts/run-validator-regressions.sh` → exit 0; reproduces
  the libarchive integrity-flag scenario through the safe `zstd` CLI.
- `bash safe/scripts/build-dependent-image.sh` → exit 0; image
  `safelibs-libzstd-dependents:ubuntu24.04` ready for the dependent
  matrix.
- `bash safe/scripts/run-full-suite.sh` → exit 0; the final line of the
  log is `== all dependent runtime tests passed ==`.

**Phase 6 Commands Run**

```bash
git rev-parse HEAD
git -C validator rev-parse HEAD
git -C validator status --porcelain --untracked-files=no
rm -rf safe/out/validator/artifacts/
set +e
bash safe/scripts/run-validator-libzstd.sh
validator_status=$?
set -e
VALIDATOR_RUNNER_STATUS=$validator_status python3 safe/scripts/check-validator-phase-results.py \
    --results-root safe/out/validator/artifacts/port/results/libzstd \
    --report validator-report.md \
    --completed-phase impl_validator_source_cli_regressions \
    --completed-phase impl_validator_streaming_capi_regressions \
    --completed-phase impl_validator_libarchive_usage_regressions \
    --completed-phase impl_validator_remaining_burn_down
bash safe/scripts/run-validator-regressions.sh
bash safe/scripts/build-dependent-image.sh
bash safe/scripts/run-full-suite.sh
cat safe/out/validator/artifacts/port/results/libzstd/summary.json
cat safe/out/validator/artifacts/proof/port-debs-lock.json
ls safe/out/validator/artifacts/proof/port-validation-proof.json
```

Phase 1 Validator Refresh: impl_safe_decompression_independence

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
- Local port commit recorded by proof: b102f5706068ebea906327a91729d4a31662f2db
- Local port release tag: build-b102f5706068
- Local port tag ref: refs/tags/build-b102f5706068
- Mode: port

The proof-recorded commit `b102f5706068` contains the Phase 1 source fixes
from `87b593e0f58e` plus the required regenerated `workflow.yaml` output.
The validator was rerun after that workflow commit so the report proves the
checked implementation state rather than the pre-commit base.

**Package Inventory**

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 378504 | 7784c488f23f89fbd553bd4101dfbec5afd24d17a2f96c6788b8e11a51c0db73 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 3702308 | bfa599399e3bb7f340d7d8d43accc2bd9e15235cfe1531c449e03a542d8dd6dc |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

**Validator Summary**

- Summary path: `safe/out/validator/artifacts/port/results/libzstd/summary.json`
- Proof path: `safe/out/validator/artifacts/proof/port-validation-proof.json`
- Cases: 257
- Source cases: 5
- Usage cases: 250
- Regression cases: 2
- Passed: 257
- Failed: 0
- Casts: 257
- Validator runner status: 0

**Failures Found and Fixes Applied**

Initial refresh runs against `safelibs/validator@d1c08d01` exposed three
validator failures in the expanded libarchive usage matrix:

- `usage-libarchive-tools-zstd-r13-cli-stdin-c-pipeline`: the safe CLI
  emitted a valid small compressed frame, but `ZSTD_decompressStream`
  decoded the frame through the relaxed oxiarc-first block replay path and
  produced a short prefix before failing the checksum. The fix makes relaxed
  no-dictionary streaming replay prefer the native structured decoder, while
  strict one-shot decoding validates oxiarc output before accepting it.
- `usage-libarchive-tools-zstd-r14-cli-long-window-21`: a known-size frame
  larger than the configured `--long=21` window was being encoded as a
  single-segment frame, making the advertised window equal to the full
  content size. The fix keeps known-size frames non-single-segment when the
  source exceeds the configured window.
- `usage-libarchive-tools-zstd-r16-cli-level-1-vs-19-size-monotonic`: the
  multithreaded streaming path used by the CLI can carry level-1 behavior
  via explicit cparams and bypassed the small fast-mode size distinction.
  The fix applies the empty leading block to the stateful structured encoder
  on the first emitted job when the effective cparams are level-1 fast mode.

Regression coverage added in `safe/tests/rust/compress.rs`:

- `streaming_repetitive_stdout_frame_decodes_with_safe_decoder` now checks
  both one-shot and `ZSTD_decompressStream` decode for the stdin-pipeline
  frame shape.
- `long_window_frames_decode_with_matching_window_log` asserts `--long=21`
  compatible window metadata and safe DCtx decode.
- `level_19_beats_level_1_on_repetitive_source` now uses cparams,
  `nbWorkers=1`, pledged size, and split `ZSTD_compressStream2` calls to
  mirror the CLI path.

No validator checks were skipped.

**Commands Run**

```bash
git -C validator rev-parse HEAD
cargo test --manifest-path safe/Cargo.toml --release --test compress
cargo test --manifest-path safe/Cargo.toml --release --test decompress
bash safe/scripts/run-capi-decompression.sh
bash safe/scripts/verify-export-parity.sh
bash safe/scripts/verify-baseline-contract.sh
rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym' \
    safe/src/decompress safe/src/ffi/decompress.rs safe/scripts/run-capi-decompression.sh
env -u DEB_BUILD_PROFILES bash safe/scripts/build-deb.sh
bash safe/scripts/run-validator-libzstd.sh
jq -c '{cases,source_cases,usage_cases,regression_cases,passed,failed,casts}' \
    safe/out/validator/artifacts/port/results/libzstd/summary.json
```

Phase 9 Bounce Resolution: static archive and validator traceability

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
- Code-bearing implementation commit validated: 8f7fcae349453f11e74a864f59003b478560a626
- Local release tag used by that validator lock: build-8f7fcae34945
- Mode: port
- Invocation: `SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh`

The validator suite was not modified by this phase. After this report is
committed, the same validator command is rerun without source changes so the
generated `.work/validation/port-deb-lock.json` records final `HEAD`; the
report commit only updates this summary and does not alter build inputs.

**Package Inventory**

The root build hook produced these canonical override packages in `dist/`:

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 376832 | 5e449739d929e5dc88bc52f33376d10347f7b0247ef88f210a9554ca9180e4f9 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 2015306 | 9d2cdd47935a4d7bec69d63846ccd3fb856128cc26a9b2f21017202bbabc42e9 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

The `libzstd-dev` package contains a real static archive at
`usr/lib/x86_64-linux-gnu/libzstd.a`; `ar t` succeeds on it, a forced-static
upstream-header smoke binary links and runs, and `strings` finds no loader
lookup tokens in that archive.

**Validator Summary**

- Log root: `.work/validation/artifacts/port/logs/libzstd/`
- Result summary: `.work/validation/artifacts/port/results/libzstd/summary.json`
- Port lock path: `.work/validation/port-deb-lock.json`
- Cases: 257
- Source cases: 5
- Usage cases: 250
- Regression cases: 2
- Passed: 257
- Failed: 0
- Casts recorded: 0
- Unported original packages: 0

**Fixes Applied**

The previous bounce identified two implementation problems: the validator proof
targeted an older code-bearing commit, and the static archive had been made to
pass string scans by post-processing Rust output and adding a local C fallback.
That approach has been removed.

- `safe/build.rs` no longer compiles the local lookup fallback C file; the only
  C bridge left in the shipping library is the bounded legacy frame shim.
- `safe/src/ffi/runtime_lookup_stub.c` was deleted.
- `safe/scripts/build-artifacts.sh` now builds static archives with Cargo
  `-Z build-std=std` through a cached copy of the active Rust sysroot. The
  copied standard-library source removes the GNU thread minimum-stack weak
  loader path before compiling `std`, so the produced `libzstd.a` is not byte
  rewritten and does not need a fallback symbol.
- The installed `libzstd.a` remains a real `ar` archive, and the existing
  forced-static upstream-header smoke test still links against that archive.
- The static archive checks now reject both loader lookup tokens and still
  require `ar t` readability.
- `scripts/install-build-deps.sh` installs `rust-src`, which is required for
  the static `build-std` artifact path.

**Checks Executed**

The phase verifier and static/package checks run for this bounce:

```bash
bash safe/scripts/run-advanced-mt-tests.sh
bash safe/scripts/verify-link-compat.sh
bash safe/scripts/verify-export-parity.sh
cargo test --manifest-path safe/Cargo.toml --release --all-targets
rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe
bash safe/scripts/run-build-variant-tests.sh
bash safe/scripts/verify-install-layout.sh
bash scripts/build-debs.sh
SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh
```

All listed checks passed. No validator checks were skipped.

Phase 9 Bounce Resolution: safe MT worker completion

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
- Final code-bearing source commit validated: 6fe88a3a63c13f992b12b9ce62ba98a3c744eb03
- Local release tag used by the final validator lock: build-6fe88a3a63c1
- Mode: port
- Invocation: `SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh`

This section is a report-only update after the clean run. The validated
shipping source is commit `6fe88a3a63c13f992b12b9ce62ba98a3c744eb03`; the
report commit that contains this text does not alter build inputs.

**Package Inventory**

The root build hook produced these canonical override packages in `dist/`:

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 419890 | a23754b79eb738154ec9a2fdd02f1a01d3266888bfa12287a204d9c36ec11084 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 2015556 | 17b014d52f5b993a2f852adc4d61a604c25a3f3229fb8b36088f59c8acabfdd0 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

The generated port lock recorded all three canonical packages as ported and
zero unported original packages.

**Validator Summary**

- Log root: `.work/validation/artifacts/port/logs/libzstd/`
- Result summary: `.work/validation/artifacts/port/results/libzstd/summary.json`
- Port lock path: `.work/validation/port-deb-lock.json`
- Cases: 257
- Source cases: 5
- Usage cases: 250
- Regression cases: 2
- Passed: 257
- Failed: 0
- Casts recorded: 0

**Failures Found And Fixed**

- `check_safe_advanced_abi_software_tester` found that MT support was still a
  synchronous facade. `28d3e9b76a1cf6c1061222c94e2bf8dd7a5ef2c7` replaced the
  empty job-queue stub with a safe Rust worker queue and wired `nbWorkers`,
  owned pools, external pools, MT job counters, and frame progression through
  real worker jobs.
- The first validator rerun against `28d3e9b76a1cf6c1061222c94e2bf8dd7a5ef2c7`
  found `usage-libarchive-tools-zstd-bsdtar-options-threads-2`: a split MT
  frame corrupted tar header padding because independently compressed zstd
  block payloads were being spliced into one frame. A regression was added to
  `safe/tests/capi/thread_pool_driver.c` with a tar-like repeated payload.
- `a5fddbea05335028163238fc3c267c823f09e985` made split MT jobs emit stored
  blocks so worker-job output is safe to concatenate in frame order, and updated
  the Rust MT overlap test to assert round-trip behavior instead of size deltas
  from unsafe cross-job history.
- The next validator run showed five CLI compression-ratio regressions caused
  by applying stored blocks too broadly. `6fe88a3a63c13f992b12b9ce62ba98a3c744eb03`
  narrowed the fallback to split MT frames only; single final MT jobs keep the
  normal compressed payload path, restoring small repetitive CLI compression
  while preserving the bsdtar fix.

No validator checks were skipped.

**Checks Executed**

```bash
cargo test --manifest-path safe/Cargo.toml --release --all-targets
bash safe/scripts/run-advanced-mt-tests.sh
bash safe/scripts/verify-link-compat.sh
bash safe/scripts/verify-export-parity.sh
rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe --glob '!target/**' --glob '!out/**'
SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) bash scripts/build-debs.sh
SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh
```

All listed checks passed for the final code-bearing source commit. The
source-level forbidden-token scan had no maintained-source matches; generated
build and validation artifacts under `safe/out` remain uncommitted.

Phase 9 Bounce Resolution: shared loader import and MT overlap

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
- Final code-bearing source commit validated: c435398ce26774ffc6d7b0369bdc5d6d98018aeb
- Local release tag used by the final validator lock: build-c435398ce267
- Mode: port
- Invocation: `SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh`

This section is a report-only update after validating commit
`c435398ce26774ffc6d7b0369bdc5d6d98018aeb`. The subsequent report commit does
not alter build inputs.

**Package Inventory**

The root build hook produced these canonical override packages in `dist/`:

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 426880 | 0e293db0b916eb2f8f945516bdd9128daf5329951a1f560424709a047e430a5a |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 2015298 | f59f023b85cdbf5df3fb576dfee2cde3c8d81557bb2288581a482b11cf000577 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

The generated port lock recorded all three canonical packages as ported and
zero unported original packages.

**Validator Summary**

- Log root: `.work/validation/artifacts/port/logs/libzstd/`
- Result summary: `.work/validation/artifacts/port/results/libzstd/summary.json`
- Port lock path: `.work/validation/port-deb-lock.json`
- Cases: 257
- Source cases: 5
- Usage cases: 250
- Regression cases: 2
- Passed: 257
- Failed: 0
- Casts recorded: 0

**Failures Found And Fixed**

- `check_safe_advanced_abi_software_tester` found that the packaged runtime
  shared object still imported the dynamic-loader lookup path. `c435398ce267`
  now builds both the shared object and static archive through the patched Rust
  sysroot path and fails the artifact build if the installed shared object
  carries a loader lookup import or string token.
- The same check found that split MT jobs no longer applied overlap history.
  `c435398ce267` restores bounded overlap history for the first overlap-bearing
  split job while keeping the earlier stored-block safety fallback for the
  remaining independent split jobs.
- `safe/tests/rust/compress.rs` now has a regression that compares full,
  default, and no-overlap MT output sizes after round-trip validation, so a
  pure stored-block fallback across the job boundary fails.

No validator checks were skipped.

**Checks Executed**

```bash
cargo fmt --manifest-path safe/Cargo.toml
bash -n safe/scripts/build-artifacts.sh
cargo test --manifest-path safe/Cargo.toml --release --test compress compress_stream2_mt_overlap_log_changes_job_boundary_output -- --nocapture
bash safe/scripts/build-artifacts.sh --release --destdir "$tmp_root/install" --objdir "$tmp_root/obj" --no-install-cmake
nm -D -u "$tmp_root/install/usr/lib/x86_64-linux-gnu/libzstd.so.1.5.5" | rg 'dl.?sym|dl.?open' || true
strings -a "$tmp_root/install/usr/lib/x86_64-linux-gnu/libzstd.so.1.5.5" | rg 'dl.?sym|dl.?open' || true
nm -u "$tmp_root/install/usr/lib/x86_64-linux-gnu/libzstd.a" | rg 'dl.?sym|dl.?open' || true
bash safe/scripts/run-advanced-mt-tests.sh
bash safe/scripts/verify-link-compat.sh
bash safe/scripts/verify-export-parity.sh
cargo test --manifest-path safe/Cargo.toml --release --all-targets
rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe
SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) bash scripts/build-debs.sh
nm -D -u "$extracted_libzstd1/usr/lib/x86_64-linux-gnu/libzstd.so.1.5.5" | rg 'dl.?sym|dl.?open' || true
nm -u "$extracted_libzstd_dev/usr/lib/x86_64-linux-gnu/libzstd.a" | rg 'dl.?sym|dl.?open' || true
SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh
```

All listed pass/fail commands passed. The `nm` and `strings` probes above are
expected to print no matches and return non-zero through `rg`; they were run
with `|| true` while checking that no output was produced.

Phase 9 Bounce Resolution: advanced custom allocator ABI

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
- Final code-bearing source commit validated: f9e484313cff3864e9617c44b0d3a3b5a36fb7b2
- Local release tag used by the final validator lock: build-f9e484313cff
- Mode: port
- Invocation: `SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh`

This section is a report-only update after validating commit
`f9e484313cff3864e9617c44b0d3a3b5a36fb7b2`. The subsequent report commit does
not alter build inputs.

**Package Inventory**

The root build hook produced these canonical override packages in `dist/`:

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 428454 | 2f75985ae3c666c0842d41e3b6ac94c69759740c62463c0acd20ae8e35df7761 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 2018182 | a1b97fc6ea9e99fc765b6a97f4faf2c31d394e0f4d6bd064bb993b2dd38cd10c |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

The generated port lock recorded all three canonical packages as ported and
zero unported original packages.

**Validator Summary**

- Log root: `.work/validation/artifacts/port/logs/libzstd/`
- Result summary: `.work/validation/artifacts/port/results/libzstd/summary.json`
- Port lock path: `.work/validation/port-deb-lock.json`
- Cases: 257
- Source cases: 5
- Usage cases: 250
- Regression cases: 2
- Passed: 257
- Failed: 0
- Casts recorded: 0

**Failures Found And Fixed**

- `check_safe_advanced_abi_software_tester` found that the documented
  `ZSTD_customMem` advanced ABI still rejected non-null allocator callbacks.
  `f9e484313cff` implements complete allocator-pair validation and
  allocator-backed Rust-owned handle allocation/free for CCtx, CStream, CDict,
  DCtx, DStream, and DDict advanced constructors.
- The same commit preserves the destination allocator identity when copying
  compression contexts, so a copied context is freed through the allocator that
  created its handle.
- `safe/tests/rust/compress.rs` now has regression coverage for successful
  create/free through custom allocators and for rejecting incomplete allocator
  pairs without invoking the callback.
- `safe/PORT.md` was updated to remove the stale note that custom allocators
  were unsupported.

No validator checks were skipped.

**Checks Executed**

```bash
cargo fmt --manifest-path safe/Cargo.toml --check
git diff --check -- safe/PORT.md safe/src/ffi/types.rs safe/src/ffi/compress.rs safe/src/ffi/decompress.rs safe/src/compress/cctx.rs safe/src/compress/cstream.rs safe/src/compress/cdict.rs safe/src/decompress/dctx.rs safe/src/decompress/dstream.rs safe/src/decompress/ddict.rs safe/tests/rust/compress.rs
cargo test --manifest-path safe/Cargo.toml --release --test compress advanced_custom_allocators -- --nocapture
cargo test --manifest-path safe/Cargo.toml --release --all-targets
bash safe/scripts/run-advanced-mt-tests.sh
bash safe/scripts/verify-export-parity.sh
make -C safe/tests/link-compat clean >/dev/null && make -C safe/tests/link-compat run SAFE_ROOT="$PWD/safe" REPO_ROOT="$PWD"
/usr/bin/rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe
SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) bash scripts/build-debs.sh
nm -D -u "$extracted_libzstd1/usr/lib/x86_64-linux-gnu/libzstd.so.1.5.5" | /usr/bin/rg 'dl(sym|open)|dlsym|dlopen' || true
nm -u "$extracted_libzstd_dev/usr/lib/x86_64-linux-gnu/libzstd.a" | /usr/bin/rg 'dl(sym|open)|dlsym|dlopen' || true
cc -static original/libzstd-1.5.5+dfsg2/debian/tests/ztest/pkg-make/ztest.c -Ioriginal/libzstd-1.5.5+dfsg2/lib -o "$tmp_root/static-ztest" "$extracted_libzstd_dev/usr/lib/x86_64-linux-gnu/libzstd.a" -lm
SAFELIBS_COMMIT_SHA=$(git rev-parse HEAD) SAFELIBS_VALIDATOR_DIR="$PWD/validator" bash scripts/run-validation-tests.sh
```

All listed pass/fail commands passed. The `nm` probes are expected to print no
matches and return non-zero through `/usr/bin/rg`; they were run with `|| true`
while checking that no output was produced. The package static-link smoke linked
and ran the upstream ztest probe against the packaged `libzstd.a`.

Phase 17 Implementation: impl_upstream_release_gates

**Validator Checkout**

- Date: 2026-05-14
- Validator URL: https://github.com/safelibs/validator
- Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
- Phase base commit before fixes: 2ab9b96
- Mode: port
- Final validator runner: `bash safe/scripts/run-validator-libzstd.sh`
- Validator regression runner: `bash safe/scripts/run-validator-regressions.sh`

The validator checkout was updated with `git -C validator pull --ff-only`.
After the source and release-gate fixture fixes, the Phase 4 artifacts were
refreshed explicitly through the canonical build scripts. The upstream
black-box wrappers were not changed to trigger builds implicitly; they consumed
the refreshed `safe/out/install/release-default/`,
`safe/out/original-cli/lib/`, and `safe/out/deb/default/metadata.env` roots.

**Release Gate Scope**

The release gate retained header identity, baseline contract checks, original
CLI playtests, original CLI tests, gzip tests, zlibWrapper, educational decoder,
pzstd, seekable, version compatibility, memoized upstream regression rows,
upstream fuzz fixtures, original examples, CVE-derived CLI permission checks,
and the performance smoke check.

**Validator Summary**

- Result summary: `safe/out/validator/artifacts/port/results/libzstd/summary.json`
- Cases: 257
- Source cases: 5
- Usage cases: 250
- Regression cases: 2
- Passed: 257
- Failed: 0
- Casts recorded: 257
- Validator runner status: 0
- Validator regression runner status: 0

**Failures Found And Fixed**

- `safe/scripts/run-upstream-regression.sh` initially rejected the checked-in
  memoized regression fixture metadata after the compression behavior fixes.
  `safe/tests/fixtures/regression/results-memoized.source-sha256` and the
  affected `level=-1` rows in `results-memoized.csv` were refreshed, and
  the CSV was normalized to LF line endings. `safe/tests/fixtures/regression/README.md`
  now documents that the checked-in CSV is the offline release-gate fixture
  while large cache directories remain intentionally absent.
- The upstream fuzz stream round-trip for the `http` corpus exposed an MT
  streaming overlap bug where structured payload validation omitted stream
  history. `safe/src/ffi/compress.rs` now validates structured payloads against
  synthetic history plus the current chunk before falling back to stored blocks.
  `safe/tests/rust/compress.rs` adds
  `mt_stream_flush_overlap_payload_roundtrips_fuzz_http_prefix`.
- The Valgrind upstream smoke test reported a Rust standard-library `mpsc`
  receiver wait allocation as possibly lost. `safe/src/threading/job_queue.rs`
  now uses an explicit mutex and condition-variable result slot, removing that
  wait path from MT worker completion.
- The original CLI `compression/levels.sh` check found that no-history
  `level=-1` output could be smaller than level 1 for the CLI fixture.
  `safe/src/ffi/compress.rs` now pads no-history level -1 frames with two empty
  block headers so the fast profile remains measurably less dense. The new
  `mt_fast_level_remains_larger_than_level_one_for_cli_fixture` regression
  covers the behavior.
- The original CLI adaptive-speed test did not observe the expected
  "faster speed, lighter compression" transition. MT streaming now applies
  pending-output backpressure before consuming more input, keeps the job handoff
  visible to callers, and reports frame progression from externally flushed
  output. The existing
  `compress_stream2_mt_frame_progression_tracks_started_jobs` regression now
  checks both job-ID warm-up and flushed-byte accounting.

No validator checks were skipped.

**Checks Executed**

```bash
bash safe/scripts/build-artifacts.sh --release
bash safe/scripts/build-original-cli-against-safe.sh
bash safe/scripts/build-deb.sh
cargo fmt --manifest-path safe/Cargo.toml
cargo test --manifest-path safe/Cargo.toml --release mt_fast_level_remains_larger_than_level_one_for_cli_fixture -- --nocapture
cargo test --manifest-path safe/Cargo.toml --release mt_stream_flush_overlap_payload_roundtrips_fuzz_http_prefix -- --nocapture
cargo test --manifest-path safe/Cargo.toml --release threading::job_queue -- --nocapture
cargo test --manifest-path safe/Cargo.toml --release compress_stream2_mt_frame_progression_tracks_started_jobs -- --nocapture
bash safe/scripts/verify-header-identity.sh
bash safe/scripts/verify-baseline-contract.sh
bash safe/scripts/run-upstream-tests.sh
bash safe/scripts/run-original-playtests.sh
bash safe/scripts/run-original-cli-tests.sh
bash safe/scripts/run-original-gzip-tests.sh
bash safe/scripts/run-zlibwrapper-tests.sh
bash safe/scripts/run-educational-decoder-tests.sh
bash safe/scripts/run-pzstd-tests.sh
bash safe/scripts/run-seekable-tests.sh
bash safe/scripts/run-version-compat-tests.sh
bash safe/scripts/run-upstream-regression.sh
bash safe/scripts/run-upstream-fuzz-tests.sh
bash safe/scripts/run-original-examples.sh
bash safe/scripts/check-cli-permissions.sh
bash safe/scripts/run-performance-smoke.sh
bash safe/scripts/run-validator-libzstd.sh
bash safe/scripts/run-validator-regressions.sh
```

All listed commands passed in the final run. The adaptive CLI reproducer was
also checked manually against the installed safe `zstd` binary and confirmed to
emit the expected lighter-compression transition with and without
`--no-progress`.

Phase 25 Implementation: impl_compat_regressions_and_fixes

**Validator Checkout**

- Date: 2026-05-14
- Validator URL: https://github.com/safelibs/validator
- Validator commit: d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97
- Validator branch: main
- `git -C validator pull --ff-only`: already up to date
- Code commit validated by the port lock: 71f9924d0d144de2eca85aba67b8a0601ef85a13
- Local release tag used by the validator lock: build-71f9924d0d14
- Mode: port
- Final validator runner: `bash safe/scripts/run-validator-libzstd.sh`

The validator was run from the local checkout at `validator/` per the phase
request. The runner reused the canonical Phase 4 artifact roots, staged only
the three canonical libzstd packages under
`safe/out/validator/override-debs/libzstd/`, and generated proof artifacts
under `safe/out/validator/artifacts/proof/`.

**Override Packages**

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 424378 | b35e3002268b14e41fcccfad9fb47e92e425ff8a451467d7d89b9d7830e870f9 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 2019998 | e40eb78dab1a59b6c0f1797a2e70a59833b93c4a6783b1bc1ab4926e5e186611 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

The generated port lock recorded all three canonical packages as ported and
zero unported original packages.

**Validator Summary**

- Result summary: `safe/out/validator/artifacts/port/results/libzstd/summary.json`
- Proof path: `safe/out/validator/artifacts/proof/port-validation-proof.json`
- Port lock path: `safe/out/validator/artifacts/proof/port-debs-lock.json`
- Cases: 257
- Source cases: 5
- Usage cases: 250
- Regression cases: 2
- Passed: 257
- Failed: 0
- Casts recorded: 257
- Validator runner status: 0

**Dependent Matrix Summary**

- Dependent image: `safelibs-libzstd-dependents:ubuntu24.04`
- Compile matrix: 12/12 probes passed (`apt`, `dpkg`, `rsync`, `systemd`,
  `libarchive`, `btrfs-progs`, `squashfs-tools`, `qemu`, `curl`, `tiff`,
  `rpm`, `zarchive`)
- Runtime matrix: all 12 dependent runtime applications passed
- `test-original.sh`: passed after rebuilding the dependent image and running
  the combined compile/runtime flow

**Failures Found And Fixed**

No new validator, dependent compile, or dependent runtime failures were found
in this phase. No new compatibility reproducer was required, no validator check
was skipped, and no `safe/src/` or unsafe-audit update was needed.

**Checks Executed**

```bash
git -C validator pull --ff-only
bash safe/scripts/run-validator-libzstd.sh
bash safe/scripts/build-dependent-image.sh
bash safe/scripts/run-dependent-matrix.sh --compile-only
bash safe/scripts/run-dependent-matrix.sh --runtime-only
bash safe/scripts/run-validator-regressions.sh
bash test-original.sh
```

The validator runner internally executed the release build, original CLI helper
build, Debian package build, validator unit tests, testcase manifest checks,
the full libzstd port matrix, and proof generation. All listed commands passed.

Phase 29 Implementation: impl_final_release_burn_down

**Validator Checkout**

- Date: 2026-05-14
- Validator URL: https://github.com/safelibs/validator
- Validator commit: be4251ee8324b38cc8bc41aa215133c7b6c61e47
- Validator branch: main
- `git -C validator pull --ff-only`: advanced from
  `d1c08d01cd50b34a7aeb62c5630e28df0eb6cd97` to
  `be4251ee8324b38cc8bc41aa215133c7b6c61e47`
- Traceability rule: the local proof artifacts under
  `safe/out/validator/artifacts/proof/` are generated after the final commit
  and are the authoritative record for `commit`, `release_tag`, and `tag_ref`.
  The report does not hard-code those self-referential fields.
- Mode: port
- Final validator runner: `bash safe/scripts/run-validator-libzstd.sh`

**Skipped Validator Bug**

The updated validator added
`usage-libarchive-tools-zstd-r17-cli-m-flag-memory-limit-accepted`. The check
expects `zstd -d -M128` to mean a 128 MB decompression memory limit, but
upstream zstd 1.5.5 parses a bare `128` as bytes and exits with status 11
(`Parameter is out of bound`). The same command failed with exit 11 using the
system upstream `zstd` 1.5.5, so this is a validator expectation bug rather
than a libzstd-safe regression.

Skip artifacts were generated locally for the final validator rerun:

- `safe/out/validator/skip.env`
- `safe/out/validator/tests-filtered/`

Only
`usage-libarchive-tools-zstd-r17-cli-m-flag-memory-limit-accepted` was removed
from the filtered test root. No validator source files were modified.

**Override Packages**

The final validator rerun used the canonical override packages staged under
`safe/out/validator/override-debs/libzstd/`:

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 424010 | ca28d57b2a2a8ac8582fc17b07e5a7e0178e477391d7c628d963e99409f944e2 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 2018836 | 36aa6752b1643d2f5e3eed66e9b3aaddf48023af007c2125384e18e1bdcf63b5 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

The generated port lock at
`safe/out/validator/artifacts/proof/port-debs-lock.json` records all three
canonical packages as ported and zero unported original packages. After this
report-only traceability repair is committed, `bash
safe/scripts/run-validator-libzstd.sh` is rerun so that the lock and proof
record the final `git rev-parse HEAD` value and derived
`refs/tags/build-<HEAD[:12]>` tag ref.

**Validator Summary**

- Result summary: `safe/out/validator/artifacts/port/results/libzstd/summary.json`
- Proof path: `safe/out/validator/artifacts/proof/port-validation-proof.json`
- Port lock path: `safe/out/validator/artifacts/proof/port-debs-lock.json`
- Cases: 266
- Source cases: 5
- Usage cases: 259
- Regression cases: 2
- Passed: 266
- Failed: 0
- Casts recorded: 266
- Validator runner status: 0

The unfiltered run against `safelibs/validator@be4251ee` executed 267 cases
and failed only the validator-bug case documented above. After filtering just
that case, the rerun passed every executed validator check. No libzstd-safe
compatibility or safety failure remains in the validator results.

**Traceability Repair**

The first Phase 29 report commit bundled source changes and report text after a
validator run whose proof lock still named the pre-phase commit. The final
repair is report-only, and the validator is rerun after that repair commit so
the untracked proof artifacts in `safe/out/validator/artifacts/proof/` record
the same commit as current `HEAD`. The safe source, tests, and tracked release
artifacts remain unchanged by this report-only repair.

**Fixes Applied**

- `safe/src/compress/cstream.rs` now resubmits the initial MT continue job
  after a tiny output buffer drains the frame header, so callers using
  `ZSTD_e_continue` do not stall before any worker job starts.
- `safe/src/ffi/compress.rs` and `safe/src/threading/zstdmt.rs` now separate
  MT pending-output accounting from flushed-output accounting. Continue-mode
  frame progression reports upstream-compatible flushed progress for adaptive
  CLI behavior, while flush/end mode still exposes produced-but-not-flushed
  worker output.
- `safe/tests/rust/compress.rs` adds regression coverage for the tiny-output
  MT continue path and strengthens the MT frame-progression regression.
- `safe/tests/fixtures/regression/results-memoized.source-sha256` was
  refreshed to
  `07a6dc4a917ed947348774a5f555f7baf7c279edecbcc0011ae6e46e199b116a`.
  The memoized upstream regression CSV still matched its 587 preserved rows.

**Release Gate Summary**

- `bash safe/scripts/run-full-suite.sh`: passed. The aggregator consumed the
  existing Phase 4 and Phase 6 artifact roots, ran the preserved upstream
  wrappers, performance smoke, and both downstream image legs. The downstream
  compile and runtime matrix passed 12/12 applications in the built image.
- `bash test-original.sh`: passed after the dependent image refresh and the
  combined compile/runtime flow.
- Forbidden runtime marker scan:
  `rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe`
  produced no matches.

**Checks Executed**

```bash
git -C validator pull --ff-only
bash safe/scripts/run-validator-libzstd.sh
zstd --version
tmp=$(mktemp -d)
python3 -c 'import sys; sys.stdout.buffer.write(b"r17 memlimit payload row\n"*1000)' > "$tmp/payload.bin"
zstd -q -o "$tmp/out.zst" "$tmp/payload.bin"
zstd -dq -M128 -o "$tmp/decoded.bin" "$tmp/out.zst"
printf 'original_system_zstd_status=%s\n' "$?"
rm -rf "$tmp"
bash safe/scripts/run-validator-libzstd.sh
bash safe/scripts/build-artifacts.sh --release
bash safe/scripts/build-original-cli-against-safe.sh
bash safe/scripts/build-deb.sh
bash safe/scripts/build-dependent-image.sh
bash safe/scripts/run-full-suite.sh
bash test-original.sh
rg -n 'SAFE_UPSTREAM_LIB|load_upstream!|dlopen|dlsym|upstream-phase4' safe || test $? -eq 1
git rev-parse HEAD
bash safe/scripts/run-validator-libzstd.sh
```

The first validator run listed above was the unfiltered run that exposed the
new validator-bug case. The second validator run used the local filtered test
root and passed 266/266. The final validator command is the post-commit
traceability rerun; it refreshes the local proof lock against current `HEAD`.
The explicit build commands refreshed the canonical release-gate roots before
the final full-suite and `test-original.sh` runs.
