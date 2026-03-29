#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$SCRIPT_DIR
DEPENDENTS_JSON="$REPO_ROOT/dependents.json"
DEPENDENT_MATRIX="$REPO_ROOT/safe/tests/dependents/dependent_matrix.toml"
SOURCE_DIR="$REPO_ROOT/original/libzstd-1.5.5+dfsg2"
DOCKER_IMAGE=${DOCKER_IMAGE:-ubuntu:24.04}

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 1
fi

if [ ! -d "$SOURCE_DIR" ]; then
  echo "missing source tree: $SOURCE_DIR" >&2
  exit 1
fi

python3 - "$DEPENDENTS_JSON" "$DEPENDENT_MATRIX" "$REPO_ROOT" <<'PY'
import json
import pathlib
import sys
import tomllib

expected = {
    "apt",
    "btrfs-progs",
    "curl",
    "dpkg",
    "libarchive",
    "qemu",
    "rsync",
    "squashfs-tools",
    "systemd",
    "tiff",
}

with open(sys.argv[1], "r", encoding="utf-8") as f:
    data = json.load(f)
with open(sys.argv[2], "rb") as f:
    matrix = tomllib.load(f)

repo_root = pathlib.Path(sys.argv[3])

actual = {pkg["source_package"] for pkg in data["packages"]}
matrix_lookup = {entry["source_package"]: entry for entry in matrix["dependent"]}
matrix_sources = set(matrix_lookup)

missing = sorted(expected - actual)
extra = sorted(actual - expected)
matrix_missing = sorted(actual - matrix_sources)
matrix_extra = sorted(matrix_sources - actual)

if missing or extra or matrix_missing or matrix_extra:
    raise SystemExit(
        "dependents.json mismatch: "
        f"missing={missing or '[]'} extra={extra or '[]'} "
        f"matrix_missing={matrix_missing or '[]'} matrix_extra={matrix_extra or '[]'}"
    )

for source_package, entry in matrix_lookup.items():
    probe = repo_root / entry["compile_probe"]
    if not probe.is_file():
        raise SystemExit(f"missing dependent compile probe for {source_package}: {probe}")
PY

docker run --rm --privileged -i \
  -e HOST_UID="$(id -u)" \
  -e HOST_GID="$(id -g)" \
  -v "$REPO_ROOT:/work" \
  -w /work \
  "$DOCKER_IMAGE" \
  bash <<'EOF'
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

cleanup() {
  if [[ -d /work/safe/out ]]; then
    chown -R "$HOST_UID:$HOST_GID" /work/safe/out
  fi
}
trap cleanup EXIT

apt-get update >/dev/null
apt-get install -y --no-install-recommends \
  apt \
  apt-utils \
  btrfs-progs \
  build-essential \
  ca-certificates \
  cargo \
  cmake \
  curl \
  debhelper \
  devscripts \
  dh-package-notes \
  dpkg-dev \
  fakeroot \
  help2man \
  jq \
  libarchive-tools \
  liblz4-dev \
  liblzma-dev \
  libtiff-tools \
  less \
  pkgconf \
  python3 \
  python3-pil \
  qemu-utils \
  rsync \
  rustc \
  squashfs-tools \
  systemd \
  zlib1g-dev \
  zstd >/dev/null

export CARGO_HOME=/root/.cargo
export RUSTUP_HOME=/root/.rustup
export PATH="$CARGO_HOME/bin:$PATH"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null
cargo --version
rustc --version

DEB_HOST_MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
SAFE_UPSTREAM_LIB=/opt/safelibs/upstream/libzstd-upstream.so.1
install -d -m 0755 "$(dirname "$SAFE_UPSTREAM_LIB")"
cp -Lf "/usr/lib/$DEB_HOST_MULTIARCH/libzstd.so.1" "$SAFE_UPSTREAM_LIB"
chmod 0644 "$SAFE_UPSTREAM_LIB"
export SAFE_UPSTREAM_LIB

