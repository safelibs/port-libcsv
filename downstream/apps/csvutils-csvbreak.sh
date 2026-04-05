#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../common.sh
source "$SCRIPT_DIR/../common.sh"

APP_ID="csvutils-csvbreak"
BINARY="csvbreak"

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

  downstream::run_logged "$DOWNSTREAM_LOG_DIR/csvbreak.out" \
    bash -lc "cd '$DOWNSTREAM_LOG_DIR' && '$app_root/usr/bin/$BINARY' -c -f role fixture.csv"

  python3 - "$DOWNSTREAM_LOG_DIR/csvbreak.out" <<'PY'
from pathlib import Path
import sys

lines = sorted(Path(sys.argv[1]).read_text(encoding="utf-8").splitlines())
assert lines == ["admin: 1", "user: 2"], lines
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
