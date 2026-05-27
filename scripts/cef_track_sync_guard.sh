#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

json="${1:-docs/CEF_TRACK_REPORT.json}"
md="${2:-docs/CEF_TRACK_REPORT.md}"

if [[ ! -f "$json" ]]; then
  echo "cef track sync guard: missing artifact: $json" >&2
  exit 1
fi
if [[ ! -f "$md" ]]; then
  echo "cef track sync guard: missing artifact: $md" >&2
  exit 1
fi

workdir="$(mktemp -d)"
cleanup() {
  rm -rf "$workdir"
}
trap cleanup EXIT

tmp_json="$workdir/CEF_TRACK_REPORT.json"
tmp_md="$workdir/CEF_TRACK_REPORT.md"

bash scripts/cef_track_report.sh "$tmp_json" "$tmp_md" >/dev/null

cmp -s "$json" "$tmp_json"
cmp -s "$md" "$tmp_md"

echo "cef track sync guard: PASS"