bash /work/safe/scripts/build-deb.sh
bash /work/safe/scripts/install-safe-debs.sh --skip-build

SAFE_VERSION=$(dpkg-query -W -f='${Version}' libzstd1)
for pkg in libzstd1 libzstd-dev zstd; do
  version=$(dpkg-query -W -f='${Version}' "$pkg")
  [[ $version == "$SAFE_VERSION" ]] || {
    echo "$pkg installed as $version instead of $SAFE_VERSION" >&2
    exit 1
  }
  [[ $version == *safelibs* ]] || {
    echo "$pkg is not using the safe package version: $version" >&2
    exit 1
  }
done

SAFE_LIB=/usr/lib/$DEB_HOST_MULTIARCH/libzstd.so.1
SAFE_LIB_REAL=$(readlink -f "$SAFE_LIB")

bash /work/safe/scripts/check-dependent-compile-compat.sh

TEST_ROOT=/tmp/libzstd-dependent-tests
rm -rf "$TEST_ROOT"
mkdir -p "$TEST_ROOT"

log() {
  printf '\n== %s ==\n' "$*"
}

assert_uses_safe_lib() {
  local target=$1
  local resolved
  resolved=$(ldd "$target" 2>/dev/null | awk '/libzstd\.so\.1/ {print $3; exit}')
  if [[ -z $resolved ]]; then
    echo "expected $target to link against libzstd.so.1" >&2
    ldd "$target" >&2 || true
    return 1
  fi
  if [[ $(readlink -f "$resolved") != "$SAFE_LIB_REAL" ]]; then
    echo "expected $target to resolve libzstd through $SAFE_LIB_REAL" >&2
    ldd "$target" >&2 || true
    return 1
  fi
}

ensure_loop_node() {
  local next num
  next=$(losetup -f)
  num=${next#/dev/loop}
  if [ ! -e "$next" ]; then
    mknod -m 660 "$next" b 7 "$num"
    chgrp disk "$next"
  fi
}

test_apt() {
  local dir server_pid
  dir=$TEST_ROOT/apt
  rm -rf "$dir"
  mkdir -p "$dir/pkg/DEBIAN" "$dir/pkg/usr/share/testpkg" "$dir/repo"

  assert_uses_safe_lib "/usr/lib/$DEB_HOST_MULTIARCH/libapt-pkg.so.6.0"

  cat >"$dir/pkg/DEBIAN/control" <<'CONTROL'
Package: testpkg
Version: 1.0
Section: misc
Priority: optional
Architecture: amd64
Maintainer: Test <test@example.com>
Description: test package for apt zstd metadata
CONTROL
  printf 'hello from apt repo\n' >"$dir/pkg/usr/share/testpkg/payload.txt"
  dpkg-deb -Zzstd -b "$dir/pkg" "$dir/testpkg_1.0_amd64.deb" >/dev/null

  cp "$dir/testpkg_1.0_amd64.deb" "$dir/repo/"
  (
    cd "$dir/repo"
    dpkg-scanpackages . /dev/null >Packages
    zstd -q -f Packages -o Packages.zst
    rm Packages
  )

  cat >"$dir/server.py" <<'PY'
import http.server
import socketserver

class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, fmt, *args):
        print(self.requestline, flush=True)

with socketserver.TCPServer(("127.0.0.1", 8000), Handler) as httpd:
    httpd.serve_forever()
PY

  (
    cd "$dir/repo"
    python3 "$dir/server.py" >"$dir/http.log" 2>&1
  ) &
  server_pid=$!
  sleep 1

  mkdir -p \
    "$dir/apt/etc/apt" \
    "$dir/apt/state/lists/partial" \
    "$dir/apt/cache/archives/partial" \
    "$dir/apt/sourceparts"
  printf 'deb [trusted=yes] http://127.0.0.1:8000 ./\n' >"$dir/apt/etc/apt/sources.list"

  apt-get update \
    -o Dir::Etc::sourcelist="$dir/apt/etc/apt/sources.list" \
    -o Dir::Etc::sourceparts="$dir/apt/sourceparts" \
    -o Dir::State="$dir/apt/state" \
    -o Dir::Cache="$dir/apt/cache" \
    -o Dir::State::status=/var/lib/dpkg/status \
    -o APT::Get::List-Cleanup=0 >/dev/null

  apt-cache policy testpkg \
    -o Dir::Etc::sourcelist="$dir/apt/etc/apt/sources.list" \
    -o Dir::Etc::sourceparts="$dir/apt/sourceparts" \
    -o Dir::State="$dir/apt/state" \
    -o Dir::Cache="$dir/apt/cache" \
    -o Dir::State::status=/var/lib/dpkg/status | grep -F 'Candidate: 1.0' >/dev/null

  grep -F 'GET /./Packages.zst HTTP/1.1' "$dir/http.log" >/dev/null

  kill "$server_pid"
  wait "$server_pid" || true
}

