Phase 1 Base Commit: c9970b608feeb7d1e1cfc94e40c7ee8aa1ed7fbb

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Validator branch: main
- Planning reference commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Local safe commit: 93bd2c1baafa6691637b6a9edf62035453e3fd6d

**Python Setup**

- Python setup path: python3 (/home/yans/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3)
- PyYAML source: host Python already provided `yaml`; `safe/out/validator/venv/` was not created.

**Override Packages**

The validator override leaf is `safe/out/validator/override-debs/libzstd/`.

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 379926 | c0f6bdc23d5338e12a832443c33ec5d7322f98089e40d12b6bf5683390cacad3 |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 3831578 | 7c6abfe048c50409f0bbc9ac4dcfb4c9db3f516db246e04e1ef0b767adc31c6c |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

**Generated Port Lock**

- Path: `safe/out/validator/artifacts/proof/port-04-test-debs-lock.json`
- Repository: local/port-libzstd
- Tag ref: refs/tags/libzstd/04-test-local
- Commit: 93bd2c1baafa6691637b6a9edf62035453e3fd6d
- Release tag: build-93bd2c1baafa
- Package architectures: amd64
- Package sizes: libzstd1=379926, libzstd-dev=3831578, zstd=159324
- Package SHA256 hashes: libzstd1=c0f6bdc23d5338e12a832443c33ec5d7322f98089e40d12b6bf5683390cacad3, libzstd-dev=7c6abfe048c50409f0bbc9ac4dcfb4c9db3f516db246e04e1ef0b767adc31c6c, zstd=8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91

**Exact Commands Run**

```bash
git clone https://github.com/safelibs/validator validator
chmod +x safe/scripts/run-validator-libzstd.sh safe/scripts/check-validator-phase-results.py safe/scripts/run-validator-regressions.sh
bash -n safe/scripts/run-validator-libzstd.sh
bash -n safe/scripts/run-validator-regressions.sh
python3 -m py_compile safe/scripts/check-validator-phase-results.py
rm -rf safe/scripts/__pycache__
rm -f safe/out/validator/skip.env
rm -rf safe/out/validator/tests-filtered safe/out/validator/artifacts
set +e
DEB_BUILD_PROFILES=noudeb bash safe/scripts/run-validator-libzstd.sh
status=$?
printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"
set -e
VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --allow-remaining-phase impl_validator_source_cli_regressions --allow-remaining-phase impl_validator_streaming_capi_regressions --allow-remaining-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down
cargo fmt --manifest-path safe/Cargo.toml --check
exit 0
```

The canonical helper executed these validator steps:

```bash
bash safe/scripts/build-artifacts.sh --release
bash safe/scripts/build-original-cli-against-safe.sh
env -u DEB_BUILD_PROFILES bash safe/scripts/build-deb.sh
PYTHON=python3 make -C validator unit
PYTHON=python3 make -C validator check-testcases
PYTHON=python3 bash validator/test.sh --config validator/repositories.yml --tests-root validator/tests --artifact-root safe/out/validator/artifacts --mode port-04-test --library libzstd --override-deb-root safe/out/validator/override-debs --port-deb-lock safe/out/validator/artifacts/proof/port-04-test-debs-lock.json --record-casts
```

Proof generation was not run because the matrix had failed testcases.

**Matrix Inventory**

- Source cases: 5
- Usage cases: 80
- Total cases: 85

**Initial Run**

- Summary path: `safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json`
- Cases: 85
- Source cases: 5
- Usage cases: 80
- Passed: 84
- Failed: 1
- Casts: 85
- Validator runner status: 1

**Failure Classification**

| testcase_id | kind | client_application | exit_code | error | result_path | log_path | assigned_remediation_phase | remediation_status | regression_test | fix_commit | notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| usage-libarchive-tools-zstd-extract-specific-member | usage | libarchive-tools | 1 | testcase command exited with status 1 | port-04-test/results/libzstd/usage-libarchive-tools-zstd-extract-specific-member.json | port-04-test/logs/libzstd/usage-libarchive-tools-zstd-extract-specific-member.log | impl_validator_libarchive_usage_regressions | open |  |  | bsdtar reported "Unrecognized archive format" while extracting a named member from a zstd-compressed tar archive. Observed in the Phase 3 verifier rerun and assigned to the later libarchive usage phase. |
| usage-libarchive-tools-zstd-two-topdirs-list | usage | libarchive-tools | 1 | testcase command exited with status 1 | port-04-test/results/libzstd/usage-libarchive-tools-zstd-two-topdirs-list.json | port-04-test/logs/libzstd/usage-libarchive-tools-zstd-two-topdirs-list.log | impl_validator_libarchive_usage_regressions | open |  |  | bsdtar reported "Unrecognized archive format" while listing a zstd-compressed tar archive with two top-level directories. |

**Skip List**

- Empty. No validator checks were skipped in Phase 1.

**Phase 2: Source CLI, Dictionary, Multiframe, and Corruption Failures**

Phase 2 Base Commit: fe85807776ce1b743ad476b3301aa58bbdf4542f
- Implement phase: `impl_validator_source_cli_regressions`
- Validator Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Source cases inspected: 5
- Source cases assigned to this phase in the original Phase 1 table: 0
- No source-case failures assigned to impl_validator_source_cli_regressions
- Net safe code changes in this phase: none

**Phase 2 Commands Run**

