#!/usr/bin/env bash
# This port ships prebuilt .deb artifacts at the repository root rather
# than rebuilding from source on each commit. Copy them into dist/ so the
# rest of the hook chain (validation, upload, release) sees them.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_root/dist"

rm -rf -- "$dist_dir"
mkdir -p -- "$dist_dir"

shopt -s nullglob
debs=("$repo_root"/*.deb)
shopt -u nullglob
if (( ${#debs[@]} == 0 )); then
  printf 'build-debs: no *.deb files at repository root\n' >&2
  exit 1
fi

cp -v "${debs[@]}" "$dist_dir"/
