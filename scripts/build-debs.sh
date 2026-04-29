#!/usr/bin/env bash
# libzstd: drive the port-owned safe/scripts/build-deb.sh, which writes
# .deb artifacts under safe/out/.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=/dev/null
. "$repo_root/scripts/lib/build-deb-common.sh"

prepare_rust_env
prepare_dist_dir "$repo_root"

cd "$repo_root"
bash safe/scripts/build-deb.sh

shopt -s nullglob
debs=(safe/out/*.deb)
shopt -u nullglob
if (( ${#debs[@]} == 0 )); then
  printf 'build-debs: no *.deb under safe/out/\n' >&2
  exit 1
fi
cp -v "${debs[@]}" "$repo_root/dist"/
