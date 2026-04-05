#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="tellico"

build() {
  local tellico_bin=""
  local csvtest_bin=""
  local support_root=""

  downstream::set_app_context "$APP_ID"
  downstream::assert_packaged_layout
  downstream::require_dir "$DOWNSTREAM_SOURCE_DIR"

  downstream::log "building tellico"
  rm -rf "$DOWNSTREAM_BUILD_DIR" "$DOWNSTREAM_INSTALL_ROOT"
  mkdir -p "$DOWNSTREAM_BUILD_DIR" "$DOWNSTREAM_INSTALL_ROOT" "$DOWNSTREAM_LOG_DIR"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/configure.log" \
    cmake \
      -S "$DOWNSTREAM_SOURCE_DIR" \
      -B "$DOWNSTREAM_BUILD_DIR" \
      -GNinja \
      -DBUILD_TESTING=ON \
      -DBUILD_FETCHER_TESTS=OFF \
      -DUSE_KHTML=ON \
      -DCMAKE_BUILD_TYPE=RelWithDebInfo \
      -DCMAKE_INSTALL_PREFIX=/usr

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/build.log" \
    cmake --build "$DOWNSTREAM_BUILD_DIR" --parallel --target tellico csvtest

  tellico_bin="$(downstream::find_built_file "$DOWNSTREAM_BUILD_DIR" '*/src/tellico')"
  csvtest_bin="$(downstream::find_built_file "$DOWNSTREAM_BUILD_DIR" '*/src/tests/csvtest')"

  downstream::assert_links_to_packaged_libcsv "$tellico_bin" tellico
  downstream::assert_links_to_packaged_libcsv "$csvtest_bin" tellico-csvtest

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/install.log" \
    bash -lc "install -Dm0755 '$tellico_bin' '$DOWNSTREAM_INSTALL_ROOT/usr/bin/tellico' && install -Dm0755 '$csvtest_bin' '$DOWNSTREAM_INSTALL_ROOT/usr/bin/csvtest'"

  support_root="$DOWNSTREAM_INSTALL_ROOT/opt/downstream-support"
  mkdir -p "$support_root"
  cp -a "$DOWNSTREAM_BUILD_DIR" "$support_root/build"
  cp -a "$DOWNSTREAM_SOURCE_DIR" "$support_root/source"
}

probe() {
  local support_build="/work/target/downstream/build/tellico"
  local probe_build="/tmp/downstream-tellico"
  local csvtest_bin=""

  downstream::set_app_context "$APP_ID"
  downstream::assert_packaged_layout

  mkdir -p "$DOWNSTREAM_LOG_DIR"
  [[ -d "$support_build" ]] || downstream::die "missing tellico support build tree in image"

  csvtest_bin="$(downstream::find_built_file "$support_build" '*/src/tests/csvtest')"
  downstream::assert_links_to_packaged_libcsv "$csvtest_bin" tellico-csvtest

  rm -rf "$probe_build"
  mkdir -p "$probe_build"
  cp -a "$support_build/." "$probe_build/"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/csvtest.log" \
    bash -lc "cd '$probe_build' && QT_QPA_PLATFORM=offscreen xvfb-run -a ctest -R '^csvtest$' --output-on-failure"

  test -s "$DOWNSTREAM_LOG_DIR/csvtest.log" || downstream::die "tellico csvtest log is empty"
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
