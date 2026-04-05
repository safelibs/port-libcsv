#!/usr/bin/env bash
set -euo pipefail

DOWNSTREAM_COMMON_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
: "${DOWNSTREAM_REPO_ROOT:=$(cd -- "$DOWNSTREAM_COMMON_DIR/.." && pwd)}"
: "${DOWNSTREAM_TARGET_ROOT:=$DOWNSTREAM_REPO_ROOT/target/downstream}"
: "${DOWNSTREAM_FIXTURE_ROOT:=$DOWNSTREAM_REPO_ROOT/downstream/fixtures}"
: "${DOWNSTREAM_IMAGE_ROOT_BASE:=/opt/downstream/apps}"
: "${DOWNSTREAM_HARNESS_ROOT:=/opt/downstream/harness}"

if [[ -n "${DOWNSTREAM_MULTIARCH:-}" ]]; then
  _downstream_multiarch="$DOWNSTREAM_MULTIARCH"
else
  _downstream_multiarch="$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || true)"
  if [[ -z "$_downstream_multiarch" ]]; then
    _downstream_multiarch="x86_64-linux-gnu"
  fi
fi
export DOWNSTREAM_MULTIARCH="$_downstream_multiarch"
export DOWNSTREAM_PACKAGED_RUNTIME_SO="${DOWNSTREAM_PACKAGED_RUNTIME_SO:-/usr/lib/${DOWNSTREAM_MULTIARCH}/libcsv.so.3.0.2}"

downstream::die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

downstream::log() {
  if [[ -n "${DOWNSTREAM_APP_ID:-}" ]]; then
    printf '==> [%s] %s\n' "$DOWNSTREAM_APP_ID" "$*"
  else
    printf '==> %s\n' "$*"
  fi
}

downstream::require_dir() {
  [[ -d "$1" ]] || downstream::die "missing directory: $1"
}

downstream::require_file() {
  [[ -f "$1" ]] || downstream::die "missing file: $1"
}

downstream::nproc() {
  nproc 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || printf '1\n'
}

downstream::set_app_context() {
  local app_id="$1"

  export DOWNSTREAM_APP_ID="$app_id"
  export DOWNSTREAM_SOURCE_DIR="${DOWNSTREAM_TARGET_ROOT}/sources/${app_id}"
  export DOWNSTREAM_BUILD_DIR="${DOWNSTREAM_TARGET_ROOT}/build/${app_id}"
  export DOWNSTREAM_INSTALL_ROOT="${DOWNSTREAM_TARGET_ROOT}/install/${app_id}"
  export DOWNSTREAM_LOG_DIR="${DOWNSTREAM_TARGET_ROOT}/logs/${app_id}"
  export DOWNSTREAM_IMAGE_APP_ROOT="${DOWNSTREAM_IMAGE_ROOT_BASE%/}/${app_id}"
}

downstream::runtime_app_root() {
  if [[ -d "$DOWNSTREAM_IMAGE_APP_ROOT" ]]; then
    printf '%s\n' "$DOWNSTREAM_IMAGE_APP_ROOT"
  else
    printf '%s\n' "$DOWNSTREAM_INSTALL_ROOT"
  fi
}

downstream::run_logged() {
  local log_file="$1"
  shift

  mkdir -p "$(dirname -- "$log_file")"
  if ! "$@" >"$log_file" 2>&1; then
    cat "$log_file" >&2
    return 1
  fi
}

