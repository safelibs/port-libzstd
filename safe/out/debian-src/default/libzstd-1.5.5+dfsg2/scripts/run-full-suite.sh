#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
DEPENDENT_IMAGE_METADATA_FILE="$SAFE_ROOT/out/dependents/image-context/metadata.env"

source "$SAFE_ROOT/scripts/phase6-common.sh"

run_step() {
    local label=$1
    shift
    printf '\n== %s ==\n' "$label"
    "$@"
}

require_dependent_image() {
    [[ -f $DEPENDENT_IMAGE_METADATA_FILE ]] || {
        printf 'missing dependent image metadata: %s\n' "$DEPENDENT_IMAGE_METADATA_FILE" >&2
        printf 'run bash safe/scripts/build-dependent-image.sh first\n' >&2
        exit 1
    }

    # shellcheck disable=SC1090
    source "$DEPENDENT_IMAGE_METADATA_FILE"

    [[ -n ${DEPENDENT_IMAGE:-} ]] || {
        printf 'metadata file is missing DEPENDENT_IMAGE: %s\n' "$DEPENDENT_IMAGE_METADATA_FILE" >&2
        exit 1
    }

    if ! command -v docker >/dev/null 2>&1; then
        printf 'docker is required\n' >&2
        exit 1
    fi

    docker image inspect "$DEPENDENT_IMAGE" >/dev/null 2>&1 || {
        printf 'missing dependent image: %s\n' "$DEPENDENT_IMAGE" >&2
        printf 'run bash safe/scripts/build-dependent-image.sh first\n' >&2
        exit 1
    }
}

phase6_require_phase4_inputs "$0"
require_dependent_image
phase6_log "running the final release gate against the existing Phase 4 and Phase 6 artifact roots"

run_step "rust tests" cargo test --manifest-path "$SAFE_ROOT/Cargo.toml" --release --all-targets
run_step "header identity" bash "$SAFE_ROOT/scripts/verify-header-identity.sh"
run_step "baseline contract" bash "$SAFE_ROOT/scripts/verify-baseline-contract.sh"
run_step "export parity" bash "$SAFE_ROOT/scripts/verify-export-parity.sh"
run_step "link compatibility" bash "$SAFE_ROOT/scripts/verify-link-compat.sh"
run_step "c api decompression" bash "$SAFE_ROOT/scripts/run-capi-decompression.sh"
run_step "c api roundtrip" bash "$SAFE_ROOT/scripts/run-capi-roundtrip.sh"
run_step "advanced mt tests" bash "$SAFE_ROOT/scripts/run-advanced-mt-tests.sh"
run_step "install layout" bash "$SAFE_ROOT/scripts/verify-install-layout.sh"
run_step "debian install layout" bash "$SAFE_ROOT/scripts/verify-install-layout.sh" --debian
run_step "install layout variants" bash "$SAFE_ROOT/scripts/run-build-variant-tests.sh"
run_step "debian profile outputs" bash "$SAFE_ROOT/scripts/verify-deb-profiles.sh"
run_step "debian autopkgtests" bash "$SAFE_ROOT/scripts/run-debian-autopkgtests.sh"
run_step "upstream core release gates" bash "$SAFE_ROOT/scripts/run-upstream-tests.sh"
run_step "upstream playtests and variants" bash "$SAFE_ROOT/scripts/run-original-playtests.sh"
run_step "upstream original cli" bash "$SAFE_ROOT/scripts/run-original-cli-tests.sh"
run_step "upstream gzip compatibility" bash "$SAFE_ROOT/scripts/run-original-gzip-tests.sh"
run_step "upstream zlib wrapper" bash "$SAFE_ROOT/scripts/run-zlibwrapper-tests.sh"
run_step "upstream educational decoder" bash "$SAFE_ROOT/scripts/run-educational-decoder-tests.sh"
run_step "upstream pzstd" bash "$SAFE_ROOT/scripts/run-pzstd-tests.sh"
run_step "upstream examples" bash "$SAFE_ROOT/scripts/run-original-examples.sh"
run_step "seekable format" bash "$SAFE_ROOT/scripts/run-seekable-tests.sh"
run_step "upstream version compatibility" bash "$SAFE_ROOT/scripts/run-version-compat-tests.sh"
run_step "upstream offline regression" bash "$SAFE_ROOT/scripts/run-upstream-regression.sh"
run_step "upstream fuzz corpus" bash "$SAFE_ROOT/scripts/run-upstream-fuzz-tests.sh"
run_step "cli permissions audit" bash "$SAFE_ROOT/scripts/check-cli-permissions.sh"
run_step "performance smoke" bash "$SAFE_ROOT/scripts/run-performance-smoke.sh"
run_step "downstream compile compatibility" bash "$SAFE_ROOT/scripts/run-dependent-matrix.sh" --compile-only
run_step "downstream runtime coverage" bash "$SAFE_ROOT/scripts/run-dependent-matrix.sh" --runtime-only
