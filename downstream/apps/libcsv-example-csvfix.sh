#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="libcsv-example-csvfix"
BINARY="csvfix"

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

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/csvfix.out" \
    "$app_root/usr/bin/$BINARY" \
    "$DOWNSTREAM_LOG_DIR/fixture.csv" \
    "$DOWNSTREAM_LOG_DIR/fixed.csv"

  python3 - "$DOWNSTREAM_LOG_DIR/fixed.csv" <<'PY'
from pathlib import Path
import sys

expected = "\"name\",\"role\",\"score\",\"notes\"\n\"Alice, A.\",\"admin\",\"42\",\"likes;semicolons\"\n\"Bob\",\"user\",\"7\",\"plain text\"\n\"Cara\",\"user\",\"99\",\"multi word\"\n"
actual = Path(sys.argv[1]).read_text(encoding="utf-8")
assert actual == expected, actual
PY
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