downstream::build_ld_library_path() {
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

downstream::install_root_library_path() {
  local install_root="$1"
  local candidate=""
  local path=""

  for candidate in \
    "$install_root/usr/lib/$DOWNSTREAM_MULTIARCH" \
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

downstream::assert_packaged_layout() {
  local runtime_link=""
  local dev_link=""

  [[ -f /usr/include/csv.h ]] || downstream::die "packaged libcsv header was not installed"
  [[ -f "$DOWNSTREAM_PACKAGED_RUNTIME_SO" ]] || downstream::die "packaged runtime library was not installed"

  runtime_link="$(readlink -f "/usr/lib/${DOWNSTREAM_MULTIARCH}/libcsv.so.3")"
  dev_link="$(readlink -f "/usr/lib/${DOWNSTREAM_MULTIARCH}/libcsv.so")"
  [[ "$runtime_link" == "$DOWNSTREAM_PACKAGED_RUNTIME_SO" ]] || downstream::die "runtime symlink does not resolve to $DOWNSTREAM_PACKAGED_RUNTIME_SO"
  [[ "$dev_link" == "$DOWNSTREAM_PACKAGED_RUNTIME_SO" ]] || downstream::die "development symlink does not resolve to $DOWNSTREAM_PACKAGED_RUNTIME_SO"
}

downstream::assert_links_to_packaged_libcsv() {
  local target="$1"
  local label="$2"
  local extra_ld_library_path="${3:-}"
  local runtime_path=""
  local resolved=""
  local ld_library_path=""

  [[ -e "$target" ]] || downstream::die "missing binary to inspect: $target"

  ld_library_path="$(downstream::build_ld_library_path "$extra_ld_library_path")"
  runtime_path="$(env LD_LIBRARY_PATH="$ld_library_path" ldd "$target" 2>/dev/null | awk '$1 == "libcsv.so.3" || $1 == "libcsv.so" { print $3; exit }')"
  [[ -n "$runtime_path" ]] || {
    printf '%s does not resolve libcsv at runtime\n' "$label" >&2
    env LD_LIBRARY_PATH="$ld_library_path" ldd "$target" >&2 || true
    return 1
  }

  resolved="$(readlink -f "$runtime_path")"
  if [[ "$resolved" != "$DOWNSTREAM_PACKAGED_RUNTIME_SO" ]]; then
    printf '%s resolved libcsv to %s instead of %s\n' "$label" "$resolved" "$DOWNSTREAM_PACKAGED_RUNTIME_SO" >&2
    env LD_LIBRARY_PATH="$ld_library_path" ldd "$target" >&2 || true
    return 1
  fi
}

downstream::find_built_file() {
  local root="$1"
  local pattern="$2"
  local match=""

  match="$(find -L "$root" -type f -path "$pattern" | LC_ALL=C sort | head -n1 || true)"
  [[ -n "$match" ]] || downstream::die "unable to locate built file matching $pattern under $root"
  printf '%s\n' "$match"
}

downstream::copy_fixture() {
  local fixture_rel="$1"
  local destination="$2"

  downstream::require_file "$DOWNSTREAM_FIXTURE_ROOT/$fixture_rel"
  mkdir -p "$(dirname -- "$destination")"
  cp "$DOWNSTREAM_FIXTURE_ROOT/$fixture_rel" "$destination"
}

downstream::stage_shared_fixture_csv() {
  downstream::copy_fixture "shared/quoted-users.csv" "$1"
}

downstream::stage_bad_unterminated_csv() {
  downstream::copy_fixture "shared/bad-unterminated.csv" "$1"
}

downstream::stage_bad_malformed_csv() {
  downstream::copy_fixture "shared/bad-malformed.csv" "$1"
}

downstream::build_csvutils_binary() {
  local binary="$1"

  downstream::assert_packaged_layout
  downstream::require_dir "$DOWNSTREAM_SOURCE_DIR"

  downstream::log "building ${binary}"
  rm -rf "$DOWNSTREAM_BUILD_DIR" "$DOWNSTREAM_INSTALL_ROOT"
  mkdir -p \
    "$DOWNSTREAM_BUILD_DIR" \
    "$DOWNSTREAM_INSTALL_ROOT/usr/bin" \
    "$DOWNSTREAM_INSTALL_ROOT/usr/share/man/man1" \
    "$DOWNSTREAM_LOG_DIR"
  cp -a "$DOWNSTREAM_SOURCE_DIR/." "$DOWNSTREAM_BUILD_DIR/"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/build.log" \
    bash -lc "cd '$DOWNSTREAM_BUILD_DIR' && make CPPFLAGS='' CFLAGS='-Wall -ansi -pedantic' INCLUDES='-I include' '$binary'"

  install -Dm0755 "$DOWNSTREAM_BUILD_DIR/$binary" "$DOWNSTREAM_INSTALL_ROOT/usr/bin/$binary"
  if [[ -f "$DOWNSTREAM_BUILD_DIR/$binary.1.gz" ]]; then
    install -Dm0644 "$DOWNSTREAM_BUILD_DIR/$binary.1.gz" "$DOWNSTREAM_INSTALL_ROOT/usr/share/man/man1/$binary.1.gz"
  fi

  downstream::assert_links_to_packaged_libcsv "$DOWNSTREAM_INSTALL_ROOT/usr/bin/$binary" "$binary"
}

downstream::build_libcsv_example_binary() {
  local binary="$1"

  downstream::assert_packaged_layout
  downstream::require_dir "$DOWNSTREAM_SOURCE_DIR"

  downstream::log "building ${binary}"
  rm -rf "$DOWNSTREAM_BUILD_DIR" "$DOWNSTREAM_INSTALL_ROOT"
  mkdir -p "$DOWNSTREAM_BUILD_DIR" "$DOWNSTREAM_INSTALL_ROOT/usr/bin" "$DOWNSTREAM_LOG_DIR"
  cp -a "$DOWNSTREAM_SOURCE_DIR/." "$DOWNSTREAM_BUILD_DIR/"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/build.log" \
    bash -lc "cd '$DOWNSTREAM_BUILD_DIR/examples' && gcc -o '$binary' '$binary.c' -lcsv"

  install -Dm0755 "$DOWNSTREAM_BUILD_DIR/examples/$binary" "$DOWNSTREAM_INSTALL_ROOT/usr/bin/$binary"
  downstream::assert_links_to_packaged_libcsv "$DOWNSTREAM_INSTALL_ROOT/usr/bin/$binary" "$binary"
}

downstream::run_readstat_probe() {
  local readstat_bin="$1"
  local extract_metadata_bin="$2"
  local extra_ld_library_path="$3"
  local work_dir="$4"
  local ld_library_path=""

  mkdir -p "$work_dir"
  rm -f \
    "$work_dir/output.dta" \
    "$work_dir/extracted.json" \
    "$work_dir/roundtrip.csv"
  downstream::copy_fixture "readstat/input.csv" "$work_dir/input.csv"
  downstream::copy_fixture "readstat/metadata.json" "$work_dir/metadata.json"

  ld_library_path="$(downstream::build_ld_library_path "$extra_ld_library_path")"

  downstream::run_logged "$work_dir/convert.log" \
    env LD_LIBRARY_PATH="$ld_library_path" \
      "$readstat_bin" \
      "$work_dir/input.csv" \
      "$work_dir/metadata.json" \
      "$work_dir/output.dta"

  grep -E 'Converted 3 variables and 2 rows' "$work_dir/convert.log" >/dev/null \
    || downstream::die "readstat did not report the expected CSV conversion summary"

  downstream::run_logged "$work_dir/extract.log" \
    env LD_LIBRARY_PATH="$ld_library_path" \
      "$extract_metadata_bin" \
      "$work_dir/output.dta" \
      "$work_dir/extracted.json"

  downstream::run_logged "$work_dir/roundtrip.log" \
    env LD_LIBRARY_PATH="$ld_library_path" \
      "$readstat_bin" \
      "$work_dir/output.dta" \
      "$work_dir/roundtrip.csv"

  python3 - "$work_dir/extracted.json" "$work_dir/roundtrip.csv" <<'PY'
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

with Path(sys.argv[2]).open(newline="", encoding="utf-8") as handle:
    rows = list(csv.reader(handle))

assert rows == [
    ["name", "score", "notes"],
    ["Alice, A.", "42.000000", "likes;semicolons"],
    ["Bob", "7.000000", "plain text"],
], rows
PY
}
