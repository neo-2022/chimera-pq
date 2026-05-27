#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

json="${1:-docs/CEF_TRACK_REPORT.json}"
track_md="${2:-docs/CEF_TRACK_REPORT.md}"
gap_md="${3:-docs/CEF_GAP_MAP_2026-05-18.md}"

cargo run -q -p chimera-lab --bin cef_consistency_guard -- "$json" "$track_md" "$gap_md"
