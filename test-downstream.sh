#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="${LIBCSV_DOWNSTREAM_IMAGE:-libcsv-downstream:dev}"
INVENTORY="$ROOT/downstream-apps.json"
IMAGE_MANIFEST="$ROOT/target/downstream/image-manifest.json"
ONLY=""
REPORT="$ROOT/target/downstream/smoke-results.json"
KEEP_GOING=0

usage() {
  cat <<'EOF'
usage: test-downstream.sh [--only <app-id>] [--report <path>] [--keep-going]

Runs the downstream matrix harness around scripts/downstream-matrix.py.

--only limits execution to one application id from downstream-apps.json.
--report writes the machine-readable run report to the given path.
--keep-going keeps running after per-application failures and returns non-zero
after writing the full report.
EOF
}

while (($#)); do
  case "$1" in
    --only)
      ONLY="${2:?missing value for --only}"
      shift 2
      ;;
    --report)
      REPORT="${2:?missing value for --report}"
      shift 2
      ;;
    --keep-going)
      KEEP_GOING=1
      shift
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

cmd=(
  python3
  "$ROOT/scripts/downstream-matrix.py"
  run
  --inventory
  "$INVENTORY"
  --image-tag
  "$IMAGE_TAG"
  --image-manifest
  "$IMAGE_MANIFEST"
  --report
  "$REPORT"
)

if [[ -n "$ONLY" ]]; then
  cmd+=(--only "$ONLY")
fi

if [[ "$KEEP_GOING" -eq 1 ]]; then
  cmd+=(--keep-going)
fi

exec "${cmd[@]}"
