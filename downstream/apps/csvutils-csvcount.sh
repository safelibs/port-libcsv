#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="csvutils-csvcount"
BINARY="csvcount"

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

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/csvcount.out" \
    bash -lc "cd '$DOWNSTREAM_LOG_DIR' && '$app_root/usr/bin/$BINARY' -r -f fixture.csv"

  python3 - "$DOWNSTREAM_LOG_DIR/csvcount.out" <<'PY'
from pathlib import Path
import sys

parts = Path(sys.argv[1]).read_text(encoding="utf-8").split()
assert parts == ["4", "16", "fixture.csv"], parts
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
