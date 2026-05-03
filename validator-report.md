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
