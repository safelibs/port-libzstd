#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=/opt/libzstd-matrix
SAFE_ROOT="$REPO_ROOT/safe"
DEPENDENTS_JSON="$REPO_ROOT/dependents.json"
DEPENDENT_MATRIX="$SAFE_ROOT/tests/dependents/dependent_matrix.toml"
DEPENDENT_LOG_DIR=${DEPENDENT_LOG_DIR:-/out/logs}
DEPENDENT_BUILD_DIR=${DEPENDENT_BUILD_DIR:-/out/compile-compat}
DEPENDENT_LOG_FILE=${DEPENDENT_LOG_FILE:-}
HOST_UID=${HOST_UID:-}
HOST_GID=${HOST_GID:-}
TEST_ROOT=/tmp/libzstd-dependent-tests

install -d "$DEPENDENT_LOG_DIR" "$DEPENDENT_BUILD_DIR"

fix_output_permissions() {
  if [[ -n $HOST_UID && -n $HOST_GID ]]; then
    chown -R "$HOST_UID:$HOST_GID" "$DEPENDENT_LOG_DIR" "$DEPENDENT_BUILD_DIR" 2>/dev/null || true
  fi
}
trap fix_output_permissions EXIT

setup_logging() {
  if [[ -n $DEPENDENT_LOG_FILE ]]; then
    install -d "$(dirname "$DEPENDENT_LOG_FILE")"
    : >"$DEPENDENT_LOG_FILE"
    exec > >(tee -a "$DEPENDENT_LOG_FILE") 2>&1
  fi
}

setup_logging

DEB_HOST_MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
SAFE_LIB=/usr/lib/$DEB_HOST_MULTIARCH/libzstd.so.1
SAFE_LIB_REAL=$(readlink -f "$SAFE_LIB")

usage() {
  cat <<'EOF'
usage: entrypoint.sh [compile|runtime|all] [source_package[,source_package...]]
EOF
}

log() {
  printf '\n== %s ==\n' "$*"
}

check_inventory_consistency() {
  python3 - "$DEPENDENTS_JSON" "$DEPENDENT_MATRIX" "$REPO_ROOT" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys
import tomllib

expected_sources = [
    "apt",
    "dpkg",
    "rsync",
    "systemd",
    "libarchive",
    "btrfs-progs",
    "squashfs-tools",
    "qemu",
    "curl",
    "tiff",
    "rpm",
    "zarchive",
]
expected_runtime = {
    "apt": "test_apt",
    "dpkg": "test_dpkg",
    "rsync": "test_rsync",
    "systemd": "test_systemd",
    "libarchive": "test_libarchive",
    "btrfs-progs": "test_btrfs",
    "squashfs-tools": "test_squashfs",
    "qemu": "test_qemu",
    "curl": "test_curl",
    "tiff": "test_tiff",
    "rpm": "test_rpm",
    "zarchive": "test_zarchive",
}

dependents_path = pathlib.Path(sys.argv[1])
matrix_path = pathlib.Path(sys.argv[2])
repo_root = pathlib.Path(sys.argv[3])

dependents = json.loads(dependents_path.read_text(encoding="utf-8"))
matrix = tomllib.loads(matrix_path.read_text(encoding="utf-8"))

json_order = [entry["source_package"] for entry in dependents["packages"]]
matrix_entries = matrix["dependent"]
matrix_order = [entry["source_package"] for entry in matrix_entries]
runtime_lookup = {entry["source_package"]: entry["runtime_test"] for entry in matrix_entries}

if json_order != expected_sources:
    raise SystemExit(f"dependents.json mismatch: {json_order}")
if matrix_order != expected_sources:
    raise SystemExit(f"dependent_matrix.toml mismatch: {matrix_order}")
if runtime_lookup != expected_runtime:
    raise SystemExit(f"dependent runtime mismatch: {runtime_lookup}")

json_lookup = {entry["source_package"]: entry for entry in dependents["packages"]}
for entry in matrix_entries:
    source_package = entry["source_package"]
    probe = repo_root / entry["compile_probe"]
    if not probe.is_file():
        raise SystemExit(f"missing dependent compile probe for {source_package}: {probe}")
    if entry["binary_package"] != json_lookup[source_package]["binary_package"]:
        raise SystemExit(f"binary package mismatch for {source_package}")
PY
}

