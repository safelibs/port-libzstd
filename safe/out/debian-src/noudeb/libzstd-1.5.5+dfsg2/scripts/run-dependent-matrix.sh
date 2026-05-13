#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SAFE_ROOT/.." && pwd)
source "$SAFE_ROOT/scripts/phase6-common.sh"
DEPENDENT_ROOT="$SAFE_ROOT/out/dependents"
IMAGE_METADATA_FILE="$DEPENDENT_ROOT/image-context/metadata.env"
LOG_ROOT="$DEPENDENT_ROOT/logs"
COMPILE_ROOT="$DEPENDENT_ROOT/compile-compat"
STAMP_ROOT="$DEPENDENT_ROOT/stamps"

usage() {
  cat <<'EOF'
usage: run-dependent-matrix.sh [--compile-only | --runtime-only] [--apps a,b]
EOF
}

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

mode=all
apps_csv=

while [[ $# -gt 0 ]]; do
  case "$1" in
    --compile-only)
      [[ $mode == all ]] || {
        echo "choose only one of --compile-only or --runtime-only" >&2
        exit 2
      }
      mode=compile
      ;;
    --runtime-only)
      [[ $mode == all ]] || {
        echo "choose only one of --compile-only or --runtime-only" >&2
        exit 2
      }
      mode=runtime
      ;;
    --apps)
      apps_csv=${2:?missing app list}
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

if [[ $mode == compile && -n $apps_csv ]]; then
  echo "--apps can only be used with runtime or all modes" >&2
  exit 2
fi

[[ -f $IMAGE_METADATA_FILE ]] || {
  echo "missing dependent image metadata: $IMAGE_METADATA_FILE" >&2
  echo "run bash safe/scripts/build-dependent-image.sh first" >&2
  exit 1
}

source "$IMAGE_METADATA_FILE"

[[ -n ${DEPENDENT_IMAGE:-} ]] || {
  echo "metadata file is missing DEPENDENT_IMAGE: $IMAGE_METADATA_FILE" >&2
  exit 1
}

docker image inspect "$DEPENDENT_IMAGE" >/dev/null 2>&1 || {
  echo "missing dependent image: $DEPENDENT_IMAGE" >&2
  echo "run bash safe/scripts/build-dependent-image.sh first" >&2
  exit 1
}

install -d "$LOG_ROOT" "$COMPILE_ROOT"
install -d "$STAMP_ROOT"

log_suffix=$mode
if [[ -n $apps_csv ]]; then
  log_suffix+="-${apps_csv//,/-}"
fi
log_file="$LOG_ROOT/$log_suffix.log"
temp_log_file="$LOG_ROOT/$log_suffix.log.tmp.$$"
stamp_file="$STAMP_ROOT/$log_suffix.stamp"

if phase6_stamp_is_fresh \
  "$stamp_file" \
  "$0" \
  "$SAFE_ROOT/abi/export_map.toml" \
  "$SAFE_ROOT/Cargo.toml" \
  "$SAFE_ROOT/out/deb/default/metadata.env" \
  "$IMAGE_METADATA_FILE" \
  "$REPO_ROOT/dependents.json" \
  && phase6_tracked_repo_paths_are_fresh \
    "$stamp_file" \
    "$SAFE_ROOT/tests/dependents" \
    "$SAFE_ROOT/docker/dependents" \
    "$SAFE_ROOT/include" \
    "$SAFE_ROOT/src"
then
  echo "dependent matrix mode '$mode' already fresh; skipping rerun" >&2
  exit 0
fi

rm -f "$temp_log_file"
trap 'rm -f "$temp_log_file"' EXIT

if [[ $mode == compile || $mode == all ]]; then
  find "$COMPILE_ROOT" -mindepth 1 -maxdepth 1 -exec rm -rf {} + 2>/dev/null || true
fi

docker_args=(
  docker run --rm
  -e "HOST_UID=$(id -u)"
  -e "HOST_GID=$(id -g)"
  -e DEPENDENT_LOG_DIR=/out/logs
  -e DEPENDENT_BUILD_DIR=/out/compile-compat
  -e "DEPENDENT_LOG_FILE=/out/logs/$(basename "$temp_log_file")"
  -v "$LOG_ROOT:/out/logs"
  -v "$COMPILE_ROOT:/out/compile-compat"
)

if [[ $mode != compile ]]; then
  docker_args+=(--privileged --tmpfs /run --tmpfs /run/lock)
fi

docker_args+=("$DEPENDENT_IMAGE")
case "$mode" in
  compile)
    docker_args+=(compile)
    ;;
  runtime)
    docker_args+=(runtime)
    ;;
  all)
    docker_args+=(all)
    ;;
esac

if [[ -n $apps_csv ]]; then
  docker_args+=("$apps_csv")
fi

"${docker_args[@]}"
mv -f "$temp_log_file" "$log_file"
trap - EXIT
touch "$stamp_file"
