#!/usr/bin/env bash
set -euo pipefail

report_json="${1:-docs/SHIP_READINESS_REPORT.json}"
report_md="${2:-docs/SHIP_READINESS_REPORT.md}"
reality_json="${3:-docs/REALITY_AUDIT_LATEST.json}"

cargo run -q -p chimera-lab --bin ship_readiness_json_guard -- \
  "$report_json" "$report_md" "$reality_json"
