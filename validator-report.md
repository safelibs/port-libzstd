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
| usage-libarchive-tools-zstd-cli-test-integrity-flag | usage | libarchive-tools | 1 | testcase command exited with status 1 | port/results/libzstd/usage-libarchive-tools-zstd-cli-test-integrity-flag.json | port/logs/libzstd/usage-libarchive-tools-zstd-cli-test-integrity-flag.log | impl_validator_libarchive_usage_regressions | open |  |  | libarchive-tools usage case running the installed `zstd` CLI under `bsdtar`/`zstd` packaging; classified to the libarchive usage phase by ID prefix `usage-libarchive-tools-zstd-`. |

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
