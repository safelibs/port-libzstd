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

run_step "phase 4 release install tree" bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
run_step "phase 4 original cli helper" bash "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh"
run_step "phase 4 debian staging" bash "$SAFE_ROOT/scripts/build-deb.sh"
run_step "header identity" bash "$SAFE_ROOT/scripts/verify-header-identity.sh"
run_step "baseline contract" bash "$SAFE_ROOT/scripts/verify-baseline-contract.sh"
run_step "export parity" bash "$SAFE_ROOT/scripts/verify-export-parity.sh"
run_step "link compatibility" bash "$SAFE_ROOT/scripts/verify-link-compat.sh"
run_step "rust tests" cargo test --manifest-path "$SAFE_ROOT/Cargo.toml" --release --all-targets
run_step "install layout variants" bash "$SAFE_ROOT/scripts/run-build-variant-tests.sh"
run_step "debian profile outputs" bash "$SAFE_ROOT/scripts/verify-deb-profiles.sh"
run_step "debian autopkgtests" bash "$SAFE_ROOT/scripts/run-debian-autopkgtests.sh"
run_step "c api decompression" bash "$SAFE_ROOT/scripts/run-capi-decompression.sh"
run_step "c api roundtrip" bash "$SAFE_ROOT/scripts/run-capi-roundtrip.sh"
run_step "upstream core release gates" bash "$SAFE_ROOT/scripts/run-upstream-tests.sh"
run_step "upstream playtests and variants" bash "$SAFE_ROOT/scripts/run-original-playtests.sh"
run_step "upstream gzip compatibility" bash "$SAFE_ROOT/scripts/run-original-gzip-tests.sh"
run_step "upstream zlib wrapper" bash "$SAFE_ROOT/scripts/run-zlibwrapper-tests.sh"
run_step "upstream educational decoder" bash "$SAFE_ROOT/scripts/run-educational-decoder-tests.sh"
run_step "upstream pzstd" bash "$SAFE_ROOT/scripts/run-pzstd-tests.sh"
run_step "upstream examples" bash "$SAFE_ROOT/scripts/run-original-examples.sh"
run_step "seekable format" bash "$SAFE_ROOT/scripts/run-seekable-tests.sh"
run_step "downstream image matrix" bash "$REPO_ROOT/test-original.sh"
