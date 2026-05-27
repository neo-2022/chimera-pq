#!/usr/bin/env bash
set -euo pipefail

report_json="${1:-docs/SHIP_READINESS_REPORT.json}"
max_age_sec="${2:-1800}"

cargo run -q -p chimera-lab --bin ship_readiness_freshness_guard -- \
  "$report_json" "$max_age_sec"