test_dpkg() {
  local dir
  dir=$TEST_ROOT/dpkg
  rm -rf "$dir"
  mkdir -p "$dir/pkg/DEBIAN" "$dir/pkg/usr/share/testpkg" "$dir/extract"

  assert_uses_safe_lib "$(command -v dpkg-deb)"

  cat >"$dir/pkg/DEBIAN/control" <<'CONTROL'
Package: testpkg
Version: 1.0
Section: misc
Priority: optional
Architecture: amd64
Maintainer: Test <test@example.com>
Description: test package for dpkg zstd members
CONTROL
  printf 'hello from dpkg\n' >"$dir/pkg/usr/share/testpkg/payload.txt"

  dpkg-deb -Zzstd -b "$dir/pkg" "$dir/testpkg_1.0_amd64.deb" >/dev/null
  dpkg-deb -I "$dir/testpkg_1.0_amd64.deb" | grep -F 'Package: testpkg' >/dev/null
  dpkg-deb -x "$dir/testpkg_1.0_amd64.deb" "$dir/extract"
  cmp "$dir/pkg/usr/share/testpkg/payload.txt" "$dir/extract/usr/share/testpkg/payload.txt"
}

test_rsync() {
  local dir daemon_pid
  dir=$TEST_ROOT/rsync
  rm -rf "$dir"
  mkdir -p "$dir/src" "$dir/dst"

  assert_uses_safe_lib "$(command -v rsync)"

  printf 'hello via rsync zstd\n' >"$dir/src/file.txt"
  cat >"$dir/rsyncd.conf" <<EOF2
pid file = $dir/rsyncd.pid
use chroot = false
log file = $dir/rsyncd.log
[files]
    path = $dir/src
    read only = true
EOF2

  rsync --daemon --no-detach --config="$dir/rsyncd.conf" >"$dir/daemon.out" 2>&1 &
  daemon_pid=$!
  sleep 1

  rsync -av --compress --compress-choice=zstd rsync://127.0.0.1/files/ "$dir/dst/" >"$dir/client.log"
  cmp "$dir/src/file.txt" "$dir/dst/file.txt"
  rsync --version | grep -F 'zstd' >/dev/null

  kill "$daemon_pid"
  wait "$daemon_pid" || true
}