```bash
sed -n '1,240p' .plan/workflow-structure.yaml
sed -n '1,260p' validator-report.md
git rev-parse HEAD
git -C validator rev-parse HEAD
git -C validator status --short --branch
python3 - <<'PY'
import json, pathlib
p=pathlib.Path('safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json')
print(p.exists())
if p.exists():
    data=json.loads(p.read_text())
    print(json.dumps(data, indent=2)[:2000])
PY
rg -n 'impl_validator_source_cli_regressions|Phase 2|zstd-compress-decompress|dictionary-train-use|multi-frame-behavior|corrupted-frame-rejection' validator-report.md
cargo test --manifest-path safe/Cargo.toml --release --all-targets
bash safe/scripts/verify-export-parity.sh
cargo test --manifest-path safe/Cargo.toml --release --test compress
cargo test --manifest-path safe/Cargo.toml --release --test decompress
bash safe/scripts/run-capi-roundtrip.sh
bash safe/scripts/run-capi-decompression.sh
bash -lc 'if [ -d safe/tests/validator ]; then test -x safe/scripts/run-validator-regressions.sh; bash safe/scripts/run-validator-regressions.sh; else echo no-validator-regression-dir; fi'
test ! -f safe/out/validator/skip.env
set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"; exit 0
VALIDATOR_RUNNER_STATUS=1 python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --allow-remaining-phase impl_validator_streaming_capi_regressions --allow-remaining-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down
```

**Phase 2 Result**

No safe implementation, package, or regression-test changes remain in the net Phase 2 diff because no source-case failures were assigned to `impl_validator_source_cli_regressions`. At the Phase 2 handoff, the known libarchive usage failure remained recorded as an open row assigned to `impl_validator_libarchive_usage_regressions`, the later phase reserved for libarchive usage remediation.

Phase 2 post-correction validator artifacts at `safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json`: 85 cases, 5 source cases, 80 usage cases, 84 passed, 1 failed, 85 casts, validator runner status 1. The failed testcase at that point was `usage-libarchive-tools-zstd-two-topdirs-list`; `check-validator-phase-results.py` passed and reported it as an allowed remaining failed testcase for `impl_validator_libarchive_usage_regressions`.

**Phase 3: Streaming C API Validator Failures**

Phase 3 Base Commit: ff7819723e25d9b669aebf22032e4daab5db7a38
- Implement phase: `impl_validator_streaming_capi_regressions`
- Validator Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Streaming C API rows assigned to this phase in the original Phase 1 table: 0
- No streaming C API failures assigned to impl_validator_streaming_capi_regressions
- Existing `streaming-c-api-smoke` validator artifact: status `passed`, exit code 0, result path `safe/out/validator/artifacts/port-04-test/results/libzstd/streaming-c-api-smoke.json`, log path `safe/out/validator/artifacts/port-04-test/logs/libzstd/streaming-c-api-smoke.log`
- Current validator artifact summary after the Phase 3 verifier rerun: 85 cases, 5 source cases, 80 usage cases, 83 passed, 2 failed, 85 casts, validator runner status 1
- Remaining open validator rows: `usage-libarchive-tools-zstd-extract-specific-member` and `usage-libarchive-tools-zstd-two-topdirs-list`, both assigned to `impl_validator_libarchive_usage_regressions`
- Net safe code changes in this phase: none

**Phase 3 Commands Inspected**

```bash
git status --short && git rev-parse HEAD
sed -n '1,240p' .plan/workflow-structure.yaml
sed -n '1,260p' validator-report.md
rg -n 'streaming-c-api-smoke|impl_validator_streaming_capi_regressions|Phase 3|Phase 1|open|fixed|assigned' validator-report.md safe/out/validator/artifacts/port-04-test/results/libzstd safe/out/validator/artifacts/port-04-test/logs/libzstd
git -C validator rev-parse HEAD && git -C validator status --short --branch
python3 - <<'PY'
import json, pathlib
for name in ['summary.json', 'streaming-c-api-smoke.json']:
    p=pathlib.Path('safe/out/validator/artifacts/port-04-test/results/libzstd')/name
    print('---', p)
    print('exists', p.exists())
    if p.exists():
        data=json.loads(p.read_text())
        print(json.dumps(data, indent=2)[:4000])
PY
sed -n '1,220p' safe/out/validator/artifacts/port-04-test/logs/libzstd/streaming-c-api-smoke.log
rg -n 'streaming-c-api-smoke|impl_validator_streaming_capi_regressions|Phase 3|usage-libarchive-tools-zstd-two-topdirs-list' safe/out/validator/artifacts/port-04-test/results/libzstd/*.json validator-report.md
sed -n '1,260p' safe/scripts/check-validator-phase-results.py
git log --oneline --decorate -5
git status --short
sed -n '1,240p' safe/out/validator/artifacts/port-04-test/logs/libzstd/usage-libarchive-tools-zstd-extract-specific-member.log
python3 - <<'PY'
import json, pathlib
root=pathlib.Path('safe/out/validator/artifacts/port-04-test/results/libzstd')
print(json.dumps(json.loads((root/'summary.json').read_text()), indent=2))
for p in sorted(root.glob('*.json')):
    if p.name != 'summary.json':
        data=json.loads(p.read_text())
        if data.get('status') == 'failed':
            print(data['testcase_id'], data.get('log_path'))
PY
```

**Phase 3 Result**

No safe implementation, package, or regression-test changes were made because no Phase 1 row was assigned to `impl_validator_streaming_capi_regressions`. The existing `streaming-c-api-smoke` result is passing. The currently remaining failed testcases are libarchive usage failures assigned to the later libarchive usage phase.
