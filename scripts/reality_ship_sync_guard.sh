#!/usr/bin/env bash
set -euo pipefail

reality_json="${1:-docs/REALITY_AUDIT_LATEST.json}"
ship_json="${2:-docs/SHIP_READINESS_REPORT.json}"
probe_json="${3:-docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json}"

cargo run -q -p chimera-lab --bin reality_ship_sync_guard -- \
  "$reality_json" "$ship_json" "$probe_json"
