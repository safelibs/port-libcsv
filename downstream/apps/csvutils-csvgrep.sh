#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="csvutils-csvgrep"
BINARY="csvgrep"

build() {
  downstream::set_app_context "$APP_ID"
  downstream::build_csvutils_binary "$BINARY"
}

probe() {
  local app_root=""

  downstream::set_app_context "$APP_ID"
  app_root="$(downstream::runtime_app_root)"
  mkdir -p "$DOWNSTREAM_LOG_DIR"
  downstream::stage_shared_fixture_csv "$DOWNSTREAM_LOG_DIR/fixture.csv"

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/csvgrep.out" \
    bash -lc "cd '$DOWNSTREAM_LOG_DIR' && '$app_root/usr/bin/$BINARY' -f role -c '^user$' fixture.csv"

  grep -qx '2' "$DOWNSTREAM_LOG_DIR/csvgrep.out" \
    || downstream::die "csvgrep did not report the expected match count"
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
