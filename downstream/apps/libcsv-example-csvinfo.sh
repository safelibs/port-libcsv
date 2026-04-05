#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="libcsv-example-csvinfo"
BINARY="csvinfo"

build() {
  downstream::set_app_context "$APP_ID"
  downstream::build_libcsv_example_binary "$BINARY"
}

probe() {
  local app_root=""

  downstream::set_app_context "$APP_ID"
  app_root="$(downstream::runtime_app_root)"
  mkdir -p "$DOWNSTREAM_LOG_DIR"
  downstream::stage_shared_fixture_csv "$DOWNSTREAM_LOG_DIR/fixture.csv"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/csvinfo.out" \
    "$app_root/usr/bin/$BINARY" \
    "$DOWNSTREAM_LOG_DIR/fixture.csv"

  grep -qx "${DOWNSTREAM_LOG_DIR}/fixture.csv: 16 fields, 4 rows" "$DOWNSTREAM_LOG_DIR/csvinfo.out" \
    || downstream::die "csvinfo did not report the expected field and row totals"
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
