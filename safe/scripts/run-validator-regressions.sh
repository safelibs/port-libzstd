#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
VALIDATOR_ARTIFACT_ROOT="$REPO_ROOT/safe/out/validator/artifacts"
export REPO_ROOT SAFE_ROOT VALIDATOR_ARTIFACT_ROOT

TEST_ROOT="$SAFE_ROOT/tests/validator"
if [[ ! -d "$TEST_ROOT" ]]; then
    printf 'no validator regression tests found at %s\n' "$TEST_ROOT"
    exit 0
fi

mapfile -t TESTS < <(
    find "$TEST_ROOT" -maxdepth 1 -type f -name '*.sh' -perm /111 -printf '%p\n' |
        LC_ALL=C sort
)

if [[ ${#TESTS[@]} -eq 0 ]]; then
    printf 'validator regression test directory exists but has no executable *.sh tests: %s\n' "$TEST_ROOT" >&2
    exit 1
fi

for test_script in "${TESTS[@]}"; do
    printf 'running validator regression: %s\n' "${test_script#$REPO_ROOT/}"
    (cd "$REPO_ROOT" && "$test_script" "$@")
done
