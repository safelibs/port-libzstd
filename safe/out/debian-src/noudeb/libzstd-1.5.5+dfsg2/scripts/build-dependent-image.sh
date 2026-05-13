#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
DEPENDENT_ROOT="$SAFE_ROOT/out/dependents"
IMAGE_CONTEXT_ROOT="$DEPENDENT_ROOT/image-context"
LOG_ROOT="$DEPENDENT_ROOT/logs"
COMPILE_ROOT="$DEPENDENT_ROOT/compile-compat"
IMAGE_METADATA_FILE="$IMAGE_CONTEXT_ROOT/metadata.env"
DEPENDENT_IMAGE='safelibs-libzstd-dependents:ubuntu24.04'
DEPENDENT_BASE_IMAGE='ubuntu:24.04'

source "$SAFE_ROOT/scripts/phase6-common.sh"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

bash "$SAFE_ROOT/scripts/build-artifacts.sh" --release
bash "$SAFE_ROOT/scripts/build-original-cli-against-safe.sh"
bash "$SAFE_ROOT/scripts/build-deb.sh"

phase6_require_phase4_inputs "$SCRIPT_DIR/build-dependent-image.sh"

rm -rf "$IMAGE_CONTEXT_ROOT"
install -d \
  "$IMAGE_CONTEXT_ROOT" \
  "$IMAGE_CONTEXT_ROOT/safe/out/deb/default/packages" \
  "$IMAGE_CONTEXT_ROOT/safe/tests/dependents" \
  "$IMAGE_CONTEXT_ROOT/safe/scripts" \
  "$IMAGE_CONTEXT_ROOT/safe/docker/dependents" \
  "$LOG_ROOT" \
  "$COMPILE_ROOT"

install -m 644 "$REPO_ROOT/dependents.json" "$IMAGE_CONTEXT_ROOT/dependents.json"
install -m 644 "$PHASE4_METADATA_FILE" "$IMAGE_CONTEXT_ROOT/safe/out/deb/default/metadata.env"
install -m 755 \
  "$SAFE_ROOT/scripts/check-dependent-compile-compat.sh" \
  "$IMAGE_CONTEXT_ROOT/safe/scripts/check-dependent-compile-compat.sh"
install -m 755 \
  "$SAFE_ROOT/docker/dependents/entrypoint.sh" \
  "$IMAGE_CONTEXT_ROOT/safe/docker/dependents/entrypoint.sh"
rsync -a --delete "$PHASE6_DEB_PACKAGE_DIR/" "$IMAGE_CONTEXT_ROOT/safe/out/deb/default/packages/"
rsync -a --delete "$SAFE_ROOT/tests/dependents/" "$IMAGE_CONTEXT_ROOT/safe/tests/dependents/"

cat >"$IMAGE_METADATA_FILE" <<EOF
DEPENDENT_IMAGE='$DEPENDENT_IMAGE'
DEPENDENT_BASE_IMAGE='$DEPENDENT_BASE_IMAGE'
EOF

docker build \
  --build-arg "BASE_IMAGE=$DEPENDENT_BASE_IMAGE" \
  --file "$SAFE_ROOT/docker/dependents/Dockerfile" \
  --tag "$DEPENDENT_IMAGE" \
  "$IMAGE_CONTEXT_ROOT"

phase6_log "built dependent image $DEPENDENT_IMAGE from $DEPENDENT_BASE_IMAGE"
phase6_log "staged image context under $IMAGE_CONTEXT_ROOT"
