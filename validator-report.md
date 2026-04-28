Phase 1 Base Commit: c9970b608feeb7d1e1cfc94e40c7ee8aa1ed7fbb

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Validator branch: main
- Planning reference commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Local safe commit: 369d504d9a470b276c43174b12591adb636e682f

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
- Commit: 369d504d9a470b276c43174b12591adb636e682f
- Release tag: build-369d504d9a47
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
PYTHON=python3 validator/tools/verify_proof_artifacts.py --config validator/repositories.yml --tests-root validator/tests --artifact-root safe/out/validator/artifacts --proof-output safe/out/validator/artifacts/proof/port-04-test-validation-proof.json --mode port-04-test --library libzstd --min-source-cases 5 --min-usage-cases 80 --min-cases 85 --require-casts
```

Proof generation completed: `safe/out/validator/artifacts/proof/port-04-test-validation-proof.json`.

**Matrix Inventory**

- Source cases: 5
- Usage cases: 80
- Total cases: 85

**Initial Run**

- Summary path: `safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json`
- Cases: 85
- Source cases: 5
- Usage cases: 80
- Passed: 85
- Failed: 0
- Casts: 85
- Validator runner status: 0

**Failure Classification**

| testcase_id | kind | client_application | exit_code | error | result_path | log_path | assigned_remediation_phase | remediation_status | regression_test | fix_commit | notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |

The current Phase 1 validator evidence run has no failing testcase rows to assign. No libzstd implementation or regression-test changes are included in this phase.

**Skip List**

- Empty. No validator checks were skipped in Phase 1.
