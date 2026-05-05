#!/usr/bin/env bash
# Install apt packages and a rust toolchain (read from
# safe/rust-toolchain.toml when present, falling back to stable) needed
# for libzstd's safe build. The port's safe/scripts/build-deb.sh runs
# cmake + dpkg-buildpackage and pulls in additional system libraries
# (lz4, lzma, zlib) plus debhelper/dh-package-notes/help2man/less that
# don't come with the runner-default ubuntu-latest image.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

toolchain="${SAFELIBS_RUST_TOOLCHAIN:-}"
if [[ -z "$toolchain" && -f "$repo_root/safe/rust-toolchain.toml" ]]; then
  toolchain="$(grep -oP '^channel\s*=\s*"\K[^"]+' "$repo_root/safe/rust-toolchain.toml" || true)"
fi
toolchain="${toolchain:-stable}"

export DEBIAN_FRONTEND=noninteractive
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  cmake \
  curl \
  debhelper \
  devscripts \
  dh-package-notes \
  dpkg-dev \
  equivs \
  fakeroot \
  file \
  git \
  help2man \
  jq \
  less \
  liblz4-dev \
  liblzma-dev \
  python3 \
  rsync \
  xz-utils \
  zlib1g-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain "$toolchain" --no-modify-path

# shellcheck source=/dev/null
. "$HOME/.cargo/env"
rustup default "$toolchain"
rustc --version
cargo --version

if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$HOME/.cargo/bin" >> "$GITHUB_PATH"
fi
