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
| usage-libarchive-tools-zstd-extract-specific-member | usage | libarchive-tools | 1 | testcase command exited with status 1 | port-04-test/results/libzstd/usage-libarchive-tools-zstd-extract-specific-member.json | port-04-test/logs/libzstd/usage-libarchive-tools-zstd-extract-specific-member.log | impl_validator_libarchive_usage_regressions | fixed | safe/docker/dependents/entrypoint.sh:test_libarchive; safe/tests/rust/compress.rs::compress_streaming_libarchive_tar_chunks_roundtrip_for_listing_usage | 5b0b77fac760ec0c7896e748f2e22af292081d76 | Fixed by validating structured stream payloads under the exact emitted zstd frame header and falling back to stored blocks when that frame would not round-trip. Final validator result passed. |
| usage-libarchive-tools-zstd-two-topdirs-list | usage | libarchive-tools | 1 | testcase command exited with status 1 | port-04-test/results/libzstd/usage-libarchive-tools-zstd-two-topdirs-list.json | port-04-test/logs/libzstd/usage-libarchive-tools-zstd-two-topdirs-list.log | impl_validator_libarchive_usage_regressions | fixed | safe/docker/dependents/entrypoint.sh:test_libarchive; safe/tests/rust/compress.rs::compress_streaming_libarchive_tar_chunks_roundtrip_for_listing_usage | 5b0b77fac760ec0c7896e748f2e22af292081d76 | Fixed by validating structured stream payloads under the exact emitted zstd frame header and falling back to stored blocks when that frame would not round-trip. Final validator result passed. |

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
- Bounced Phase 3 software verifier `streaming-c-api-smoke` artifact: status `passed`, exit code 0, port commit `0ac23718a626c7f0d060f7713721f5b277c06c9b`, result path `safe/out/validator/artifacts/port-04-test/results/libzstd/streaming-c-api-smoke.json`, log path `safe/out/validator/artifacts/port-04-test/logs/libzstd/streaming-c-api-smoke.log`
- Bounced Phase 3 software verifier artifact summary: 85 cases, 5 source cases, 80 usage cases, 83 passed, 2 failed, 85 casts, validator runner status 1
- Phase 3 package-backed reruns observed `usage-libarchive-tools-zstd-extract-specific-member` both passing and failing; because it recurred as a non-streaming libarchive usage failure, it remains recorded as an open later-phase row.
- Remaining open validator rows: `usage-libarchive-tools-zstd-extract-specific-member` and `usage-libarchive-tools-zstd-two-topdirs-list`, both assigned to `impl_validator_libarchive_usage_regressions`
- Streaming C API senior-bounce fix commit: `9da27a66e67f999b649ad6d220ed00c76847d656`
- Regression test: `safe/tests/capi/zstream_driver.c` (`decompress_stream_rejects_staged_null_destination`)
- Net safe code changes in this phase: `safe/src/ffi/decompress.rs` rejects null staged-output destinations before copying, and `safe/tests/capi/zstream_driver.c` covers the staged `ZSTD_decompressStream` null-destination error path.

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
cargo fmt --manifest-path safe/Cargo.toml --check
bash safe/scripts/run-capi-roundtrip.sh
cargo test --manifest-path safe/Cargo.toml --release --all-targets
bash safe/scripts/run-capi-decompression.sh
set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"; exit 0
VALIDATOR_RUNNER_STATUS=1 python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --allow-remaining-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down
python3 - <<'PY'
import json, pathlib
root=pathlib.Path('safe/out/validator/artifacts/port-04-test/results/libzstd')
print(json.dumps(json.loads((root/'summary.json').read_text()), indent=2))
for name in ['streaming-c-api-smoke.json',
             'usage-libarchive-tools-zstd-extract-specific-member.json',
             'usage-libarchive-tools-zstd-two-topdirs-list.json']:
    data=json.loads((root/name).read_text())
    print(name, data.get('status'), data.get('exit_code'), data.get('port_commit'))
