#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="readstat"

build() {
  local readstat_bin=""
  local extract_metadata_bin=""
  local install_lib_path=""

  downstream::set_app_context "$APP_ID"
  downstream::assert_packaged_layout
  downstream::require_dir "$DOWNSTREAM_SOURCE_DIR"

  downstream::log "building readstat"
  rm -rf "$DOWNSTREAM_BUILD_DIR" "$DOWNSTREAM_INSTALL_ROOT"
  mkdir -p "$DOWNSTREAM_BUILD_DIR" "$DOWNSTREAM_INSTALL_ROOT" "$DOWNSTREAM_LOG_DIR"
  cp -a "$DOWNSTREAM_SOURCE_DIR/." "$DOWNSTREAM_BUILD_DIR/"

  if [[ ! -x "$DOWNSTREAM_BUILD_DIR/configure" ]]; then
    downstream::run_logged "$DOWNSTREAM_LOG_DIR/autoreconf.log" \
      bash -lc "cd '$DOWNSTREAM_BUILD_DIR' && autoreconf -fi"
  fi

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/configure.log" \
    bash -lc "cd '$DOWNSTREAM_BUILD_DIR' && ./configure --prefix=/usr"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/build.log" \
    bash -lc "cd '$DOWNSTREAM_BUILD_DIR' && make -j'$(downstream::nproc)'"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/install.log" \
    bash -lc "cd '$DOWNSTREAM_BUILD_DIR' && make install DESTDIR='$DOWNSTREAM_INSTALL_ROOT'"

  readstat_bin="$DOWNSTREAM_INSTALL_ROOT/usr/bin/readstat"
  extract_metadata_bin="$DOWNSTREAM_INSTALL_ROOT/usr/bin/extract_metadata"
  install_lib_path="$(downstream::install_root_library_path "$DOWNSTREAM_INSTALL_ROOT")"

  [[ -x "$readstat_bin" ]] || downstream::die "readstat was not installed into $DOWNSTREAM_INSTALL_ROOT"
  [[ -x "$extract_metadata_bin" ]] || downstream::die "extract_metadata was not installed into $DOWNSTREAM_INSTALL_ROOT"

  downstream::assert_links_to_packaged_libcsv "$readstat_bin" readstat "$install_lib_path"
}

probe() {
  local app_root=""
  local readstat_bin=""
  local extract_metadata_bin=""
  local install_lib_path=""

  downstream::set_app_context "$APP_ID"
  downstream::assert_packaged_layout

  app_root="$(downstream::runtime_app_root)"
  readstat_bin="$app_root/usr/bin/readstat"
  extract_metadata_bin="$app_root/usr/bin/extract_metadata"
  install_lib_path="$(downstream::install_root_library_path "$app_root")"

  downstream::assert_links_to_packaged_libcsv "$readstat_bin" readstat "$install_lib_path"
  downstream::run_readstat_probe "$readstat_bin" "$extract_metadata_bin" "$install_lib_path" "$DOWNSTREAM_LOG_DIR"
}

case "${1:-}" in
  build)
    build
    ;;
  probe)
    probe
    ;;
  *)
    downstream::die "usage: ${0##*/} {build|probe}"
    ;;
esac
