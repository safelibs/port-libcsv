#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="libcsv-example-csvvalid"
BINARY="csvvalid"

build() {
  downstream::set_app_context "$APP_ID"
  downstream::build_libcsv_example_binary "$BINARY"
}

probe() {
  local app_root=""

  downstream::set_app_context "$APP_ID"
  app_root="$(downstream::runtime_app_root)"
  mkdir -p "$DOWNSTREAM_LOG_DIR"
  downstream::stage_bad_malformed_csv "$DOWNSTREAM_LOG_DIR/bad.csv"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/csvvalid.out" \
    "$app_root/usr/bin/$BINARY" \
    "$DOWNSTREAM_LOG_DIR/bad.csv"

  grep -q 'malformed at byte 23' "$DOWNSTREAM_LOG_DIR/csvvalid.out" \
    || downstream::die "csvvalid did not report the expected malformed-byte offset"
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
