#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)

run_step() {
    local label=$1
    shift
    printf '\n== %s ==\n' "$label"
    "$@"
}

run_step "export parity" bash "$SAFE_ROOT/scripts/verify-export-parity.sh"
run_step "link compatibility" bash "$SAFE_ROOT/scripts/verify-link-compat.sh"
run_step "rust tests" cargo test --manifest-path "$SAFE_ROOT/Cargo.toml" --release --all-targets
run_step "install layout variants" bash "$SAFE_ROOT/scripts/run-build-variant-tests.sh"
run_step "debian profile outputs" bash "$SAFE_ROOT/scripts/verify-deb-profiles.sh"
run_step "debian autopkgtests" bash "$SAFE_ROOT/scripts/run-debian-autopkgtests.sh"
run_step "c api decompression" bash "$SAFE_ROOT/scripts/run-capi-decompression.sh"
run_step "c api roundtrip" bash "$SAFE_ROOT/scripts/run-capi-roundtrip.sh"
run_step "version compatibility" bash "$SAFE_ROOT/scripts/run-version-compat-tests.sh"
run_step "original cli" bash "$SAFE_ROOT/scripts/run-original-cli-tests.sh"
run_step "seekable format" bash "$SAFE_ROOT/scripts/run-seekable-tests.sh"
run_step "downstream dependents" bash "$REPO_ROOT/test-original.sh"