PY
sed -n '1,220p' safe/out/validator/artifacts/port-04-test/logs/libzstd/usage-libarchive-tools-zstd-two-topdirs-list.log
sed -n '1,220p' safe/out/validator/artifacts/port-04-test/logs/libzstd/streaming-c-api-smoke.log
bash -lc 'base=$(awk "/Phase 3 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'
bash safe/scripts/verify-export-parity.sh
bash safe/scripts/verify-link-compat.sh
rg -n 'streaming-c-api-smoke|No streaming C API failures assigned|ZSTD_compressStream2|ZSTD_decompressStream|usage-libarchive-tools-zstd-extract-specific-member|usage-libarchive-tools-zstd-two-topdirs-list' validator-report.md safe/tests safe/src
test ! -f safe/out/validator/skip.env
```

**Phase 3 Result**

No validator streaming C API failure row was assigned to `impl_validator_streaming_capi_regressions`, and the validator `streaming-c-api-smoke` result is passing. The senior-tester bounce identified a streaming decompression pointer-validation gap outside the validator failure table; this phase fixed it in `9da27a66e67f999b649ad6d220ed00c76847d656` and added the C regression listed above. The remaining validator failures are non-streaming libarchive usage testcases assigned to the later libarchive usage phase.

**Phase 4: Libarchive Usage Validator Failures**

Phase 4 Base Commit: 8ff3b4d98827295ed8b4fc07448c33bede1b8e50
- Implement phase: `impl_validator_libarchive_usage_regressions`
- Validator Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Assigned rows fixed: `usage-libarchive-tools-zstd-extract-specific-member`, `usage-libarchive-tools-zstd-two-topdirs-list`
- Same-symptom rows observed during Phase 4 triage and fixed by the same change: `usage-libarchive-tools-zstd-member-count-three-plus`, `usage-libarchive-tools-zstd-stdin-list-members`
- Regression coverage: `safe/tests/rust/compress.rs::compress_streaming_libarchive_tar_chunks_roundtrip_for_listing_usage` mirrors libarchive's 512-byte tar chunking; `safe/docker/dependents/entrypoint.sh:test_libarchive` now covers two top-level directories and specific-member extraction through installed `bsdtar --zstd`
- Fix commits: `f9509b72eb850b7bede1658988dcfa546caaff0f`, `ba2285d0bc3ae7d986f2eb8b7b622fd0adf8b69b`, `5b0b77fac760ec0c7896e748f2e22af292081d76`

**Phase 4 Finding**

Libarchive creates zstd-compressed tar archives by calling `ZSTD_compressStream` repeatedly with tar-sized chunks and then emitting the buffered payload in `ZSTD_endStream`. The safe compressor reused blocks from `structured-zstd` after stripping that encoder's frame header, but the validation checked the source encoder's frame instead of the final libzstd-safe streaming frame. With the emitted unknown-size streaming header, the block could decode to repeated bytes, causing `bsdtar` to report `Unrecognized archive format`.

The final fix validates the payload under the exact frame header libzstd-safe will emit. If that exact frame does not decode back to the input, the compressor uses stored blocks for that no-history payload. This preserves package-installed `bsdtar --zstd` behavior while keeping existing compressed-stream expectations passing for other cases.

**Phase 4 Commands Run**

```bash
sed -n '1,240p' .plan/workflow-structure.yaml
git rev-parse HEAD
git -C validator rev-parse HEAD
git -C validator status --short --branch
cargo fmt --manifest-path safe/Cargo.toml --check
cargo test --manifest-path safe/Cargo.toml --release --test compress compress_streaming_libarchive_tar_chunks_roundtrip_for_listing_usage
cargo test --manifest-path safe/Cargo.toml --release --test compress
bash safe/scripts/build-deb.sh
docker run --rm -t --mount type=bind,src=/home/yans/safelibs/pipeline/ports/port-libzstd/safe/out/deb/default/packages,dst=/override-debs,readonly validator-check-libzstd bash -lc 'set -euo pipefail; /validator/tests/_shared/install_override_debs.sh >/tmp/install.log; for i in 1 2 3 4 5; do tmp=$(mktemp -d); mkdir -p "$tmp/in/top1" "$tmp/in/top2" "$tmp/out"; printf "alpha\n" > "$tmp/in/top1/alpha.txt"; printf "beta\n" > "$tmp/in/top2/beta.txt"; bsdtar --zstd -cf "$tmp/a.tar.zstd" -C "$tmp/in" top1 top2; bsdtar -tf "$tmp/a.tar.zstd" | sort >"$tmp/list"; bsdtar -xf "$tmp/a.tar.zstd" -C "$tmp/out"; cmp "$tmp/in/top1/alpha.txt" "$tmp/out/top1/alpha.txt"; cmp "$tmp/in/top2/beta.txt" "$tmp/out/top2/beta.txt"; done'
cargo test --manifest-path safe/Cargo.toml --release --all-targets
bash safe/scripts/verify-export-parity.sh
bash -lc 'if [ -d safe/tests/validator ]; then test -x safe/scripts/run-validator-regressions.sh; bash safe/scripts/run-validator-regressions.sh; else echo no-validator-regression-dir; fi'
test ! -f safe/out/validator/skip.env
set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"; exit 0
VALIDATOR_RUNNER_STATUS=0 python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --allow-remaining-phase impl_validator_remaining_burn_down
bash safe/scripts/build-dependent-image.sh
bash safe/scripts/run-dependent-matrix.sh --runtime-only --apps libarchive
bash test-original.sh --runtime-only --apps libarchive
bash -lc 'base=$(awk "/Phase 4 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'
bash safe/scripts/verify-install-layout.sh
bash safe/scripts/verify-install-layout.sh --debian
bash safe/scripts/verify-link-compat.sh
git -C validator status --short --branch
rg -n 'usage-libarchive-tools|No libarchive usage failures assigned to impl_validator_libarchive_usage_regressions' validator-report.md
```

**Phase 4 Result**

Final validator artifacts at `safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json`: 85 cases, 5 source cases, 80 usage cases, 85 passed, 0 failed, 85 casts, validator runner status 0. The assigned libarchive usage rows now pass with port commit `5b0b77fac760ec0c7896e748f2e22af292081d76`. No validator checks were skipped and `safe/out/validator/skip.env` is absent. Dependent libarchive runtime coverage, original runtime comparison, install layout, Debian install layout, link compatibility, and validator worktree status checks all passed.

Current package outputs used by the final local package smoke:

| package | filename | architecture | size | sha256 |
| --- | --- | --- | --- | --- |
| libzstd1 | libzstd1_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 380926 | 9c05c6f3a144354da30827b2a020d1341f4f6f57d3e9e6c6d1aef22988b6b27c |
| libzstd-dev | libzstd-dev_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 3830588 | 1525e2933b9d26206f51a8e51af45935bcb11629cfb93203f22048ed39f5f6e6 |
| zstd | zstd_1.5.5+dfsg2-2build1.1+safelibs1_amd64.deb | amd64 | 159324 | 8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91 |

**Phase 5: Remaining Failures, Validator-Bug Triage, and Report Consolidation**

Phase 5 Base Commit: eaed055405b7cdf7d38a2b1ee76255f3dc7a5d91
- Implement phase: `impl_validator_remaining_burn_down`
- Validator Commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- No remaining failures assigned to impl_validator_remaining_burn_down
- Residual failure rows owned by this phase: 0
- Validator-bug skips generated: none
- Safe code, package metadata, and regression-test changes in this phase: none

**Phase 5 Commands Run**

```bash
git status --short
sed -n '1,240p' .plan/workflow-structure.yaml
sed -n '1,260p' validator-report.md
rg -n 'impl_validator_remaining_burn_down|Phase 5|open|Unclassified|Remaining|Validator Bug|skipped_validator_bug|fixed' validator-report.md safe/out/validator/artifacts -g '*.json' -g '*.md' -g '*.txt' -g '*.log'
sed -n '260,520p' validator-report.md
python3 -m json.tool safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json
find safe/out/validator/artifacts/port-04-test/results/libzstd -maxdepth 1 -name '*.json' -print | sort | wc -l
rg -n '"status": "failed"|"status": "error"|"failed"' safe/out/validator/artifacts/port-04-test/results/libzstd/*.json
git -C validator rev-parse HEAD
git -C validator status --short --branch
test -f safe/out/validator/skip.env; echo skip_env_status=$?
set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"; exit 0
VALIDATOR_RUNNER_STATUS=0 python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --completed-phase impl_validator_remaining_burn_down
python3 -m json.tool safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json
python3 -m json.tool safe/out/validator/artifacts/proof/port-04-test-debs-lock.json
ls -l safe/out/validator/artifacts/proof
bash safe/scripts/run-validator-regressions.sh
bash -lc 'set +e; bash safe/scripts/run-validator-libzstd.sh; status=$?; set -e; VALIDATOR_RUNNER_STATUS=$status python3 safe/scripts/check-validator-phase-results.py --results-root safe/out/validator/artifacts/port-04-test/results/libzstd --report validator-report.md --completed-phase impl_validator_source_cli_regressions --completed-phase impl_validator_streaming_capi_regressions --completed-phase impl_validator_libarchive_usage_regressions --completed-phase impl_validator_remaining_burn_down; test "$status" -eq 0'
python3 -m json.tool safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json >/dev/null
rg -n 'Unclassified|Remaining|Validator Bug|Skip|No remaining failures assigned to impl_validator_remaining_burn_down' validator-report.md
cargo test --manifest-path safe/Cargo.toml --release --all-targets
bash safe/scripts/verify-export-parity.sh
bash -lc 'base=$(awk "/Phase 5 Base Commit:/ {print \$5; exit}" validator-report.md); test -n "$base"; git diff --check "$base..HEAD"'
test -z "$(git -C validator status --porcelain --untracked-files=no)"
bash -lc 'if [ -f safe/out/validator/skip.env ]; then py=python3; if [ -x safe/out/validator/venv/bin/python ]; then py="$PWD/safe/out/validator/venv/bin/python"; else python3 -c "import yaml"; fi; set -a; . safe/out/validator/skip.env; set +a; test -n "${VALIDATOR_TESTS_ROOT:-}"; test -d "$VALIDATOR_TESTS_ROOT/libzstd"; test -d "$VALIDATOR_TESTS_ROOT/tests/libzstd"; "$py" validator/tools/testcases.py --config validator/repositories.yml --tests-root "$VALIDATOR_TESTS_ROOT" --library libzstd --check --min-source-cases "$VALIDATOR_MIN_SOURCE_CASES" --min-usage-cases "$VALIDATOR_MIN_USAGE_CASES" --min-cases "$VALIDATOR_MIN_CASES"; fi'
find safe/out/validator -maxdepth 3 -type d | sort | sed -n '1,120p'
rg -n 'skip|skipped|validator bug|justification|No remaining failures assigned to impl_validator_remaining_burn_down' validator-report.md
```

**Phase 5 Result**

Strict full validator run passed with `VALIDATOR_RUNNER_STATUS=0`. Final validator artifacts at `safe/out/validator/artifacts/port-04-test/results/libzstd/summary.json`: 85 cases, 5 source cases, 80 usage cases, 85 passed, 0 failed, 85 casts. The full result set contains 85 testcase result JSON files plus `summary.json`.

Proof artifacts were generated at `safe/out/validator/artifacts/proof/port-04-test-debs-lock.json` and `safe/out/validator/artifacts/proof/port-04-test-validation-proof.json`. The generated port lock records local safe commit `eaed055405b7cdf7d38a2b1ee76255f3dc7a5d91` with release tag `build-eaed055405b7`.

No validator bug was identified. `safe/out/validator/skip.env` is absent, no filtered test root was generated, and `validator/` remains unmodified.

Phase 5 verification passed: `safe/scripts/run-validator-regressions.sh` found no local validator regression directory, the strict validator plus phase-result check passed, `summary.json` parsed as valid JSON, `cargo test --manifest-path safe/Cargo.toml --release --all-targets` passed, `safe/scripts/verify-export-parity.sh` verified 185 exported symbols, `git diff --check` found no whitespace errors, and the validator checkout had no tracked or modified files.