test_systemd() {
  local dir journald_pid journal_file
  dir=$TEST_ROOT/systemd
  rm -rf "$dir"
  mkdir -p "$dir"

  assert_uses_safe_lib /usr/lib/systemd/systemd-journald

  rm -rf /run/log/journal /run/systemd/journal
  mkdir -p /etc/systemd /run/systemd/journal
  cat >/etc/systemd/journald.conf <<'CONF'
[Journal]
Storage=volatile
Compress=yes
CONF

  /usr/lib/systemd/systemd-journald >/tmp/systemd-journald.log 2>&1 &
  journald_pid=$!

  for _ in $(seq 1 20); do
    [ -S /run/systemd/journal/socket ] && break
    sleep 0.2
  done

  python3 - <<'PY' | systemd-cat -t zstd-test
print("A" * 200000)
PY

  : >"$dir/message.txt"
  for _ in $(seq 1 10); do
    journalctl --all --no-pager --directory=/run/log/journal -t zstd-test -o cat >"$dir/message.txt"
    [ -s "$dir/message.txt" ] && break
    sleep 1
  done

  python3 - <<'PY' "$dir/message.txt"
from pathlib import Path
import sys

message = Path(sys.argv[1]).read_text(encoding="utf-8")
payload = message.replace("\n", "")
assert set(payload) == {"A"}
assert len(payload) >= 200000, len(payload)
PY

  journal_file=$(find /run/log/journal -name system.journal | head -n1)
  python3 - <<'PY' "$journal_file"
from pathlib import Path
import sys

data = Path(sys.argv[1]).read_bytes()
magic = b"\x28\xb5\x2f\xfd"
assert data.find(magic) != -1
PY

  kill "$journald_pid"
  wait "$journald_pid" || true
}

test_libarchive() {
  local dir
  dir=$TEST_ROOT/libarchive
  rm -rf "$dir"
  mkdir -p "$dir/input/sub" "$dir/out"

  assert_uses_safe_lib "$(command -v bsdtar)"

  printf 'alpha\n' >"$dir/input/a.txt"
  printf 'beta\n' >"$dir/input/sub/b.txt"

  bsdtar --zstd -cf "$dir/archive.tar.zst" -C "$dir/input" .
  bsdtar -tf "$dir/archive.tar.zst" | grep -F './sub/b.txt' >/dev/null
  bsdtar -xf "$dir/archive.tar.zst" -C "$dir/out"
  diff -ru "$dir/input" "$dir/out"
}

test_btrfs() {
  local dir src_loop dst_loop
  dir=$TEST_ROOT/btrfs
  rm -rf "$dir"
  mkdir -p "$dir/mnt-src" "$dir/mnt-dst"

  assert_uses_safe_lib "$(command -v btrfs)"

  ensure_loop_node
  truncate -s 256M "$dir/src.img"
  truncate -s 256M "$dir/dst.img"
  mkfs.btrfs -q -f "$dir/src.img"
  mkfs.btrfs -q -f "$dir/dst.img"
  src_loop=$(losetup --find --show "$dir/src.img")
  ensure_loop_node
  dst_loop=$(losetup --find --show "$dir/dst.img")

  mount -o compress=zstd "$src_loop" "$dir/mnt-src"
  mount "$dst_loop" "$dir/mnt-dst"

  btrfs subvolume create "$dir/mnt-src/subvol" >/dev/null
  dd if=/dev/zero of="$dir/mnt-src/subvol/data.bin" bs=1M count=1 status=none
  sync
  btrfs filesystem sync "$dir/mnt-src"
  btrfs property set -ts "$dir/mnt-src/subvol" ro true
  btrfs send --compressed-data "$dir/mnt-src/subvol" | btrfs receive --force-decompress "$dir/mnt-dst" >/dev/null
  cmp "$dir/mnt-src/subvol/data.bin" "$dir/mnt-dst/subvol/data.bin"

  umount "$dir/mnt-src"
  umount "$dir/mnt-dst"
  losetup -d "$src_loop" "$dst_loop"
}

test_squashfs() {
  local dir
  dir=$TEST_ROOT/squashfs
  rm -rf "$dir"
  mkdir -p "$dir/input/sub"

  assert_uses_safe_lib "$(command -v mksquashfs)"

  printf 'gamma\n' >"$dir/input/g.txt"
  printf 'delta\n' >"$dir/input/sub/d.txt"

  mksquashfs "$dir/input" "$dir/test.sqfs" -comp zstd -noappend -quiet >/dev/null
  unsquashfs -d "$dir/out" "$dir/test.sqfs" >"$dir/unsquashfs.log"
  grep -F 'created 2 files' "$dir/unsquashfs.log" >/dev/null
  cmp "$dir/input/g.txt" "$dir/out/g.txt"
  cmp "$dir/input/sub/d.txt" "$dir/out/sub/d.txt"
}

