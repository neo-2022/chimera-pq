#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

json="${1:-docs/CEF_TRACK_REPORT.json}"
md="${2:-docs/CEF_TRACK_REPORT.md}"

cargo run -q -p chimera-lab --bin cef_track_guard -- "$json" "$md"