check_installed_safe_packages() {
  local safe_version expected_version version pkg
  local pkg_dir=$SAFE_ROOT/out/deb/default/packages
  local -a lib_debs=("$pkg_dir"/libzstd1_*.deb)

  [[ -f $SAFE_LIB_REAL ]] || {
    echo "installed safe library missing: $SAFE_LIB" >&2
    exit 1
  }
  [[ ${#lib_debs[@]} -eq 1 ]] || {
    echo "expected exactly one staged libzstd1 .deb under $pkg_dir" >&2
    exit 1
  }

  safe_version=$(dpkg-query -W -f='${Version}' libzstd1)
  expected_version=$(dpkg-deb -f "${lib_debs[0]}" Version)
  [[ $safe_version == "$expected_version" ]] || {
    echo "libzstd1 installed as $safe_version instead of staged $expected_version" >&2
    exit 1
  }
  for pkg in libzstd1 libzstd-dev zstd; do
    version=$(dpkg-query -W -f='${Version}' "$pkg")
    [[ $version == "$safe_version" ]] || {
      echo "$pkg installed as $version instead of $safe_version" >&2
      exit 1
    }
    [[ $version == *safelibs* ]] || {
      echo "$pkg is not using the safe package version: $version" >&2
      exit 1
    }
  done
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
  if [[ ! -e $next ]]; then
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
    [[ -S /run/systemd/journal/socket ]] && break
    sleep 0.2
  done

  python3 - <<'PY' | systemd-cat -t zstd-test
print("A" * 200000)
PY

  : >"$dir/message.txt"
  for _ in $(seq 1 10); do
    journalctl --all --no-pager --directory=/run/log/journal -t zstd-test -o cat >"$dir/message.txt"
    [[ -s $dir/message.txt ]] && break
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

test_rpm() {
  local dir fixture_root rpm_root rpm_path
  dir=$TEST_ROOT/rpm
  fixture_root=$SAFE_ROOT/tests/dependents/fixtures/rpm
  rpm_root=$dir/rpmbuild
  rm -rf "$dir"
  mkdir -p \
    "$rpm_root/BUILD" \
    "$rpm_root/BUILDROOT" \
    "$rpm_root/RPMS" \
    "$rpm_root/SOURCES" \
    "$rpm_root/SPECS" \
    "$rpm_root/SRPMS"

  assert_uses_safe_lib /usr/bin/rpm
  assert_uses_safe_lib /usr/bin/rpmbuild
  assert_uses_safe_lib /usr/bin/rpm2cpio

  cp -a "$fixture_root/hello.txt" "$rpm_root/SOURCES/hello.txt"
  cp -a "$fixture_root/hello.spec" "$rpm_root/SPECS/hello.spec"

  rpmbuild \
    --define "_topdir $rpm_root" \
    --define '_binary_payload w19.zstdio' \
    -bb "$rpm_root/SPECS/hello.spec" >/dev/null

  rpm_path=$(find "$rpm_root/RPMS" -name '*.rpm' -print -quit)
  [[ -n $rpm_path ]] || {
    echo "rpmbuild did not produce an rpm payload" >&2
    exit 1
  }
  [[ $(rpm -qp --qf '%{PAYLOADCOMPRESSOR}\n' "$rpm_path") == zstd ]] || {
    echo "rpm payload compressor was not zstd" >&2
    exit 1
  }

  mkdir -p "$dir/extract"
  (
    cd "$dir/extract"
    rpm2cpio "$rpm_path" | cpio -idmu >/dev/null
  )
  cmp "$fixture_root/hello.txt" "$dir/extract/usr/share/hello-rpm/hello.txt"
}

test_zarchive() {
  local dir fixture_root
  dir=$TEST_ROOT/zarchive
  fixture_root=$SAFE_ROOT/tests/dependents/fixtures/zarchive/input
  rm -rf "$dir"
  mkdir -p "$TEST_ROOT/zarchive"

  assert_uses_safe_lib /usr/bin/zarchive

  cp -a "$fixture_root" "$dir/in"
  zarchive "$dir/in" "$dir/archive.za"
  zarchive "$dir/archive.za" "$dir/out"
  diff -ru "$dir/in" "$dir/out"
}

list_runtime_tests() {
  python3 - "$DEPENDENTS_JSON" "$DEPENDENT_MATRIX" "${1:-}" <<'PY'
from __future__ import annotations

import json
import sys
import tomllib

dependents_path = sys.argv[1]
matrix_path = sys.argv[2]
requested = [item for item in sys.argv[3].split(",") if item] if len(sys.argv) > 3 and sys.argv[3] else []

with open(dependents_path, "r", encoding="utf-8") as handle:
    dependents = json.load(handle)
with open(matrix_path, "rb") as handle:
    matrix = tomllib.load(handle)

matrix_lookup = {entry["source_package"]: entry for entry in matrix["dependent"]}
known_sources = {entry["source_package"] for entry in dependents["packages"]}
requested_set = set(requested)
unknown = sorted(requested_set - known_sources)
if unknown:
    raise SystemExit(f"unknown dependent apps: {unknown}")

for entry in dependents["packages"]:
    source_package = entry["source_package"]
    if requested and source_package not in requested_set:
        continue
    matrix_entry = matrix_lookup[source_package]
    print(
        f"{source_package}\t{matrix_entry['binary_package']}\t{matrix_entry['runtime_test']}"
    )
PY
}

run_compile_suite() {
  log "compile compatibility"
  check_inventory_consistency
  check_installed_safe_packages
  DEPENDENT_BUILD_DIR="$DEPENDENT_BUILD_DIR" bash "$SAFE_ROOT/scripts/check-dependent-compile-compat.sh"
}

run_runtime_suite() {
  local apps_csv=${1:-}
  local entry source_package binary_package runtime_test

  check_inventory_consistency
  check_installed_safe_packages
  rm -rf "$TEST_ROOT"
  mkdir -p "$TEST_ROOT"

  mapfile -t runtime_entries < <(list_runtime_tests "$apps_csv")
  for entry in "${runtime_entries[@]}"; do
    IFS=$'\t' read -r source_package binary_package runtime_test <<<"$entry"
    log "$source_package ($binary_package)"
    "$runtime_test"
  done

  log "all dependent runtime tests passed"
}

if [[ $# -gt 2 ]]; then
  usage >&2
  exit 2
fi

command=${1:-all}
apps_csv=${2:-}

case "$command" in
  compile)
    [[ -z $apps_csv ]] || {
      echo "compile mode does not accept an app filter" >&2
      exit 2
    }
    run_compile_suite
    ;;
  runtime)
    run_runtime_suite "$apps_csv"
    ;;
  all)
    run_compile_suite
    run_runtime_suite "$apps_csv"
    ;;
  --help|-h)
    usage
    ;;
  *)
    echo "unknown subcommand: $command" >&2
    usage >&2
    exit 2
    ;;
esac
