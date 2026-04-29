#!/usr/bin/env bash
# Install apt packages and rust 1.94.0 (pinned by the port) for libzstd.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  cmake \
  curl \
  debhelper \
  dh-package-notes \
  devscripts \
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
  | sh -s -- -y --profile minimal --default-toolchain 1.94.0 --no-modify-path

# shellcheck source=/dev/null
. "$HOME/.cargo/env"
rustup default 1.94.0
rustc --version
cargo --version

if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$HOME/.cargo/bin" >> "$GITHUB_PATH"
fi
