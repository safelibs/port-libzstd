#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$SCRIPT_DIR
DEPENDENTS_JSON="$REPO_ROOT/dependents.json"
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

python3 - "$DEPENDENTS_JSON" <<'PY'
import json
import sys

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

actual = {pkg["source_package"] for pkg in data["packages"]}

missing = sorted(expected - actual)
extra = sorted(actual - expected)

if missing or extra:
    raise SystemExit(
        "dependents.json mismatch: "
        f"missing={missing or '[]'} extra={extra or '[]'}"
    )
PY

docker run --rm --privileged -i \
  -v "$REPO_ROOT:/work" \
  -w /work \
  "$DOCKER_IMAGE" \
  bash <<'EOF'
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive
apt-get update >/dev/null
apt-get install -y --no-install-recommends \
  apt \
  apt-utils \
  btrfs-progs \
  build-essential \
  curl \
  dpkg-dev \
  jq \
  libarchive-tools \
  libtiff-tools \
  python3 \
  python3-pil \
  qemu-utils \
  rsync \
  squashfs-tools \
  systemd \
  zstd >/dev/null

make -j"$(nproc)" -C /work/original/libzstd-1.5.5+dfsg2 install prefix=/opt/libzstd >/dev/null

export PATH="/opt/libzstd/bin:$PATH"
export LD_LIBRARY_PATH="/opt/libzstd/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PKG_CONFIG_PATH="/opt/libzstd/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

TEST_ROOT=/tmp/libzstd-dependent-tests
rm -rf "$TEST_ROOT"
mkdir -p "$TEST_ROOT"

LOCAL_LIB=/opt/libzstd/lib/libzstd.so.1

log() {
  printf '\n== %s ==\n' "$*"
}

assert_uses_local_lib() {
  local target=$1
  if ! ldd "$target" | grep -F "$LOCAL_LIB" >/dev/null; then
    echo "expected $target to resolve libzstd through $LOCAL_LIB" >&2
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

  assert_uses_local_lib /usr/lib/x86_64-linux-gnu/libapt-pkg.so.6.0

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

  assert_uses_local_lib "$(command -v dpkg-deb)"

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

  assert_uses_local_lib "$(command -v rsync)"

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

  assert_uses_local_lib /usr/lib/systemd/systemd-journald

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

  assert_uses_local_lib "$(command -v bsdtar)"

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

  assert_uses_local_lib "$(command -v btrfs)"

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

  assert_uses_local_lib "$(command -v mksquashfs)"

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

  assert_uses_local_lib "$(command -v qemu-img)"

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

  assert_uses_local_lib "$(command -v curl)"

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

  assert_uses_local_lib "$(command -v tiffcp)"

  python3 - <<'PY'
from PIL import Image
Image.new("RGB", (8, 8), (12, 34, 56)).save("/tmp/libzstd-dependent-tests/tiff/input.tif", compression="raw")
PY

  tiffcp -c zstd "$dir/input.tif" "$dir/zstd.tif"
  tiffinfo "$dir/zstd.tif" | grep -F 'Compression Scheme: ZSTD' >/dev/null
  tiffcmp "$dir/input.tif" "$dir/zstd.tif" >/dev/null
}

log "APT"
test_apt

log "dpkg"
test_dpkg

log "rsync"
test_rsync

log "systemd"
test_systemd

log "libarchive"
test_libarchive

log "btrfs-progs"
test_btrfs

log "squashfs-tools"
test_squashfs

log "qemu-utils"
test_qemu

log "curl"
test_curl

log "libtiff"
test_tiff

log "all dependent tests passed"
EOF
