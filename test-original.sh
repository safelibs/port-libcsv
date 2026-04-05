#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="${LIBCSV_ORIGINAL_TEST_IMAGE:-libcsv-original-test:ubuntu24.04}"
ONLY=""

usage() {
  cat <<'EOF'
usage: test-original.sh [--only <source-package>]

Runs a Docker-based Ubuntu 24.04 compatibility matrix for the libcsv
dependents recorded in dependents.json, building each dependent from source
against locally built libcsv Debian packages and then exercising its
documented runtime CSV functionality.

--only limits execution to one source package from dependents.json.
EOF
}

while (($#)); do
  case "$1" in
    --only)
      ONLY="${2:?missing value for --only}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      printf 'unknown option: %s\n' "$1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

command -v docker >/dev/null 2>&1 || {
  echo "docker is required to run $0" >&2
  exit 1
}

[[ -d "$ROOT/original" ]] || {
  echo "missing original source tree" >&2
  exit 1
}

[[ -d "$ROOT/safe" ]] || {
  echo "missing safe source tree" >&2
  exit 1
}

[[ -f "$ROOT/dependents.json" ]] || {
  echo "missing dependents.json" >&2
  exit 1
}

docker build -t "$IMAGE_TAG" - <<'DOCKERFILE'
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH=/root/.cargo/bin:$PATH

RUN sed -i 's/^Types: deb$/Types: deb deb-src/' /etc/apt/sources.list.d/ubuntu.sources \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      autoconf \
      automake \
      build-essential \
      ca-certificates \
      cargo \
      cmake \
      curl \
      dbus-x11 \
      debhelper \
      dpkg-dev \
      file \
      git \
      libtool \
      ninja-build \
      pkg-config \
      python3 \
      rustc \
      xauth \
      xvfb \
 && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
 && rm -rf /var/lib/apt/lists/*
DOCKERFILE

docker run \
  --rm \
  -i \
  -e "LIBCSV_TEST_ONLY=$ONLY" \
  -v "$ROOT":/work:ro \
  "$IMAGE_TAG" \
  bash -s <<'CONTAINER_SCRIPT'
set -Eeuo pipefail

export LANG=C.UTF-8
export LC_ALL=C.UTF-8

READ_ONLY_ROOT=/work
ROOT=/tmp/libcsv-work
ONLY="${LIBCSV_TEST_ONLY:-}"
APT_UPDATED=0
CURRENT_STEP=""
LIBCSV_VERSION="3.0.3+dfsg-6+safelibs1"
DEB_MULTIARCH="$(dpkg-architecture -qDEB_HOST_MULTIARCH)"
PACKAGED_RUNTIME_SO="/usr/lib/${DEB_MULTIARCH}/libcsv.so.3.0.2"
LOCAL_LIBCSV3_DEB=""
LOCAL_LIBCSV_DEV_DEB=""
declare -A BUILD_DEPS_READY=()
declare -A SOURCE_DIRS=()

log() {
  printf '\n==> %s\n' "$1"
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

trap 'rc=$?; if [[ "$rc" -ne 0 && -n "$CURRENT_STEP" ]]; then printf "failed during: %s\n" "$CURRENT_STEP" >&2; fi; exit "$rc"' EXIT

run_logged() {
  local log_file="$1"
  shift

  if ! "$@" >"$log_file" 2>&1; then
    cat "$log_file" >&2
    return 1
  fi
}

assert_dependents_inventory() {
  python3 - "$ROOT/dependents.json" <<'PY'
import json
import sys
from pathlib import Path

expected = ["readstat", "tellico"]
data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
actual = [entry["source_package"] for entry in data["dependents"]]

if actual != expected:
    raise SystemExit(
        f"unexpected dependents.json source package list: expected {expected}, found {actual}"
    )
PY
}

assert_only_filter() {
  if [[ -z "$ONLY" ]]; then
    return 0
  fi

  python3 - "$ONLY" "$ROOT/dependents.json" <<'PY'
import json
import sys
from pathlib import Path

name = sys.argv[1]
data = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
known = {entry["source_package"] for entry in data["dependents"]}

if name not in known:
    raise SystemExit(f"unknown --only source package: {name}")
PY
}

should_run() {
  local pkg="$1"
  [[ -z "$ONLY" || "$ONLY" == "$pkg" ]]
}

apt_refresh() {
  if [[ "$APT_UPDATED" -eq 0 ]]; then
    CURRENT_STEP="apt-get update"
    apt-get update >/dev/null
    APT_UPDATED=1
  fi
}

install_build_deps() {
  local pkg="$1"

  if [[ -n "${BUILD_DEPS_READY[$pkg]:-}" ]]; then
    return 0
  fi

  log "$pkg: installing build dependencies"
  apt_refresh
  CURRENT_STEP="$pkg build-dependencies"
  apt-get build-dep -y "$pkg" >/tmp/"$pkg"-build-deps.log 2>&1 || {
    cat /tmp/"$pkg"-build-deps.log >&2
    return 1
  }
  BUILD_DEPS_READY[$pkg]=1
}

find_built_file() {
  local root="$1"
  local pattern="$2"
  local match=""

  match="$(find "$root" -type f -path "$pattern" | LC_ALL=C sort | head -n1 || true)"
  [[ -n "$match" ]] || die "unable to locate built file matching $pattern under $root"
  printf '%s\n' "$match"
}

fetch_source() {
  local pkg="$1"
  local src_root="/tmp/dependent-sources/$pkg"
  local source_dir=""

  if [[ -n "${SOURCE_DIRS[$pkg]:-}" ]]; then
    printf '%s\n' "${SOURCE_DIRS[$pkg]}"
    return 0
  fi

  log "$pkg: fetching source package" >&2
  apt_refresh
  rm -rf "$src_root"
  mkdir -p "$src_root"
  CURRENT_STEP="$pkg source download"
  (
    cd "$src_root"
    apt-get source "$pkg" >/tmp/"$pkg"-source.log 2>&1
  ) || {
    cat /tmp/"$pkg"-source.log >&2
    return 1
  }

  source_dir="$(find "$src_root" -mindepth 1 -maxdepth 1 -type d | head -n1)"
  [[ -n "$source_dir" ]] || die "failed to unpack source package for $pkg"

  SOURCE_DIRS[$pkg]="$source_dir"
  printf '%s\n' "$source_dir"
}

prepare_writable_root() {
  log "Copying repository into a writable workspace"
  rm -rf "$ROOT"
  mkdir -p "$ROOT"
  cp -a "$READ_ONLY_ROOT/." "$ROOT/"
}

build_local_libcsv_packages() {
  local build_dir="$ROOT/safe"

  [[ -d "$build_dir" ]] || die "missing safe source tree in writable workspace"

  log "Building local libcsv Debian packages"
  find "$ROOT" -maxdepth 1 -type f \
    \( -name 'libcsv3_*.deb' -o -name 'libcsv-dev_*.deb' -o -name 'libcsv_*.buildinfo' -o -name 'libcsv_*.changes' \) \
    -delete

  CURRENT_STEP="libcsv package build"
  run_logged /tmp/libcsv-package-build.log \
    bash -lc "cd '$build_dir' && dpkg-buildpackage -us -uc -b"

  LOCAL_LIBCSV3_DEB="$(find "$ROOT" -maxdepth 1 -type f -name "libcsv3_${LIBCSV_VERSION}_*.deb" | LC_ALL=C sort | head -n1 || true)"
  LOCAL_LIBCSV_DEV_DEB="$(find "$ROOT" -maxdepth 1 -type f -name "libcsv-dev_${LIBCSV_VERSION}_*.deb" | LC_ALL=C sort | head -n1 || true)"

  [[ -n "$LOCAL_LIBCSV3_DEB" ]] || die "failed to locate built libcsv3 package"
  [[ -n "$LOCAL_LIBCSV_DEV_DEB" ]] || die "failed to locate built libcsv-dev package"
}

install_local_libcsv_packages() {
  local runtime_version=""
  local dev_version=""
  local runtime_link=""
  local dev_link=""

  [[ -n "$LOCAL_LIBCSV3_DEB" ]] || die "missing locally built libcsv3 package path"
  [[ -n "$LOCAL_LIBCSV_DEV_DEB" ]] || die "missing locally built libcsv-dev package path"

  log "Installing locally built libcsv packages"
  apt_refresh

  CURRENT_STEP="purging archive libcsv packages"
  apt-get purge -y libcsv-dev libcsv3 >/tmp/libcsv-purge.log 2>&1 || {
    cat /tmp/libcsv-purge.log >&2
    return 1
  }

  CURRENT_STEP="installing local libcsv packages"
  apt-get install -y "$LOCAL_LIBCSV3_DEB" "$LOCAL_LIBCSV_DEV_DEB" >/tmp/libcsv-install.log 2>&1 || {
    cat /tmp/libcsv-install.log >&2
    return 1
  }
  ldconfig

  runtime_version="$(dpkg-query -W -f='${Version}' libcsv3)"
  dev_version="$(dpkg-query -W -f='${Version}' libcsv-dev)"
  [[ "$runtime_version" == "$LIBCSV_VERSION" ]] || die "unexpected libcsv3 version: $runtime_version"
  [[ "$dev_version" == "$LIBCSV_VERSION" ]] || die "unexpected libcsv-dev version: $dev_version"

  [[ -f /usr/include/csv.h ]] || die "packaged libcsv header was not installed"
  [[ -f "$PACKAGED_RUNTIME_SO" ]] || die "packaged runtime library was not installed"

  runtime_link="$(readlink -f "/usr/lib/${DEB_MULTIARCH}/libcsv.so.3")"
  dev_link="$(readlink -f "/usr/lib/${DEB_MULTIARCH}/libcsv.so")"
  [[ "$runtime_link" == "$PACKAGED_RUNTIME_SO" ]] || die "runtime symlink does not resolve to $PACKAGED_RUNTIME_SO"
  [[ "$dev_link" == "$PACKAGED_RUNTIME_SO" ]] || die "development symlink does not resolve to $PACKAGED_RUNTIME_SO"
}

build_ld_library_path() {
  local extra_path="${1:-}"

  if [[ -n "$extra_path" && -n "${LD_LIBRARY_PATH:-}" ]]; then
    printf '%s:%s\n' "$extra_path" "$LD_LIBRARY_PATH"
  elif [[ -n "$extra_path" ]]; then
    printf '%s\n' "$extra_path"
  elif [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
    printf '%s\n' "$LD_LIBRARY_PATH"
  else
    printf '\n'
  fi
}

install_root_library_path() {
  local install_root="$1"
  local candidate=""
  local path=""

  for candidate in \
    "$install_root/usr/lib/$DEB_MULTIARCH" \
    "$install_root/usr/lib"
  do
    if [[ -d "$candidate" ]]; then
      if [[ -n "$path" ]]; then
        path+=":$candidate"
      else
        path="$candidate"
      fi
    fi
  done

  printf '%s\n' "$path"
}

assert_links_to_packaged_libcsv() {
  local target="$1"
  local label="$2"
  local extra_ld_library_path="${3:-}"
  local runtime_path=""
  local resolved=""
  local ld_library_path=""

  [[ -e "$target" ]] || die "missing binary to inspect: $target"

  ld_library_path="$(build_ld_library_path "$extra_ld_library_path")"
  runtime_path="$(env LD_LIBRARY_PATH="$ld_library_path" ldd "$target" 2>/dev/null | awk '$1 == "libcsv.so.3" || $1 == "libcsv.so" { print $3; exit }')"
  [[ -n "$runtime_path" ]] || {
    echo "$label does not resolve libcsv at runtime" >&2
    env LD_LIBRARY_PATH="$ld_library_path" ldd "$target" >&2 || true
    return 1
  }

  resolved="$(readlink -f "$runtime_path")"
  if [[ "$resolved" != "$PACKAGED_RUNTIME_SO" ]]; then
    printf '%s resolved libcsv to %s instead of %s\n' "$label" "$resolved" "$PACKAGED_RUNTIME_SO" >&2
    env LD_LIBRARY_PATH="$ld_library_path" ldd "$target" >&2 || true
    return 1
  fi
}

run_readstat_smoke() {
  local readstat_bin="$1"
  local extract_metadata_bin="$2"
  local extra_ld_library_path="$3"
  local smoke_dir="/tmp/readstat-smoke"
  local ld_library_path=""

  log "readstat: exercising CSV-plus-metadata runtime conversion"
  rm -rf "$smoke_dir"
  mkdir -p "$smoke_dir"

  cat >"$smoke_dir/input.csv" <<'EOF'
name;score;notes
"Alice, A.";42;"likes;semicolons"
Bob;7;"plain text"
EOF

  cat >"$smoke_dir/metadata.json" <<'EOF'
{
  "type": "SPSS",
  "separator": ";",
  "variables": [
    {
      "name": "name",
      "type": "STRING",
      "label": "Name"
    },
    {
      "name": "score",
      "type": "NUMERIC",
      "label": "Score",
      "format": "NUMBER",
      "decimals": 0
    },
    {
      "name": "notes",
      "type": "STRING",
      "label": "Notes"
    }
  ]
}
EOF

  ld_library_path="$(build_ld_library_path "$extra_ld_library_path")"

  CURRENT_STEP="readstat runtime conversion"
  run_logged /tmp/readstat-convert.log \
    env LD_LIBRARY_PATH="$ld_library_path" \
      "$readstat_bin" \
      "$smoke_dir/input.csv" \
      "$smoke_dir/metadata.json" \
      "$smoke_dir/output.dta"

  grep -E 'Converted 3 variables and 2 rows' /tmp/readstat-convert.log >/dev/null \
    || die "readstat did not report the expected CSV conversion summary"

  CURRENT_STEP="readstat metadata extraction"
  run_logged /tmp/readstat-extract.log \
    env LD_LIBRARY_PATH="$ld_library_path" \
      "$extract_metadata_bin" \
      "$smoke_dir/output.dta" \
      "$smoke_dir/extracted.json"

  CURRENT_STEP="readstat round-trip back to csv"
  run_logged /tmp/readstat-roundtrip.log \
    env LD_LIBRARY_PATH="$ld_library_path" \
      "$readstat_bin" \
      "$smoke_dir/output.dta" \
      "$smoke_dir/roundtrip.csv"

  python3 - "$smoke_dir/extracted.json" "$smoke_dir/roundtrip.csv" <<'PY'
import csv
import json
import sys
from pathlib import Path

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
variables = metadata["variables"]
assert [v["name"] for v in variables] == ["name", "score", "notes"], variables
assert variables[0]["type"] == "STRING", variables[0]
assert variables[1]["type"] == "NUMERIC", variables[1]
assert variables[2]["type"] == "STRING", variables[2]

with Path(sys.argv[2]).open(newline="", encoding="utf-8") as f:
    rows = list(csv.reader(f))

assert rows == [
    ["name", "score", "notes"],
    ["Alice, A.", "42.000000", "likes;semicolons"],
    ["Bob", "7.000000", "plain text"],
], rows
PY
}

build_and_test_readstat() {
  local src_dir=""
  local build_dir="/tmp/build-readstat"
  local install_root="/tmp/readstat-install"
  local install_lib_path=""
  local readstat_bin=""
  local extract_metadata_bin=""

  install_build_deps readstat
  install_local_libcsv_packages
  src_dir="$(fetch_source readstat)"

  log "readstat: building against local libcsv packages"
  rm -rf "$build_dir" "$install_root"
  cp -a "$src_dir/." "$build_dir/"

  if [[ ! -x "$build_dir/configure" ]]; then
    CURRENT_STEP="readstat autoreconf"
    run_logged /tmp/readstat-autoreconf.log \
      bash -lc "cd '$build_dir' && autoreconf -fi"
  fi

  CURRENT_STEP="readstat configure"
  run_logged /tmp/readstat-configure.log \
    bash -lc "cd '$build_dir' && ./configure --prefix=/usr"

  CURRENT_STEP="readstat build"
  run_logged /tmp/readstat-build.log \
    bash -lc "cd '$build_dir' && make -j'$(nproc)'"

  CURRENT_STEP="readstat install"
  run_logged /tmp/readstat-install.log \
    bash -lc "cd '$build_dir' && make install DESTDIR='$install_root'"

  readstat_bin="$install_root/usr/bin/readstat"
  extract_metadata_bin="$install_root/usr/bin/extract_metadata"
  install_lib_path="$(install_root_library_path "$install_root")"

  [[ -x "$readstat_bin" ]] || die "readstat was not installed into $install_root"
  [[ -x "$extract_metadata_bin" ]] || die "extract_metadata was not installed into $install_root"
  assert_links_to_packaged_libcsv "$readstat_bin" readstat "$install_lib_path"
  run_readstat_smoke "$readstat_bin" "$extract_metadata_bin" "$install_lib_path"
}

build_and_test_tellico() {
  local src_dir=""
  local build_dir="/tmp/build-tellico"
  local tellico_bin=""
  local csvtest_bin=""

  install_build_deps tellico
  install_local_libcsv_packages
  src_dir="$(fetch_source tellico)"

  log "tellico: configuring against local libcsv packages"
  rm -rf "$build_dir"
  CURRENT_STEP="tellico configure"
  run_logged /tmp/tellico-configure.log \
    cmake \
      -S "$src_dir" \
      -B "$build_dir" \
      -GNinja \
      -DBUILD_TESTING=ON \
      -DBUILD_FETCHER_TESTS=OFF \
      -DUSE_KHTML=ON \
      -DCMAKE_BUILD_TYPE=RelWithDebInfo

  CURRENT_STEP="tellico build"
  run_logged /tmp/tellico-build.log \
    cmake --build "$build_dir" --parallel --target tellico csvtest

  tellico_bin="$(find_built_file "$build_dir" '*/src/tellico')"
  csvtest_bin="$(find_built_file "$build_dir" '*/src/tests/csvtest')"

  assert_links_to_packaged_libcsv "$tellico_bin" tellico
  assert_links_to_packaged_libcsv "$csvtest_bin" tellico-csvtest

  log "tellico: running upstream csvtest"
  CURRENT_STEP="tellico csvtest"
  run_logged /tmp/tellico-csvtest.log \
    bash -lc "cd '$build_dir' && QT_QPA_PLATFORM=offscreen xvfb-run -a ctest -R '^csvtest$' --output-on-failure"
}

prepare_writable_root
assert_dependents_inventory
assert_only_filter
build_local_libcsv_packages

if should_run readstat; then
  build_and_test_readstat
fi

if should_run tellico; then
  build_and_test_tellico
fi

CURRENT_STEP=""
log "All requested downstream compatibility checks passed"
CONTAINER_SCRIPT
