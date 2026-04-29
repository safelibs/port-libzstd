#!/usr/bin/env bash
# libzstd: drive the port-owned safe/scripts/build-deb.sh, which writes
# .deb artifacts under safe/out/.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_root/dist"

# shellcheck source=/dev/null
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

if [[ -d "$HOME/.cargo/bin" ]]; then
  case ":$PATH:" in
    *":$HOME/.cargo/bin:"*) ;;
    *) export PATH="$HOME/.cargo/bin:$PATH" ;;
  esac
fi

rm -rf -- "$dist_dir"
mkdir -p -- "$dist_dir"

cd "$repo_root"
bash safe/scripts/build-deb.sh

shopt -s nullglob
debs=(safe/out/*.deb)
shopt -u nullglob
if (( ${#debs[@]} == 0 )); then
  printf 'build-debs: no *.deb under safe/out/\n' >&2
  exit 1
fi
cp -v "${debs[@]}" "$dist_dir"/
