Phase 1 Base Commit: c9970b608feeb7d1e1cfc94e40c7ee8aa1ed7fbb

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Validator branch: main
- Planning reference commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Local safe commit: 02ef19834532653585e41f3c34bed786a39682a4

**Python Setup**

- Python setup path: python3 (/home/yans/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3)
- PyYAML source: host Python already provided `yaml`; `safe/out/validator/venv/` was not created.

**Override Packages**

The validator override leaf is `safe/out/validator/override-debs/libzstd/`.

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 380312 | 5a8876acaf8d17a96a0ced4465b84df3c84e846ec4ca27674c12a8afe58fa21c |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 3831596 | c8a309e65a59648d30e2f587aa01712301c2b408fec1f8cb42142a633e9d5eab |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

**Generated Port Lock**

- Path: `safe/out/validator/artifacts/proof/port-04-test-debs-lock.json`
- Repository: local/port-libzstd
- Tag ref: refs/tags/libzstd/04-test-local
- Commit: 02ef19834532653585e41f3c34bed786a39682a4
- Release tag: build-02ef19834532
- Package architectures: amd64
- Package sizes: libzstd1=380312, libzstd-dev=3831596, zstd=159324
- Package SHA256 hashes: libzstd1=5a8876acaf8d17a96a0ced4465b84df3c84e846ec4ca27674c12a8afe58fa21c, libzstd-dev=c8a309e65a59648d30e2f587aa01712301c2b408fec1f8cb42142a633e9d5eab, zstd=8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91

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
bash safe/scripts/run-validator-libzstd.sh
status=$?
printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"
VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --allow-remaining-phase impl_validator_source_cli_regressions --allow-remaining-phase impl_validator_streaming_capi_regressions --allow-remaining-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down
cargo test --manifest-path safe/Cargo.toml small_stream_archive_payload_uses_raw_blocks_for_dependent_readers -- --nocapture
cargo fmt --manifest-path safe/Cargo.toml --check
DEB_BUILD_PROFILES=noudeb bash safe/scripts/run-validator-libzstd.sh
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

All validator testcases passed after fixing small no-history stream compression to emit raw zstd blocks for small archive-sized payloads. Regression coverage: `safe/tests/rust/compress.rs::small_stream_archive_payload_uses_raw_blocks_for_dependent_readers`. Fix commit: `02ef19834532653585e41f3c34bed786a39682a4`.

**Skip List**

- Empty. No validator checks were skipped in Phase 1.
