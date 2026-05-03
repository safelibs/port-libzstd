Phase 1 Base Commit: 40c81c097986912f51acf790980168dacf7ff72b

**Validator Checkout**

- Validator URL: https://github.com/safelibs/validator
- Validator Commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
- Validator branch: main (detached HEAD at origin/main)
- Planning reference commit: 1319bb0374ef66428a42dd71e49553c6d057feaf
- Local safe commit: 40c81c097986912f51acf790980168dacf7ff72b

The pinned planning commit `1319bb03` predates `safelibs/validator@c58e3e28`, the
upstream rename of the matrix mode from `port-04-test` to `port`. The local CI
script at `safe/scripts/run-validator-libzstd.sh` invokes `--mode port`, so the
validator checkout was refreshed to `origin/main` (`87b321fe`) before the
matrix run; older commits reject the new mode label.

**Python Setup**

- Python setup path: `python3` (`/home/yans/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3`).
- PyYAML source: host Python already provided `yaml`; `safe/out/validator/venv/` was not created.

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
- Tag ref: refs/tags/build-40c81c097986
- Commit: 40c81c097986912f51acf790980168dacf7ff72b
- Release tag: build-40c81c097986
- Package architectures: amd64
- Package sizes: libzstd1=380926, libzstd-dev=3830588, zstd=159324
- Package SHA256 hashes: libzstd1=9c05c6f3a144354da30827b2a020d1341f4f6f57d3e9e6c6d1aef22988b6b27c, libzstd-dev=1525e2933b9d26206f51a8e51af45935bcb11629cfb93203f22048ed39f5f6e6, zstd=8d19c5e52f1c186e34a425c112c6b6a98be85390dc233456bc3f40da9d919f91

The `tag_ref` field stored in the lock matches `refs/tags/<release_tag>`, the
shape required by `safelibs/validator@b5b1b5df` when validating port-mode
deb-locks. The runner script's inline lock generator emits the historical
`refs/tags/libzstd/04-test-local` value; the lock was rewritten in place to
the validator-required form before `validator/test.sh` was invoked.

**Exact Commands Run**

```bash
git rev-parse HEAD
ls safe/out/validator/artifacts/proof/
rm -f safe/out/validator/artifacts/proof/port-04-test-debs-lock.json safe/out/validator/artifacts/proof/port-04-test-validation-proof.json
python3 -c 'import yaml'
git -C validator fetch origin
git -C validator checkout origin/main
git -C validator rev-parse HEAD
set +e
bash safe/scripts/run-validator-libzstd.sh
status=$?
set -e
printf 'VALIDATOR_RUNNER_STATUS=%s\n' "$status"
python3 - <<'PY'
import json, pathlib
p = pathlib.Path("safe/out/validator/artifacts/proof/port-debs-lock.json")
data = json.loads(p.read_text())
lib = data["libraries"][0]
lib["tag_ref"] = f"refs/tags/{lib['release_tag']}"
p.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PY
rm -rf safe/out/validator/artifacts/port
set +e
PYTHON=python3 bash validator/test.sh \
    --config validator/repositories.yml \
    --tests-root validator/tests \
    --artifact-root safe/out/validator/artifacts \
    --mode port \
    --library libzstd \
    --override-deb-root safe/out/validator/override-debs \
    --port-deb-lock safe/out/validator/artifacts/proof/port-debs-lock.json \
    --record-casts
test_sh_status=$?
set -e
test ! -f safe/out/validator/skip.env
VALIDATOR_RUNNER_STATUS=1 python3 safe/scripts/check-validator-phase-results.py \
    --results-root safe/out/validator/artifacts/port/results/libzstd \
    --report validator-report.md \
    --allow-remaining-phase impl_validator_source_cli_regressions \
    --allow-remaining-phase impl_validator_streaming_capi_regressions \
    --allow-remaining-phase impl_validator_libarchive_usage_regressions \
    --allow-remaining-phase impl_validator_remaining_burn_down
```

The canonical helper executed these validator build/setup steps internally:

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

The runner-internal `validator/test.sh` invocation aborted before producing
`summary.json` because the inline-generated lock used the historical
`tag_ref` value. The build/setup steps it had already executed (artifact
build, CLI build, deb build, override-deb staging, lock generation, validator
unit and `check-testcases` runs) all succeeded. The lock was then rewritten in
place and `validator/test.sh` re-invoked manually with the same arguments;
that run produced the matrix evidence captured below.

Proof generation was not run because the matrix has 1 failed testcase
(`safe/scripts/run-validator-libzstd.sh` only invokes
`tools/verify_proof_artifacts.py` on a clean run with zero failures).

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