test_qemu() {
  local dir
  dir=$TEST_ROOT/qemu
  rm -rf "$dir"
  mkdir -p "$dir"

  assert_uses_safe_lib "$(command -v qemu-img)"

  truncate -s 4M "$dir/raw.img"
  printf 'qcow-zstd\n' | dd of="$dir/raw.img" conv=notrunc status=none
  qemu-img convert -f raw -O qcow2 -c -o compression_type=zstd "$dir/raw.img" "$dir/image.qcow2"
  qemu-img info --output=json "$dir/image.qcow2" | jq -e '.["format-specific"].data["compression-type"] == "zstd"' >/dev/null
  qemu-img convert -f qcow2 -O raw "$dir/image.qcow2" "$dir/roundtrip.img"
  cmp "$dir/raw.img" "$dir/roundtrip.img"
}

test_curl() {
  local dir server_pid
  dir=$TEST_ROOT/curl
  rm -rf "$dir"
  mkdir -p "$dir"

  assert_uses_safe_lib "$(command -v curl)"

  printf 'curl zstd response\n' >"$dir/body.txt"
  zstd -q -f "$dir/body.txt" -o "$dir/body.zst"
  cat >"$dir/server.py" <<'PY'
from http.server import BaseHTTPRequestHandler, HTTPServer

BODY = open("/tmp/libzstd-dependent-tests/curl/body.zst", "rb").read()

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        self.send_header("Content-Encoding", "zstd")
        self.send_header("Content-Length", str(len(BODY)))
        self.end_headers()
        self.wfile.write(BODY)

    def log_message(self, fmt, *args):
        pass

HTTPServer(("127.0.0.1", 8001), Handler).serve_forever()
PY

  python3 "$dir/server.py" &
  server_pid=$!
  sleep 1

  curl --silent --show-error --compressed http://127.0.0.1:8001/ >"$dir/out.txt"
  cmp "$dir/body.txt" "$dir/out.txt"

  kill "$server_pid"
  wait "$server_pid" || true
}

test_tiff() {
  local dir
  dir=$TEST_ROOT/tiff
  rm -rf "$dir"
  mkdir -p "$dir"

  assert_uses_safe_lib "$(command -v tiffcp)"

  python3 - <<'PY'
from PIL import Image
Image.new("RGB", (8, 8), (12, 34, 56)).save("/tmp/libzstd-dependent-tests/tiff/input.tif", compression="raw")
PY

  tiffcp -c zstd "$dir/input.tif" "$dir/zstd.tif"
  tiffinfo "$dir/zstd.tif" | grep -F 'Compression Scheme: ZSTD' >/dev/null
  tiffcmp "$dir/input.tif" "$dir/zstd.tif" >/dev/null
}

mapfile -t runtime_tests < <(python3 - <<'PY'
from __future__ import annotations

import json
import tomllib

with open("/work/dependents.json", "r", encoding="utf-8") as handle:
    dependents = json.load(handle)
with open("/work/safe/tests/dependents/dependent_matrix.toml", "rb") as handle:
    matrix = tomllib.load(handle)

runtime_lookup = {entry["source_package"]: entry["runtime_test"] for entry in matrix["dependent"]}
binary_lookup = {entry["source_package"]: entry["binary_package"] for entry in matrix["dependent"]}

for entry in dependents["packages"]:
    source_package = entry["source_package"]
    print(f"{source_package}\t{binary_lookup[source_package]}\t{runtime_lookup[source_package]}")
PY
)

for entry in "${runtime_tests[@]}"; do
  IFS=$'\t' read -r source_package binary_package runtime_test <<<"$entry"
  log "$source_package ($binary_package)"
  "$runtime_test"
done

log "all dependent tests passed"
EOF
