#!/usr/bin/env bash
set -euo pipefail

report_json="${1:-docs/SHIP_READINESS_REPORT.json}"
report_md="${2:-docs/SHIP_READINESS_REPORT.md}"

cargo run -q -p chimera-lab --bin ship_structure_guard -- \
  "$report_json" "$report_md"
